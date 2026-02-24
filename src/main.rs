mod compile;
mod configuration;
mod context;
mod criterion;
mod direction;
mod helper;
mod hook;
mod parameter;
mod runner;
mod strategies;
mod utils;
mod workspace;

use crate::{
    configuration::Configuration,
    context::Context,
    direction::{Direction, Sort},
    helper::Helper,
    hook::Hook,
    parameter::Individual,
    runner::Runner,
    strategies::{exhaustive::Exhaustive, options::Step, Checkpoint},
    utils::{manually_move::ManuallyMove, union::Union},
};
use anyhow::anyhow;
use argh::FromArgs;
use fxhash::FxHashMap;
use libc::{SIGQUIT, SIGSEGV};
use libloading::Library;
use rand::seq::SliceRandom;
use signal_hook_registry::{register, register_unchecked, unregister};
use std::{fs, hint, process, rc::Rc, time::SystemTime};
use tempdir::TempDir;

#[derive(FromArgs)]
/// CLI Arguments
struct Options {
    #[argh(positional)]
    configuration: String,

    #[argh(option, default = "Vec::new()")]
    /// path to source files
    sources: Vec<String>,

    #[argh(option, default = "Vec::new()")]
    /// path to helper files
    helper: Vec<String>,

    #[argh(option, default = "Vec::new()")]
    /// path to hook files
    hook: Vec<String>,

    #[argh(option, short = 'c', default = "Vec::new()")]
    /// CPU cores to use
    cores: Vec<usize>,

    #[argh(option, short = 'r', default = "15")]
    /// number of repetitions for each individual (default: 15)
    repetition: usize,

    #[argh(option, default = "32")]
    /// number of candidates (default: 32)
    candidates: usize,

    #[argh(option, arg_name = "continue")]
    /// path to checkpoint file
    continue_: Option<String>,

    #[argh(option, default = "\"result.json\".to_string()")]
    /// output file (default: result.json)
    output: String,

    #[argh(switch, short = 'v')]
    /// verbose output
    verbose: bool,
}

struct Autotuner<'a> {
    sources: &'a [String],
    configuration: Configuration,
    cores: Option<Vec<usize>>,
    temp_dir: TempDir,
    helper: Library,
    hook: Library,
    workspace: workspace::Workspace<'a>,
}

impl<'a> Drop for Autotuner<'a> {
    fn drop(&mut self) {
        unsafe {
            let helper = self
                .helper
                .get::<Helper>(self.configuration.helper.post.as_bytes())
                .unwrap();
            helper.call(&mut self.workspace);
        }
    }
}

impl<'a> Autotuner<'a> {
    fn new(
        sources: &'a [String],
        helper: &'a [String],
        hook: &'a [String],
        configuration: Configuration,
        cores: &Option<Vec<usize>>,
    ) -> anyhow::Result<Self> {
        match &configuration.strategy {
            strategies::Strategy::Exhaustive(_) => {}
            strategies::Strategy::Genetic(options) => {
                if options.initial <= 1 {
                    return Err(anyhow!("Initial population size must be greater than 1"));
                }
                if options.generate.value == 0 {
                    return Err(anyhow!("Number of each generation must be greater than 0"));
                }
            }
        }

        let cores = if let Some(cores) = cores {
            if affinity::get_thread_affinity().is_err() {
                eprintln!("[WARNING] Failed to get thread affinity");
                None
            } else {
                if cores.is_empty() {
                    return Err(anyhow!("At least one CPU core must be specified"));
                }
                Some(cores.to_vec())
            }
        } else {
            None
        };

        let temp_dir = TempDir::new("autotuner")?;
        fs::create_dir(temp_dir.path().join("individuals"))?;

        let path = temp_dir.path().join("libhelper.so");
        compile::compile(
            &configuration.compiler,
            &path,
            helper.iter().chain(configuration.compiler_arguments.iter()),
        )?;
        let helper = unsafe { Library::new(&path) }?;

        let path = temp_dir.path().join("libhook.so");
        compile::compile(
            &configuration.compiler,
            &path,
            hook.iter().chain(configuration.compiler_arguments.iter()),
        )?;
        let hook = unsafe { Library::new(&path) }?;

        let mut workspace = workspace::Workspace::new();

        unsafe {
            let initializer = helper
                .get::<Helper>(configuration.helper.pre.as_bytes())
                .unwrap();
            initializer.call(&mut workspace);
        }

        Ok(Autotuner {
            sources,
            configuration,
            temp_dir,
            helper,
            hook,
            workspace,
            cores,
        })
    }

