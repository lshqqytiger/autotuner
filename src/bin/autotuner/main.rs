mod results;
mod runner;
mod signal;

use crate::runner::Runner;
use anyhow::anyhow;
use argh::{FromArgValue, FromArgs};
use autotuner::{
    manually_move::ManuallyMove,
    metadata::Metadata,
    parameter::{Code, Instance},
};
use fxhash::FxHashMap;
use libc::SIGQUIT;
use rand::seq::SliceRandom;
use rayon::{ThreadPoolBuilder, prelude::*};
use results::Results;
use serde::{Deserialize, Serialize};
use signal_hook_registry::{register, unregister};
use std::{fs, process, sync::Arc, time::SystemTime};

enum Direction {
    Minimize,
    Maximize,
}

impl FromArgValue for Direction {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "minimize" => Ok(Direction::Minimize),
            "maximize" => Ok(Direction::Maximize),
            _ => Err(format!("Invalid direction: {}", value)),
        }
    }
}

#[derive(FromArgs)]
/// CLI Arguments
struct Arguments {
    #[argh(positional)]
    sources: Vec<String>,

    #[argh(option, short = 'm')]
    /// metadata file (required)
    metadata: String,

    #[argh(option, short = 'd', default = "Direction::Maximize")]
    /// optimization direction (default: maximize)
    direction: Direction,

    #[argh(option, short = 'i', default = "32")]
    /// initial population size (default: 32)
    initial: usize,

    #[argh(option, short = 'n', default = "16")]
    /// number of instances that will be made at each generation (default: 16)
    ngeneration: usize,

    #[argh(option, short = 'r', default = "1")]
    /// number of repetitions for each instance (default: 1)
    repetition: usize,

    #[argh(option, short = 'l', default = "64")]
    /// maximum number of generations (default: 64)
    limit: usize,

    #[argh(option, short = 'p', default = "1")]
    /// number of instances that will be evaluated in parallel (default: 1)
    parallelism: usize,

    #[argh(option, default = "4096")]
    /// cache size in number of entries (default: 4096)
    cache_size: usize,

    #[argh(option, arg_name = "continue")]
    /// continue from the saved state file
    continue_: Option<String>,

    #[argh(option, default = "\"results.json\".to_string()")]
    /// output file (default: results.json)
    output: String,
}

#[derive(Serialize, Deserialize)]
struct SavedState {
    instances: Vec<FxHashMap<Arc<str>, Code>>,
    results: Vec<(FxHashMap<Arc<str>, Code>, f64)>,
}

impl SavedState {
    fn new(instances: &Vec<Arc<Instance>>, results: &Results) -> Self {
        let instances = instances
            .iter()
            .map(|x| x.parameters.clone())
            .collect::<Vec<_>>();
        let results = results
            .iter()
            .filter_map(|(instance, fitness)| {
                if fitness.is_infinite() {
                    None
                } else {
                    Some((instance.parameters.clone(), *fitness))
                }
            })
            .collect::<Vec<_>>();
        SavedState { instances, results }
    }
}

#[inline]
fn round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

fn stochastic_universal_sampling(roulette: &[(f64, usize)], n: usize) -> Vec<usize> {
    assert!(!roulette.is_empty());
    assert_ne!(n, 0);

    let total_fitness: f64 = roulette.iter().map(|(fitness, _)| fitness).sum();
    assert!(total_fitness > 0.0);

    let distance = total_fitness / n as f64;

    let start = rand::random::<f64>() * distance;

    let mut selected = Vec::with_capacity(n);
    let mut current_sum = 0.0;
    let mut current_index = 0;

    for i in 0..n {
        let pointer = start + i as f64 * distance;

        while current_sum < pointer && current_index < roulette.len() {
            current_sum += roulette[current_index].0;
            if current_sum >= pointer {
                selected.push(roulette[current_index].1);
                break;
            }
            current_index += 1;
        }

        if selected.len() <= i {
            selected.push(roulette[roulette.len() - 1].1);
        }
    }

    selected
}

