mod compile;
mod criterion;
mod direction;
mod execution_result;
mod helper;
mod ranking;
mod utils;
mod workspace;

use crate::{
    criterion::Criterion, direction::Direction, helper::*, ranking::Ranking,
    utils::exhaustive::Exhaustive, workspace::Workspace,
};
use anyhow::anyhow;
use argh::{FromArgValue, FromArgs};
use autotuner::{
    first, manually_move::ManuallyMove, match_union, metadata::Metadata, parameter::Instance,
    second, union::Union,
};
use libc::{SIGQUIT, SIGSEGV};
use libloading::{Library, Symbol};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use signal_hook_registry::{register, register_unchecked, unregister};
use std::{fs, io, process, ptr, sync::Arc, time::SystemTime};
use tempdir::TempDir;

trait OrNull<T> {
    fn or_null(self) -> *mut T;
}

impl<T> OrNull<T> for Option<*mut T> {
    fn or_null(self) -> *mut T {
        match self {
            Some(ptr) => ptr,
            None => ptr::null_mut(),
        }
    }
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

impl FromArgValue for Criterion {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "maximum" => Ok(Criterion::Maximum),
            "minimum" => Ok(Criterion::Minimum),
            "median" => Ok(Criterion::Median),
            _ => Err(format!("Invalid criterion: {}", value)),
        }
    }
}

#[derive(FromArgs)]
/// CLI Arguments
struct Options {
    #[argh(positional)]
    sources: Vec<String>,

    #[argh(option, short = 'm')]
    /// metadata file (required)
    metadata: String,

    #[argh(option, short = 'd', default = "Direction::Maximize")]
    /// optimization direction (default: maximize)
    direction: Direction,

    #[argh(subcommand)]
    /// search strategy (default: genetic)
    strategy: Strategy,

    #[argh(option, short = 'c', default = "Criterion::Maximum")]
    /// criterion to aggregate multiple runs (default: maximum)
    criterion: Criterion,

    #[argh(option, short = 'r', default = "15")]
    /// number of repetitions for each instance (default: 15)
    repetition: usize,

    #[argh(option, default = "32")]
    /// number of candidates (default: 32)
    candidates: usize,

    #[argh(option, arg_name = "continue")]
    /// continue from the saved state file
    continue_: Option<String>,

    #[argh(option, default = "\"results.json\".to_string()")]
    /// output file (default: results.json)
    output: String,

    #[argh(switch, short = 'v')]
    /// verbose output
    verbose: bool,
}

#[derive(FromArgs, PartialEq, Debug, Clone)]
/// exhaustive search options
#[argh(subcommand, name = "exhaustive")]
struct ExhaustiveSearchOptions {}

#[derive(FromArgs, PartialEq, Debug, Clone)]
/// genetic search options
#[argh(subcommand, name = "genetic")]
struct GeneticSearchOptions {
    #[argh(option, short = 'i', default = "256")]
    /// initial population size (default: 256)
    initial: usize,

    #[argh(option, short = 'n', default = "32")]
    /// number of instances that will be made at each generation (default: 32)
    ngeneration: usize,

    #[argh(option, short = 'l', default = "256")]
    /// maximum number of generations (default: 256)
    limit: usize,

    #[argh(option)]
    /// output file
    history: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Strategy {
    Exhaustive(ExhaustiveSearchOptions),
    Genetic(GeneticSearchOptions),
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

#[derive(Serialize, Deserialize)]
enum SavedState {
    Exhaustive(utils::exhaustive::SearchState),
    Genetic(utils::genetic::SearchState),
}

impl From<utils::exhaustive::SearchState> for SavedState {
    fn from(state: utils::exhaustive::SearchState) -> Self {
        SavedState::Exhaustive(state)
    }
}

impl From<utils::genetic::SearchState> for SavedState {
    fn from(state: utils::genetic::SearchState) -> Self {
        SavedState::Genetic(state)
    }
}

struct Autotuner<'s> {
    sources: &'s [String],
    metadata: Metadata,
    temp_dir: TempDir,
    base: Library,
    workspace: Workspace,
}

impl<'s> Autotuner<'s> {
    fn new(sources: &'s [String], metadata: Metadata) -> anyhow::Result<Self> {
        let temp_dir = TempDir::new("autotuner")?;
        let path = temp_dir.path().join("base");
        let base = compile::compile(
            &metadata.compiler,
            &path,
            sources.iter().chain(metadata.compiler_arguments.iter()),
        )?;
        let workspace = Workspace::new(&base, &metadata)?;

        Ok(Autotuner {
            sources,
            metadata,
            temp_dir,
            base,
            workspace,
        })
    }

