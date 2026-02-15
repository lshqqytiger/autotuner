mod compile;
mod context;
mod criterion;
mod direction;
mod execution_log;
mod heap;
mod helper;
mod hook;
mod metadata;
mod parameter;
mod runner;
mod strategies;
mod utils;
mod workspace;

use crate::{
    context::Context,
    direction::{Direction, SortAndReverse},
    execution_log::IntoLogs,
    helper::Helper,
    hook::Hook,
    metadata::Metadata,
    parameter::Instance,
    runner::Runner,
    strategies::{exhaustive::Exhaustive, Checkpoint},
    utils::{manually_move::ManuallyMove, union::Union},
};
use anyhow::anyhow;
use argh::FromArgs;
use fxhash::FxHashMap;
use libc::{SIGQUIT, SIGSEGV};
use libloading::Library;
use rand::seq::SliceRandom;
use serde::Serialize;
use signal_hook_registry::{register, register_unchecked, unregister};
use std::{fs, io, process, rc::Rc, time::SystemTime};
use tempdir::TempDir;

#[derive(FromArgs)]
/// CLI Arguments
struct Options {
    #[argh(positional)]
    sources: Vec<String>,

    #[argh(option, default = "Vec::new()")]
    /// path to helper files
    helper: Vec<String>,

    #[argh(option, default = "Vec::new()")]
    /// path to hook files
    hook: Vec<String>,

    #[argh(option, short = 'm')]
    /// path to metadata file (required)
    metadata: String,

    #[argh(subcommand)]
    /// search strategy (default: genetic)
    strategy: Strategy,

    #[argh(option, short = 'r', default = "15")]
    /// number of repetitions for each instance (default: 15)
    repetition: usize,

    #[argh(option, default = "32")]
    /// number of candidates (default: 32)
    candidates: usize,

    #[argh(option, arg_name = "continue")]
    /// path to checkpoint file
    continue_: Option<String>,

    #[argh(option, default = "\"results.json\".to_string()")]
    /// output file (default: results.json)
    output: String,

    #[argh(switch, short = 'v')]
    /// verbose output
    verbose: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Strategy {
    Exhaustive(strategies::exhaustive::options::ExhaustiveSearchOptions),
    Genetic(strategies::genetic::options::GeneticSearchOptions),
}

struct Output<'a>(&'a String, String);

impl<'a> Output<'a> {
    fn new<T: Serialize>(path: &'a String, object: T) -> serde_json::Result<Self> {
        let value = serde_json::to_string_pretty(&object)?;
        Ok(Output(path, value))
    }

    fn save(self) -> io::Result<()> {
        fs::write(self.0, self.1)
    }
}