fn main() -> anyhow::Result<()> {
    let args: Arguments = argh::from_env();
    if args.initial <= 1 {
        return Err(anyhow!("Initial population size must be greater than 1"));
    }
    if args.ngeneration == 0 {
        return Err(anyhow!("Number of each generation must be greater than 0"));
    }
    if args.parallelism == 0 {
        return Err(anyhow!(
            "Number of instances that will be evaluated in parallel must be greater than 0"
        ));
    }

    ThreadPoolBuilder::new()
        .num_threads(args.parallelism)
        .thread_name(|x| format!("t{}", x))
        .start_handler(|_| unsafe {
            signal::block(SIGQUIT);
        })
        .build_global()?;

    let cores = core_affinity::get_core_ids().unwrap();
    let num_cores = cores.len();

    let metadata = serde_json::from_str::<Metadata>(
        &fs::read_to_string(args.metadata).expect("Failed to read kernel metadata file"),
    )
    .expect("Failed to parse kernel metadata");

    let saved_state = args.continue_.map(|filename| {
        let content = fs::read_to_string(filename).expect("Failed to read saved state file");
        serde_json::from_str::<SavedState>(&content).expect("Failed to parse saved state file")
    });

    let mut instances = ManuallyMove::new(Vec::new());
    let mut results = ManuallyMove::new(Results::new(args.cache_size));
    if let Some(saved_state) = saved_state {
        for parameters in saved_state.instances {
            instances.push(Arc::new(Instance::new(
                metadata.profile.clone(),
                parameters,
            )));
        }
        for (parameters, fitness) in saved_state.results {
            let instance = Arc::new(Instance::new(metadata.profile.clone(), parameters));
            results.insert(instance, fitness);
        }
    }

    for _ in instances.len()..args.initial {
        let instance = metadata.profile.random();
        instances.push(Arc::new(instance));
    }

    let runner = ManuallyMove::new(Runner::new(args.sources, metadata, args.parallelism)?);

    let sigquit_handler = unsafe {
        // Thread Unsafe
        let instances = instances.clone();
        let results = results.clone();
        let runner = runner.clone();
        register(SIGQUIT, move || {
            // Stop the autotuner and save current state.
            // Move states and runner from main() into the closure.
            let instances = instances.mov();
            let results = results.mov();
            let runner = runner.mov();
            ManuallyMove::drop(runner);

            let filename = format!(
                "saved_state.{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            let saved_state = SavedState::new(&instances, &results);
            ManuallyMove::drop(instances);
            ManuallyMove::drop(results);
            fs::write(
                &filename,
                serde_json::to_string(&saved_state).expect("Failed to serialize instances"),
            )
            .expect("Failed to write current state to file");

            println!("Saved current state to {}", filename);
            process::exit(0);
        })
    };

    let mut evaluation_results: Vec<(f64, usize)> = Vec::with_capacity(args.initial);
    let mut rng = rand::rng();
    for i in 0..args.limit {
        if !evaluation_results.is_empty() {
            let min = evaluation_results
                .iter()
                .filter(|(x, _)| !x.is_infinite())
                .fold(f64::INFINITY, |a, &b| a.min(b.0));
            let max = evaluation_results
                .iter()
                .filter(|(x, _)| !x.is_infinite())
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b.0));
            println!("min = {}, max = {}", min, max);

            let mut inversed = evaluation_results.clone();
            for pair in &mut inversed {
                if pair.0.is_infinite() {
                    pair.0 = max;
                    continue;
                }

                pair.0 = match args.direction {
                    Direction::Minimize => pair.0,
                    Direction::Maximize => max - pair.0,
                };
            }
            inversed.shuffle(&mut rng);
            let holes = stochastic_universal_sampling(&inversed, args.ngeneration);
            drop(inversed);

            for result in &mut evaluation_results {
                if result.0.is_infinite() {
                    result.0 = min;
                    continue;
                }

                result.0 = match args.direction {
                    Direction::Minimize => max - result.0,
                    Direction::Maximize => result.0,
                };
            }
            evaluation_results.shuffle(&mut rng);

            let mut children = Vec::with_capacity(args.ngeneration);
            for _ in 0..args.ngeneration {
                let result = stochastic_universal_sampling(&evaluation_results, 2);
                let child = Instance::crossover(&instances[result[0]], &instances[result[1]]);
                let child = child.mutate();
                children.push(child);
            }

            for (index, instance) in children.into_iter().enumerate() {
                instances[holes[index]] = Arc::new(instance);
            }

            evaluation_results.clear();
        }

        println!("#{}", i + 1);

        let len = instances.len();
        let mut fresh_instances = Vec::new();
        for index in 0..len {
            if let Some(&fitness) = results.get(&instances[index]) {
                evaluation_results.push((fitness, index));
                continue;
            }
            fresh_instances.push((index, instances[index].clone()));
        }

        let len = fresh_instances.len();
        for i in 0..round_up(len, args.parallelism) {
            let fresh_instances =
                &fresh_instances[(i * args.parallelism)..((i + 1) * args.parallelism).min(len)];

            if fresh_instances.len() == 1 {
                println!("Running kernel {}", fresh_instances[0].1);
            } else {
                println!("Running kernels below: ");
                for (_, instance) in fresh_instances {
                    println!("- {}", instance);
                }
            }

            with_signal_mask!(SIGQUIT, {
                let chunk: Vec<anyhow::Result<(f64, usize)>> = fresh_instances
                    .par_iter()
                    .map(|(i, instance)| {
                        let tid = rayon::current_thread_index().unwrap_or(0);
                        let index = tid
                            + args.parallelism
                                * rand::random_range(0..(num_cores / args.parallelism));
                        core_affinity::set_for_current(cores[index]);

                        let instance = instance.clone();
                        let value = runner.evaluate(&instance, args.repetition)?;
                        Ok((value, *i))
                    })
                    .collect();
                // FIXME: somehow SIGQUIT is delivered to the main thread even though it is blocked

                for result in chunk {
                    let result = result?;
                    results.insert(instances[result.1].clone(), result.0);
                    evaluation_results.push(result);
                }
            });
        }
    }

    // The signal handler must be unregistered early enough.
    if let Ok(sigquit_handler) = sigquit_handler {
        unregister(sigquit_handler);
    }

    ManuallyMove::drop(runner);

    drop(evaluation_results);
    ManuallyMove::drop(instances);

    let mut instances = results
        .iter()
        .map(|(instance, fitness)| (format!("{}", instance), *fitness))
        .collect::<Vec<_>>();
    ManuallyMove::drop(results);
    instances.sort_by(|a, b| a.1.total_cmp(&b.1));

    fs::write(
        args.output,
        serde_json::to_string_pretty(&instances).expect("Failed to serialize instances"),
    )
    .expect("Failed to write results to file");

    Ok(())
}
