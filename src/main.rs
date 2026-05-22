mod compile;
mod configuration;
mod criterion;
mod direction;
mod ffi;
mod genetic;
mod individual;
mod output;
mod parameter;
mod state;
mod utils;

use crate::{
    configuration::{Configuration, StopAction},
    direction::Direction,
    ffi::{context::Context, helper::Helper, hook::Hook, runner::Runner, workspace::Workspace},
    individual::{Fitness, Individual, Representative},
    parameter::IntoJson,
    utils::{manually_move::ManuallyMove, union::Union},
};
use anyhow::anyhow;
use argh::{FromArgValue, FromArgs};
use fxhash::FxHashSet;
use libc::{SIGQUIT, SIGSEGV};
use libloading::Library;
use rand::seq::SliceRandom;
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use signal_hook_registry::{register, register_unchecked, unregister};
use std::{fs, hint, path, process, time::SystemTime};
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
    repeat: usize,

    #[argh(option, default = "32")]
    /// number of candidates (default: 32)
    candidates: usize,

    #[argh(option, arg_name = "continue")]
    /// path to checkpoint file
    continue_: Option<String>,

    #[argh(option, default = "\"result.json\".to_string()")]
    /// output file (default: result.json)
    output: String,

    #[argh(option, short = 'l', default = "LogLevel::Normal")]
    /// logging level
    log_level: LogLevel,
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub(crate) enum LogLevel {
    Quiet = 0,
    Normal = 1,
    Verbose = 2,
}

impl FromArgValue for LogLevel {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "quiet" => Ok(LogLevel::Quiet),
            "normal" => Ok(LogLevel::Normal),
            "verbose" => Ok(LogLevel::Verbose),
            _ => Err(format!("Invalid log level: {}", value)),
        }
    }
}