    fn run(
        &'a self,
        repetition: usize,
        candidates: usize,
        checkpoint: Option<Checkpoint>,
        verbose: bool,
    ) -> Union<serde_json::Value, Checkpoint> {
        let is_canceled = ManuallyMove::new(false);
        let sigquit_handler = unsafe {
            let is_canceled = is_canceled.clone();
            register(SIGQUIT, move || {
                let mut is_canceled = is_canceled.mov();
                *is_canceled = true;
            })
        };

        let output = match &self.configuration.strategy {
            strategies::Strategy::Exhaustive(_) => {
                let mut ranking = strategies::exhaustive::output::Ranking::new(
                    &self.configuration.direction,
                    candidates,
                );
                let mut state = if let Some(Checkpoint::Exhaustive(state)) = checkpoint {
                    state
                } else {
                    self.configuration.profile.iter()
                };

                let mut count = 1;
                for individual in &mut state {
                    guard!(SIGQUIT, {
                        println!("{}/{}: ", count, self.configuration.profile.len());

                        let result = self.evaluate(&individual, repetition);
                        print!("{}", result);
                        if let Some(unit) = &self.configuration.unit {
                            print!(" {}", unit);
                        }
                        println!();
                        if verbose {
                            println!("{}", self.configuration.profile.stringify(&individual));
                        }
                        println!();

                        ranking.push(individual, result);
                    });

                    if *is_canceled {
                        break;
                    }

                    count += 1;
                }

                if *is_canceled {
                    second!(state.into())
                } else {
                    first!(ranking.into_json(&self.configuration.profile))
                }
            }
            strategies::Strategy::Genetic(options) => {
                let mut options = options.clone();
                let mut output = strategies::genetic::output::Output::new(
                    &self.configuration.direction,
                    candidates,
                );
                let mut state = if let Some(Checkpoint::Genetic(state)) = checkpoint {
                    state
                } else {
                    strategies::genetic::state::State::new(
                        &self.configuration.profile,
                        options.initial,
                    )
                };

                let mut rng = rand::rng();
                let mut temp_results = FxHashMap::default();
                // Rust compiler somehow optimizes this function call or later is_gt() call in wrong way
                // so wrap this call with black_box to prevent optimization
                let mut best_overall = hint::black_box(self.configuration.direction.worst());
                loop {
                    let mut evaluation_results = Vec::with_capacity(state.population.len());

                    // evaluate individuals
                    let len = state.population.len();
                    let mut index = 0;
                    while index < len {
                        guard!(SIGQUIT, {
                            let result = if let Some(&result) = temp_results.get(&index) {
                                result
                            } else {
                                print!("{}", state.generation);
                                if let Some(limit) = options.terminate.limit {
                                    print!("/{}", limit);
                                } else {
                                    print!(";");
                                }
                                print!(" {}/{}: ", index + 1, len);

                                let result = self.evaluate(&state.population[index], repetition);
                                print!("{}", result);
                                if result.is_finite() {
                                    if let Some(unit) = &self.configuration.unit {
                                        print!(" {}", unit);
                                    }
                                }
                                println!();

                                if verbose {
                                    println!(
                                        "{}",
                                        self.configuration
                                            .profile
                                            .stringify(&state.population[index])
                                    );
                                }
                                println!();

                                result
                            };

                            if state.generation == 1 && result.is_infinite() {
                                state.population[index] = strategies::genetic::state::State::sample(
                                    &self.configuration.profile,
                                );
                                continue;
                            } else {
                                output.ranking.push(state.population[index].clone(), result);
                                evaluation_results.push((result, index));
                                index += 1;
                            }
                        });

                        if *is_canceled {
                            break;
                        }
                    }

                    if *is_canceled {
                        break;
                    }

                    // record generation summary
                    let iter = evaluation_results
                        .iter()
                        .map(|(x, _)| *x)
                        .filter(|x| x.is_finite());
                    let boundaries = self.configuration.direction.boundaries(iter);
                    let summary = strategies::genetic::GenerationSummary::new(
                        output
                            .ranking
                            .best()
                            .map(|x| x.log(&self.configuration.profile))
                            .unwrap(),
                        boundaries,
                    );
                    println!("=== Generation #{} Summary ===", state.generation);
                    summary.print(&self.configuration.unit);
                    output.history.push(summary);

                    let (best, worst) = boundaries;
                    if self
                        .configuration
                        .direction
                        .compare(best, best_overall)
                        .is_gt()
                    {
                        state.count = 0;
                        best_overall = best;
                    } else {
                        state.count += 1;
                    }

                    // termination check
                    if let Some(endure) = options.terminate.endure {
                        print!("{}/{}\n\n", state.count, endure);
                        if state.count == endure {
                            break;
                        }
                    }

                    state.generation += 1;
                    if let Some(limit) = options.terminate.limit {
                        if state.generation > limit {
                            break;
                        }
                    }

                    // select individuals to remove
                    let mut inverted = evaluation_results.clone();
                    for pair in &mut inverted {
                        if pair.0.is_infinite() {
                            pair.0 = worst;
                            continue;
                        }

                        pair.0 = match self.configuration.direction {
                            Direction::Minimize => pair.0,
                            Direction::Maximize => worst - pair.0,
                        };
                    }
                    self.configuration.direction.sort(&mut inverted);
                    inverted.truncate(inverted.len() - options.remain);
                    inverted.shuffle(&mut rng);
                    let mut holes = strategies::genetic::stochastic_universal_sampling(
                        &inverted,
                        options.delete.value,
                    );
                    drop(inverted);

                    for result in &mut evaluation_results {
                        if result.0.is_infinite() {
                            result.0 = best;
                            continue;
                        }

                        result.0 = match self.configuration.direction {
                            Direction::Minimize => worst - result.0,
                            Direction::Maximize => result.0,
                        };
                    }
                    evaluation_results.shuffle(&mut rng);

                    // generate & evaluate children
                    let mut children = Vec::with_capacity(options.generate.value);
                    temp_results.clear();
                    let mut index = 0;
                    while index < options.generate.value {
                        let result = strategies::genetic::stochastic_universal_sampling(
                            &evaluation_results,
                            2,
                        );
                        let mut child = strategies::genetic::crossover(
                            &self.configuration.profile,
                            &state.population[result[0]],
                            &state.population[result[1]],
                        );
                        strategies::genetic::mutate(
                            &self.configuration.profile,
                            &options.mutate,
                            &mut child,
                        );

                        guard!(SIGQUIT, {
                            print!("{}", state.generation);
                            if let Some(limit) = options.terminate.limit {
                                print!("/{}", limit);
                            } else {
                                print!(";");
                            }
                            print!(" {}/{}: ", index + 1, options.generate.value);

                            let result = self.evaluate(&child, repetition);
                            print!("{}", result);
                            if result.is_finite() {
                                if let Some(unit) = &self.configuration.unit {
                                    print!(" {}", unit);
                                }
                            }
                            println!();

                            if verbose {
                                println!("{}", self.configuration.profile.stringify(&child));
                            }
                            println!();

                            // retry
                            if result.is_infinite() {
                                continue;
                            }

                            temp_results.insert(
                                if index < options.delete.value {
                                    holes[index]
                                } else {
                                    index
                                },
                                result,
                            );
                        });

                        children.push(child);
                        index += 1;
                    }

                    // replace individuals with children
                    let min = options.generate.value.min(options.delete.value);
                    let generated = children.split_off(min);
                    let deleted = holes.split_off(min);
                    assert!(generated.is_empty() || deleted.is_empty());
                    for (index, child) in children.into_iter().enumerate() {
                        state.population[holes[index]] = Rc::new(child);
                    }
                    if !generated.is_empty() {
                        for child in generated.into_iter() {
                            state.population.push(Rc::new(child));
                        }
                    }
                    if !deleted.is_empty() {
                        for index in deleted {
                            state.population.remove(index);
                        }
                    }

                    for _ in 0..options.infuse.value {
                        state
                            .population
                            .push(strategies::genetic::state::State::sample(
                                &self.configuration.profile,
                            ));
                    }

                    options.step();
                }

                if *is_canceled {
                    second!(state.into())
                } else {
                    first!(output.into_json(&self.configuration.profile))
                }
            }
        };

        ManuallyMove::drop(is_canceled);

        if let Ok(sigquit_handler) = sigquit_handler {
            unregister(sigquit_handler);
        }

        output
    }