    fn run<'a>(
        &'a self,
        direction: &Direction,
        strategy: &'a Strategy,
        criterion: &Criterion,
        repetition: usize,
        candidates: usize,
        state: Option<SavedState>,
        verbose: bool,
        path: &'a String,
    ) -> Union<Vec<Output<'a>>, SavedState> {
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
                let mut ranking = Ranking::new(direction, candidates);
                let mut state = if let Some(SavedState::Exhaustive(state)) = state {
                    state
                } else {
                    self.metadata.profile.iter()
                };

                let mut count = 1;
                for instance in &mut state {
                    unsafe {
                        utils::block(SIGQUIT);
                    }

                    println!("{}/{}: ", count, self.metadata.profile.len());

                    let result = match self.evaluate(&instance, repetition) {
                        Ok(values) => criterion.extract_representative(values),
                        Err(_) => f64::INFINITY,
                    };

                    println!("{} ms", result);
                    if verbose {
                        println!("{}", self.metadata.profile.display(&instance));
                    }
                    println!();
                    ranking.push(Arc::new(instance), result);

                    unsafe {
                        utils::unblock(SIGQUIT);
                    }

                    if *is_canceled {
                        break;
                    }

                    count += 1;
                }

                if *is_canceled {
                    second!(state.into())
                } else {
                    let mut output = ranking.to_vec();
                    direction.sort(&mut output);
                    let output: Vec<_> = output
                        .into_iter()
                        .map(|result| (self.metadata.profile.display(&result.0), result.1))
                        .collect();
                    first!(vec![
                        Output::new(path, output).expect("Failed to serialize object"),
                    ])
                }
            }
            Strategy::Genetic(options) => {
                let mut ranking = Ranking::new(direction, candidates);
                let mut state = if let Some(SavedState::Genetic(state)) = state {
                    state
                } else {
                    utils::genetic::SearchState::new(&self.metadata.profile, options.initial)
                };
                let mut history = Vec::new();

                let mut evaluation_results: Vec<(f64, usize)> = Vec::with_capacity(options.initial);
                let mut rng = rand::rng();
                while state.generation < options.limit {
                    if !evaluation_results.is_empty() {
                        let iter = evaluation_results
                            .iter()
                            .map(|(x, _)| *x)
                            .filter(|x| x.is_finite());
                        println!("=== Generation #{} Summary ===", state.generation);
                        let best = direction.best(iter.clone());
                        let worst = direction.worst(iter);
                        let summary = utils::genetic::GenerationSummary::new(
                            ranking.best().map(|x| x.1),
                            best,
                            worst,
                        );
                        print!("{}\n\n", summary);
                        history.push(summary);

                        let mut inversed = evaluation_results.clone();
                        for pair in &mut inversed {
                            if pair.0.is_infinite() {
                                pair.0 = worst;
                                continue;
                            }

                            pair.0 = match direction {
                                Direction::Minimize => pair.0,
                                Direction::Maximize => worst - pair.0,
                            };
                        }
                        inversed.shuffle(&mut rng);
                        let holes = utils::genetic::stochastic_universal_sampling(
                            &inversed,
                            options.ngeneration,
                        );
                        drop(inversed);

                        for result in &mut evaluation_results {
                            if result.0.is_infinite() {
                                result.0 = best;
                                continue;
                            }

                            result.0 = match direction {
                                Direction::Minimize => worst - result.0,
                                Direction::Maximize => result.0,
                            };
                        }
                        evaluation_results.shuffle(&mut rng);

                        let mut children = Vec::with_capacity(options.ngeneration);
                        for _ in 0..options.ngeneration {
                            let result = utils::genetic::stochastic_universal_sampling(
                                &evaluation_results,
                                2,
                            );
                            let mut child = utils::genetic::crossover(
                                &self.metadata.profile,
                                &state.instances[result[0]],
                                &state.instances[result[1]],
                            );
                            utils::genetic::mutate(&self.metadata.profile, &mut child);
                            children.push(child);
                        }

                        for (index, instance) in children.into_iter().enumerate() {
                            state.instances[holes[index]] = Arc::new(instance);
                        }

                        evaluation_results.clear();
                    }

                    let len = state.instances.len();
                    let mut fresh_instances = Vec::new();
                    for index in 0..len {
                        fresh_instances.push((index, state.instances[index].clone()));
                    }

                    let len = fresh_instances.len();
                    for i in 0..len {
                        unsafe {
                            utils::block(SIGQUIT);
                        }

                        print!(
                            "{}/{} {}/{}: ",
                            state.generation + 1,
                            options.limit,
                            i + 1,
                            len
                        );

                        let result = match self.evaluate(&fresh_instances[i].1, repetition) {
                            Ok(values) => criterion.extract_representative(values),
                            Err(_) => f64::INFINITY,
                        };
                        println!("{} ms", result);
                        if verbose {
                            println!("{}", self.metadata.profile.display(&fresh_instances[i].1));
                        }
                        println!();
                        ranking.push(state.instances[i].clone(), result);
                        evaluation_results.push((result, i));

                        unsafe {
                            utils::unblock(SIGQUIT);
                        }

                        if *is_canceled {
                            break;
                        }
                    }

                    if *is_canceled {
                        break;
                    }

                    state.generation += 1;
                }

                if *is_canceled {
                    second!(state.into())
                } else {
                    let mut output = ranking.to_vec();
                    direction.sort(&mut output);
                    let output: Vec<_> = output
                        .into_iter()
                        .map(|result| (self.metadata.profile.display(&result.0), result.1))
                        .collect();
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

        // The signal handler must be unregistered early enough.
        if let Ok(sigquit_handler) = sigquit_handler {
            unregister(sigquit_handler);
        }

        if let Some(ref name) = self.metadata.finalizer {
            unsafe {
                let finalizer = self.base.get::<Finalizer>(name.as_bytes()).unwrap();
                finalizer(
                    self.workspace.input_ptr,
                    self.workspace.output_ptr,
                    self.workspace.validation_ptr.or_null(),
                );
            }
        }

        output
    }

    fn evaluate(&self, instance: &Instance, repetition: usize) -> anyhow::Result<Vec<f64>> {
        let path = self.temp_dir.path().join(instance.id.as_ref());
        let lib = compile::compile(
            &self.metadata.compiler,
            &path,
            self.sources
                .iter()
                .chain(self.metadata.compiler_arguments.iter())
                .chain(self.metadata.profile.compiler_arguments(&instance).iter()),
        )?;
        let evaluator: Symbol<Evaluator> = unsafe { lib.get(self.metadata.evaluator.as_bytes()) }?;

        let mut fitnesses = Vec::with_capacity(repetition);
        for _ in 0..repetition {
            let fitness = unsafe {
                let result = register_unchecked(SIGSEGV, |_| {
                    // can we do better than this?
                    println!("Segmentation fault occurred during evaluation");
                    process::exit(1);
                });
                let fitness = evaluator(self.workspace.input_ptr, self.workspace.output_ptr);
                if let Ok(id) = result {
                    unregister(id);
                }
                fitness
            };
            if fitness.is_nan() {
                return Err(anyhow!("NaN value encountered"));
            }
            fitnesses.push(fitness);
        }

        if let Some(block) = self.workspace.validation_ptr {
            let validator: Symbol<Validator> =
                unsafe { lib.get(self.metadata.validator.as_ref().unwrap().as_bytes()) }?;
            if !unsafe { validator(block, self.workspace.output_ptr) } {
                return Err(anyhow!("Validation failed"));
            }
        }

        drop(lib);

        Ok(fitnesses)
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
            if options.ngeneration == 0 {
                return Err(anyhow!("Number of each generation must be greater than 0"));
            }
        }
    }

    let metadata = fs::read_to_string(&args.metadata).expect("Failed to read metadata file");
    let metadata =
        serde_json::from_str::<Metadata>(&metadata).expect("Failed to parse metadata file");

    let autotuner = Autotuner::new(&args.sources, metadata)?;
    let state = args.continue_.as_ref().map(|filename| {
        let content = fs::read_to_string(filename).expect("Failed to read saved state file");
        serde_json::from_str::<SavedState>(&content).expect("Failed to parse saved state file")
    });
    match_union!(
        autotuner.run(
            &args.direction,
            &args.strategy,
            &args.criterion,
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
        saved_state => {
            let filename = format!(
                "saved_state.{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            fs::write(
                &filename,
                serde_json::to_string(&saved_state).expect("Failed to serialize instances"),
            )
            .expect("Failed to write current state to file");
            println!("Saved current state to {}", filename);
        }
    );

    Ok(())
}