struct Autotuner<'a> {
    sources: &'a [String],
    metadata: Metadata,
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
                .get::<Helper>(self.metadata.helper.post.as_bytes())
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
        metadata: Metadata,
    ) -> anyhow::Result<Self> {
        let temp_dir = TempDir::new("autotuner")?;

        fs::create_dir(temp_dir.path().join("instances"))?;

        let path = temp_dir.path().join("libhelper.so");
        compile::compile(
            &metadata.compiler,
            &path,
            helper.iter().chain(metadata.compiler_arguments.iter()),
        )?;
        let helper = unsafe { Library::new(&path) }?;

        let path = temp_dir.path().join("libhook.so");
        compile::compile(
            &metadata.compiler,
            &path,
            hook.iter().chain(metadata.compiler_arguments.iter()),
        )?;
        let hook = unsafe { Library::new(&path) }?;

        let mut workspace = workspace::Workspace::new();

        unsafe {
            let initializer = helper
                .get::<Helper>(metadata.helper.pre.as_bytes())
                .unwrap();
            initializer.call(&mut workspace);
        }

        Ok(Autotuner {
            sources,
            metadata,
            temp_dir,
            helper,
            hook,
            workspace,
        })
    }

    fn run(
        &'a self,
        strategy: &'a Strategy,
        repetition: usize,
        candidates: usize,
        checkpoint: Option<Checkpoint>,
        verbose: bool,
        path: &'a String,
    ) -> Union<Vec<Output<'a>>, Checkpoint> {
        let is_canceled = ManuallyMove::new(false);
        let sigquit_handler = unsafe {
            let is_canceled = is_canceled.clone();
            register(SIGQUIT, move || {
                let mut is_canceled = is_canceled.mov();
                *is_canceled = true;
            })
        };

        let output = match strategy {
            Strategy::Exhaustive(_) => {
                let mut ranking = strategies::exhaustive::ranking::Ranking::new(
                    &self.metadata.direction,
                    candidates,
                );
                let mut state = if let Some(Checkpoint::Exhaustive(state)) = checkpoint {
                    state
                } else {
                    self.metadata.profile.iter()
                };

                let mut count = 1;
                for instance in &mut state {
                    guard!(SIGQUIT, {
                        println!("{}/{}: ", count, self.metadata.profile.len());

                        let result = self.evaluate(&instance, repetition);
                        println!("{}", result);
                        if verbose {
                            println!("{}", self.metadata.profile.display(&instance));
                        }
                        println!();

                        ranking.push(instance, result);
                    });

                    if *is_canceled {
                        break;
                    }

                    count += 1;
                }

                if *is_canceled {
                    second!(state.into())
                } else {
                    let mut output = ranking.to_vec();
                    output.reverse();
                    let output = output.into_logs(&self.metadata.profile);
                    first!(vec![
                        Output::new(path, output).expect("Failed to serialize object"),
                    ])
                }
            }
            Strategy::Genetic(options) => {
                let mut ranking = strategies::genetic::ranking::Ranking::new(
                    &self.metadata.direction,
                    candidates,
                );
                let mut state = if let Some(Checkpoint::Genetic(state)) = checkpoint {
                    state
                } else {
                    strategies::genetic::state::State::new(&self.metadata.profile, options.initial)
                };
                let mut history = Vec::new();

                let mut rng = rand::rng();
                let mut temp_results = FxHashMap::default();
                loop {
                    let mut evaluation_results = Vec::with_capacity(state.instances.len());

                    // evaluate instances
                    let len = state.instances.len();
                    let mut index = 0;
                    while index < len {
                        guard!(SIGQUIT, {
                            let result = if let Some(&result) = temp_results.get(&index) {
                                result
                            } else {
                                print!(
                                    "{}/{} {}/{}: ",
                                    state.generation,
                                    options.limit,
                                    index + 1,
                                    len
                                );

                                let result = self.evaluate(&state.instances[index], repetition);
                                println!("{}", result);
                                if verbose {
                                    println!(
                                        "{}",
                                        self.metadata.profile.display(&state.instances[index])
                                    );
                                }
                                println!();

                                result
                            };

                            if state.generation == 1 && result.is_infinite() {
                                state.regenerate(&self.metadata.profile, index);
                                continue;
                            } else {
                                ranking.push(state.instances[index].clone(), result);
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
                    let boundaries = self.metadata.direction.boundaries(iter);
                    let summary = strategies::genetic::GenerationSummary::new(
                        ranking
                            .best()
                            .cloned()
                            .map(|x| x.into_log(&self.metadata.profile)),
                        boundaries,
                    );
                    println!("=== Generation #{} Summary ===", state.generation);
                    println!("{}", summary);
                    history.push(summary);

                    state.generation += 1;
                    if state.generation > options.limit {
                        break;
                    }

                    let (best, worst) = boundaries;
                    let mut inverted = evaluation_results.clone();
                    for pair in &mut inverted {
                        if pair.0.is_infinite() {
                            pair.0 = worst;
                            continue;
                        }

                        pair.0 = match self.metadata.direction {
                            Direction::Minimize => pair.0,
                            Direction::Maximize => worst - pair.0,
                        };
                    }
                    self.metadata.direction.sort_and_reverse(&mut inverted);
                    inverted.truncate(inverted.len() - options.remain);
                    inverted.shuffle(&mut rng);
                    let holes = strategies::genetic::stochastic_universal_sampling(
                        &inverted,
                        options.generate,
                    );
                    drop(inverted);

                    for result in &mut evaluation_results {
                        if result.0.is_infinite() {
                            result.0 = best;
                            continue;
                        }

                        result.0 = match self.metadata.direction {
                            Direction::Minimize => worst - result.0,
                            Direction::Maximize => result.0,
                        };
                    }
                    evaluation_results.shuffle(&mut rng);

                    // generate & evaluate children
                    let mut children = Vec::with_capacity(options.generate);
                    temp_results.clear();
                    let mut index = 0;
                    while index < options.generate {
                        let result = strategies::genetic::stochastic_universal_sampling(
                            &evaluation_results,
                            2,
                        );
                        let mut child = strategies::genetic::crossover(
                            &self.metadata.profile,
                            &state.instances[result[0]],
                            &state.instances[result[1]],
                        );
                        strategies::genetic::mutate(&self.metadata.profile, &mut child);

                        guard!(SIGQUIT, {
                            print!(
                                "{}/{} {}/{}: ",
                                state.generation,
                                options.limit,
                                index + 1,
                                options.generate
                            );

                            let result = self.evaluate(&child, repetition);
                            println!("{}", result);
                            if verbose {
                                println!("{}", self.metadata.profile.display(&child));
                            }
                            println!();

                            // retry
                            if result.is_infinite() {
                                continue;
                            }

                            temp_results.insert(holes[index], result);
                        });

                        children.push(child);
                        index += 1;
                    }

                    // replace instances with children
                    for (index, instance) in children.into_iter().enumerate() {
                        state.instances[holes[index]] = Rc::new(instance);
                    }
                }

                if *is_canceled {
                    second!(state.into())
                } else {
                    let mut output = ranking.to_vec();
                    output.reverse();
                    let output = output.into_logs(&self.metadata.profile);
                    let mut outputs =
                        vec![Output::new(path, output).expect("Failed to serialize object")];
                    if let Some(path) = &options.history {
                        outputs
                            .push(Output::new(path, history).expect("Failed to serialize object"));
                    }
                    first!(outputs)
                }
            }
        };

        ManuallyMove::drop(is_canceled);

        if let Ok(sigquit_handler) = sigquit_handler {
            unregister(sigquit_handler);
        }

        output
    }

    fn evaluate(&self, instance: &Instance, repetition: usize) -> f64 {
        let temp_dir = self.temp_dir.path();

        let mut context = Context::new(
            &self.metadata.profile,
            instance,
            temp_dir.as_os_str().as_encoded_bytes(),
        );
        for name in &self.metadata.hooks.pre {
            unsafe {
                let task = self.hook.get::<Hook>(name.as_bytes()).unwrap();
                task.call(&mut context, &self.workspace);
            }
        }
        if let context::Result::Invalid = context.result {
            return self.metadata.criterion.invalid();
        }

        let path = temp_dir
            .join("instances")
            .join(instance.id.as_ref())
            .with_extension("so");
        if !path.exists() {
            compile::compile(
                &self.metadata.compiler,
                &path,
                self.sources
                    .iter()
                    .chain(self.metadata.compiler_arguments.iter())
                    .chain(context.arguments.iter())
                    .chain(self.metadata.profile.compiler_arguments(&instance).iter()),
            )
            .unwrap();
        }
        let lib = unsafe { Library::new(&path) }.unwrap();
        let runner = unsafe { lib.get::<Runner>(self.metadata.runner.as_bytes()) }.unwrap();

        let mut fitnesses = Vec::with_capacity(repetition);
        for _ in 0..repetition {
            unsafe {
                let result = register_unchecked(SIGSEGV, |_| {
                    // can we do better than this?
                    println!("Segmentation fault occurred during evaluation");
                    process::exit(1);
                });
                runner.call(&mut context, &self.workspace);
                if let Ok(id) = result {
                    unregister(id);
                }
            };
            let fitness = context.result.unwrap(&self.metadata.criterion);
            if fitness.is_nan() {
                panic!("NaN value encountered");
            }
            fitnesses.push(fitness);
        }

        drop(lib);

        context.result = context::Result::Valid(self.metadata.criterion.representative(fitnesses));

        for name in &self.metadata.hooks.post {
            unsafe {
                let task = self.hook.get::<Hook>(name.as_bytes()).unwrap();
                task.call(&mut context, &self.workspace);
            }
        }

        context.result.unwrap(&self.metadata.criterion)
    }
}

fn main() -> anyhow::Result<()> {
    let args: Options = argh::from_env();
    match &args.strategy {
        Strategy::Exhaustive(_) => {}
        Strategy::Genetic(options) => {
            if options.initial <= 1 {
                return Err(anyhow!("Initial population size must be greater than 1"));
            }
            if options.generate == 0 {
                return Err(anyhow!("Number of each generation must be greater than 0"));
            }
            if args.continue_.is_some() && options.history.is_some() {
                return Err(anyhow!(
                    "Cannot specify history output file when continuing from checkpoint"
                ));
            }
        }
    }

    let metadata = fs::read_to_string(&args.metadata).expect("Failed to read metadata file");
    let metadata =
        serde_json::from_str::<Metadata>(&metadata).expect("Failed to parse metadata file");

    let autotuner = Autotuner::new(&args.sources, &args.helper, &args.hook, metadata)?;
    let state = args.continue_.as_ref().map(|filename| {
        let content = fs::read_to_string(filename).expect("Failed to read checkpoint file");
        serde_json::from_str::<Checkpoint>(&content).expect("Failed to parse checkpoint file")
    });
    match_union!(
        autotuner.run(
            &args.strategy,
            args.repetition,
            args.candidates,
            state,
            args.verbose,
            &args.output,
        );
        output => {
            for output in output {
                output.save().expect("Failed to write output file");
            }
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