    fn evaluate(&self, individual: &Individual, repetition: usize) -> f64 {
        let temp_dir = self.temp_dir.path();

        let mut context = Context::new(
            &self.configuration.profile,
            individual,
            temp_dir.as_os_str().as_encoded_bytes(),
        );
        for name in &self.configuration.hooks.pre {
            unsafe {
                let task = self.hook.get::<Hook>(name.as_bytes()).unwrap();
                task.call(&mut context, &self.workspace);
            }
        }
        if let context::Result::Invalid = context.result {
            return self.configuration.criterion.invalid();
        }

        let path = temp_dir
            .join("individuals")
            .join(individual.id.as_ref())
            .with_extension("so");
        if !path.exists() {
            compile::compile(
                &self.configuration.compiler,
                &path,
                self.sources
                    .iter()
                    .chain(self.configuration.compiler_arguments.iter())
                    .chain(context.arguments.iter())
                    .chain(
                        self.configuration
                            .profile
                            .compiler_arguments(&individual)
                            .iter(),
                    ),
            )
            .unwrap();
        }
        let lib = unsafe { Library::new(&path) }.unwrap();
        let runner = unsafe { lib.get::<Runner>(self.configuration.runner.as_bytes()) }.unwrap();

        let mut fitnesses = Vec::with_capacity(repetition);
        for _ in 0..repetition {
            unsafe {
                let result = register_unchecked(SIGSEGV, |_| {
                    // can we do better than this?
                    println!("Segmentation fault occurred during evaluation");
                    process::exit(1);
                });
                let affinity = self.cores.as_ref().map(|cores| {
                    let affinity = affinity::get_thread_affinity().unwrap();
                    affinity::set_thread_affinity(&cores).unwrap();
                    affinity
                });
                runner.call(&mut context, &self.workspace);
                if let Some(affinity) = affinity {
                    affinity::set_thread_affinity(&affinity).unwrap();
                }
                if let Ok(id) = result {
                    unregister(id);
                }
            };
            let fitness = context.result.unwrap(&self.configuration.criterion);
            if fitness.is_nan() {
                panic!("NaN value encountered");
            }
            fitnesses.push(fitness);
        }

        drop(lib);

        context.result =
            context::Result::Valid(self.configuration.criterion.representative(fitnesses));

        for name in &self.configuration.hooks.post {
            unsafe {
                let task = self.hook.get::<Hook>(name.as_bytes()).unwrap();
                task.call(&mut context, &self.workspace);
            }
        }

        context.result.unwrap(&self.configuration.criterion)
    }
}