struct Autotuner<'a> {
    sources: &'a [String],
    configuration: Configuration,
    cores: &'a [usize],
    temp_directory: TempDir,
    helper: Library,
    hook: Library,
    workspace: Workspace<'a>,
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
        cores: &'a [usize],
    ) -> anyhow::Result<Self> {
        if configuration.hyperparameters.initial_population <= 1 {
            return Err(anyhow!("Initial population size must be greater than 1"));
        }
        if configuration.hyperparameters.generate.value == 0 {
            return Err(anyhow!("Number of each generation must be greater than 0"));
        }

        let temp_directory = TempDir::new("autotuner")?;
        fs::create_dir(temp_directory.path().join("individuals"))?;

        let path = temp_directory.path().join("libhelper.so");
        compile::compile(
            &configuration.compiler,
            &path,
            helper.iter().chain(configuration.compiler_arguments.iter()),
        )?;
        let helper = unsafe { Library::new(&path) }?;

        let path = temp_directory.path().join("libhook.so");
        compile::compile(
            &configuration.compiler,
            &path,
            hook.iter().chain(configuration.compiler_arguments.iter()),
        )?;
        let hook = unsafe { Library::new(&path) }?;

        let mut workspace = Workspace::new();

        unsafe {
            let initializer = helper
                .get::<Helper>(configuration.helper.pre.as_bytes())
                .unwrap();
            initializer.call(&mut workspace);
        }

        Ok(Autotuner {
            sources,
            configuration,
            temp_directory,
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
        checkpoint: Option<state::State>,
        log_level: LogLevel,
    ) -> Union<serde_json::Value, state::State> {
        let is_signaled = ManuallyMove::new(false);
        let sigquit_handler = unsafe {
            let is_signaled = is_signaled.clone();
            register(SIGQUIT, move || {
                let mut is_signaled = is_signaled.mov();
                *is_signaled = true;
            })
        };

        let mut output = output::Output::new(self.configuration.direction, candidates);
        let mut state = if let Some(state) = checkpoint {
            state
        } else {
            state::State::new(
                &self.configuration.hyperparameters,
                &self.configuration.profile,
            )
        };

        let mut rng = rand::rng();
        // Rust compiler somehow optimizes this function call or later is_gt() call in wrong way
        // so wrap this call with black_box to prevent optimization
        let mut best_overall = hint::black_box(self.configuration.direction.worst());
        loop {
            // remove duplicates
            let mut seen = FxHashSet::default();
            for individual in &mut state.population {
                if seen.insert(individual.id.clone()) {
                    continue;
                }

                let mut replacement = Individual::random(&self.configuration.profile);
                while !seen.insert(replacement.id.clone()) {
                    replacement = Individual::random(&self.configuration.profile);
                }
                *individual = replacement;
            }
            drop(seen);

            state.population.par_iter_mut().for_each(|individual| {
                self.compile(individual);
            });

            // evaluate individuals
            let len = state.population.len();
            let mut index = 0;
            while index < len {
                guard!(SIGQUIT, {
                    let individual = &mut state.population[index];
                    if let Fitness::Unknown = individual.fitness {
                        self.evaluate(individual, repetition);
                        if individual.fitness.is_valid() || log_level >= LogLevel::Normal {
                            print!("{}", state.generation);
                            if let Some(limit) = state.hyperparameters.terminate.limit {
                                print!("/{}", limit);
                            } else {
                                print!(";");
                            }
                            print!(" {}/{}: {}", index + 1, len, individual.fitness);
                            if individual.fitness.is_valid() {
                                if let Some(unit) = &self.configuration.unit {
                                    print!(" {}", unit);
                                }
                            }
                            println!();
                            if log_level >= LogLevel::Verbose {
                                println!(
                                    "{}",
                                    self.configuration.profile.individual_to_string(individual)
                                );
                            }
                            println!();
                        }
                    }
                });

                if *is_signaled {
                    break;
                }

                output.ranking.push(&state.population[index]);
                index += 1;
            }

            if *is_signaled {
                break;
            }

            let mut flattened = state
                .population
                .iter()
                .enumerate()
                .map(|(index, individual)| {
                    (
                        individual.fitness.into_f64(self.configuration.criterion),
                        index,
                    )
                })
                .collect::<Vec<_>>();

            // record generation summary
            let iter = flattened.iter().map(|(x, _)| *x).filter(|x| x.is_finite());
            let boundaries = self.configuration.direction.boundaries(iter);
            let summary =
                genetic::GenerationSummary::new(output.ranking.best().unwrap(), boundaries);
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
            state.generation += 1;
            if let Some(limit) = state.hyperparameters.terminate.limit {
                if state.generation > limit {
                    break;
                }
            }

            if let Some(goal) = state.hyperparameters.terminate.goal {
                if self
                    .configuration
                    .direction
                    .compare(best_overall, goal)
                    .is_ge()
                {
                    state.hyperparameters.terminate.goal = None;
                }
            } else if let Some(endure) = state.hyperparameters.terminate.endure {
                print!("{}/{}\n", state.count, endure);
                if state.count == endure {
                    break;
                }
            }

            println!();

            // select individuals to remove
            let mut inverted = flattened.clone();
            for pair in &mut inverted {
                if pair.0.is_infinite() {
                    pair.0 = worst;
                    continue;
                }

                pair.0 = match self.configuration.direction {
                    Direction::Minimize => pair.0,
                    Direction::Maximize => best - pair.0,
                };
            }
            match self.configuration.direction {
                Direction::Minimize => inverted.sort_by(|a, b| b.0.total_cmp(&a.0)),
                Direction::Maximize => inverted.sort_by(|a, b| a.0.total_cmp(&b.0)),
            }
            inverted.truncate(inverted.len() - state.hyperparameters.remain);
            inverted.shuffle(&mut rng);
            let mut holes = genetic::stochastic_universal_sampling(
                &inverted,
                state.hyperparameters.delete.value,
            );
            drop(inverted);

            for result in &mut flattened {
                if result.0.is_infinite() {
                    result.0 = best;
                    continue;
                }

                result.0 = match self.configuration.direction {
                    Direction::Minimize => worst - result.0,
                    Direction::Maximize => result.0,
                };
            }
            flattened.shuffle(&mut rng);

            for individual in &mut state.population {
                individual.reset();
            }

            // generate & evaluate children
            let mut children = Vec::with_capacity(state.hyperparameters.generate.value);
            while children.len() < state.hyperparameters.generate.value {
                let num_children = children.len();
                let num_current = state.hyperparameters.generate.value - num_children;
                let mut current = (0..num_current)
                    .into_par_iter()
                    .map(|_| {
                        let result = genetic::stochastic_universal_sampling(&flattened, 2);
                        let mut child = genetic::crossover(
                            &self.configuration.profile,
                            &state.population[result[0]],
                            &state.population[result[1]],
                        );
                        genetic::mutate(
                            &self.configuration.profile,
                            &state.hyperparameters.mutate,
                            &mut child,
                        );
                        self.compile(&mut child);
                        child
                    })
                    .collect::<Vec<_>>();

                let mut index = 0;
                while index < num_current {
                    guard!(SIGQUIT, {
                        let child = &mut current[index];
                        if let Fitness::Unknown = child.fitness {
                            self.evaluate(child, repetition);
                            if child.fitness.is_valid() || log_level >= LogLevel::Normal {
                                print!("{}", state.generation);
                                if let Some(limit) = state.hyperparameters.terminate.limit {
                                    print!("/{}", limit);
                                } else {
                                    print!(";");
                                }
                                print!(
                                    " {}/{}: {}",
                                    num_children + index + 1,
                                    state.hyperparameters.generate.value,
                                    child.fitness
                                );
                                if child.fitness.is_valid() {
                                    if let Some(unit) = &self.configuration.unit {
                                        print!(" {}", unit);
                                    }
                                }
                                println!();
                                if log_level >= LogLevel::Verbose {
                                    println!(
                                        "{}",
                                        self.configuration.profile.individual_to_string(child)
                                    );
                                }
                                println!();
                            }
                        }
                    });

                    if *is_signaled {
                        break;
                    }

                    index += 1;
                }

                if *is_signaled {
                    break;
                }

                for child in current {
                    if child.fitness.is_valid() {
                        children.push(child);
                    }
                }
            }

            if *is_signaled {
                break;
            }

            // replace individuals with children
            let min = state
                .hyperparameters
                .generate
                .value
                .min(state.hyperparameters.delete.value);
            let generated = children.split_off(min);
            let mut deleted = holes.split_off(min);
            assert!(generated.is_empty() || deleted.is_empty());
            for (index, child) in children.into_iter().enumerate() {
                state.population[holes[index]] = child;
            }
            if !generated.is_empty() {
                for child in generated.into_iter() {
                    state.population.push(child);
                }
            }
            if !deleted.is_empty() {
                deleted.sort();
                for index in deleted.into_iter().rev() {
                    // FIXME: strange behavior
                    state.population.remove(index);
                }
            }

            for _ in 0..state.hyperparameters.infuse.value {
                state
                    .population
                    .push(Individual::random(&self.configuration.profile));
            }

            state.step();
        }

        let output = if *is_signaled && self.configuration.stop_action == StopAction::SaveState {
            second!(state.into())
        } else {
            first!(output.into_json(&self.configuration.profile))
        };

        ManuallyMove::drop(is_signaled);

        if let Ok(sigquit_handler) = sigquit_handler {
            unregister(sigquit_handler);
        }

        output
    }

    #[inline]
    fn get_working_directory(&self, individual: &Individual) -> path::PathBuf {
        self.temp_directory
            .path()
            .join("individuals")
            .join(individual.id.as_ref())
    }

    fn compile(&self, individual: &mut Individual) {
        let working_directory = self.get_working_directory(individual);
        if !working_directory.exists() {
            fs::create_dir(&working_directory).unwrap();
        }

        let path = working_directory.join("lib.so");
        if path.exists() {
            return;
        }

        let mut context = Context::new(self, individual);
        for name in &self.configuration.hooks.pre {
            unsafe {
                let task = self.hook.get::<Hook>(name.as_bytes()).unwrap();
                task.call(&mut context);
            }
        }
        if context.individual.fitness == Fitness::Invalid {
            return;
        }

        compile::compile(
            &self.configuration.compiler,
            &path,
            self.sources
                .iter()
                .chain(self.configuration.compiler_arguments.iter())
                .chain(context.individual.arguments.iter()),
        )
        .unwrap();
    }

    fn evaluate(&self, individual: &mut Individual, repetition: usize) {
        let working_directory = self.get_working_directory(individual);
        if !working_directory.exists() {
            return;
        }

        let path = working_directory.join("lib.so");
        let lib = unsafe { Library::new(&path) }.unwrap();
        let runner = unsafe { lib.get::<Runner>(self.configuration.runner.as_bytes()) }.unwrap();

        let mut context = Context::new(self, individual);
        let mut fitnesses = Vec::with_capacity(repetition);
        for _ in 0..repetition {
            unsafe {
                let result = register_unchecked(SIGSEGV, |_| {
                    // can we do better than this?
                    println!("Segmentation fault occurred during evaluation");
                    process::exit(1);
                });
                let affinity = if self.cores.is_empty() {
                    None
                } else {
                    let affinity = affinity::get_thread_affinity().unwrap();
                    affinity::set_thread_affinity(&self.cores).unwrap();
                    Some(affinity)
                };
                runner.call(&mut context);
                if let Some(affinity) = affinity {
                    affinity::set_thread_affinity(&affinity).unwrap();
                }
                if let Ok(id) = result {
                    unregister(id);
                }
            };
            if context.individual.fitness.is_nan() {
                panic!("NaN value encountered");
            }
            fitnesses.push(context.individual.fitness);
        }

        drop(lib);

        context.individual.fitness = fitnesses.representative(self.configuration.criterion);

        for name in &self.configuration.hooks.post {
            unsafe {
                let task = self.hook.get::<Hook>(name.as_bytes()).unwrap();
                task.call(&mut context);
            }
        }
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
        &args.cores,
    )?;
    let state = args.continue_.as_ref().map(|filename| {
        let content = fs::read_to_string(filename).expect("Failed to read checkpoint file");
        serde_json::from_str::<state::State>(&content).expect("Failed to parse checkpoint file")
    });
    match_union!(
        autotuner.run(
            args.repeat,
            args.candidates,
            state,
            args.log_level,
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