fn main() -> anyhow::Result<()> {
    let args: Options = argh::from_env();
    let configuration =
        fs::read_to_string(&args.configuration).expect("Failed to read configuration file");
    let configuration = serde_json::from_str::<Configuration>(&configuration)
        .expect("Failed to parse configuration file");

    let autotuner = Autotuner::new(
        &args.sources,
        &args.helper,
        &args.hook,
        configuration,
        &Some(args.cores),
    )?;
    let state = args.continue_.as_ref().map(|filename| {
        let content = fs::read_to_string(filename).expect("Failed to read checkpoint file");
        serde_json::from_str::<Checkpoint>(&content).expect("Failed to parse checkpoint file")
    });
    match_union!(
        autotuner.run(
            args.repetition,
            args.candidates,
            state,
            args.verbose,
        );
        output => {
            fs::write(
                &args.output,
                serde_json::to_string_pretty(&output).expect("Failed to serialize output"),
            )
            .expect("Failed to write results to file");
        },
        checkpoint => {
            let filename = format!(
                "checkpoint.{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            fs::write(
                &filename,
                serde_json::to_string(&checkpoint).expect("Failed to serialize checkpoint"),
            )
            .expect("Failed to write checkpoint to file");
            println!("Saved checkpoint to {}", filename);
        }
    );

    Ok(())
}
