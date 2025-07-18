use anyhow::anyhow;
use argh::{FromArgValue, FromArgs};
use autotuner::{metadata::Metadata, parameter::Instance};
use hashlru::Cache;
use libloading::{Library, Symbol};
use rand::seq::SliceRandom;
use rayon::{ThreadPoolBuilder, prelude::*};
use signal_hook_registry::{register_unchecked, unregister};
use std::{ffi, fs, process::Command, ptr, sync::Arc, thread};
use tempdir::TempDir;

const SIGSEGV: i32 = 11;

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
}

type Initializer = unsafe extern "C" fn(
    arg_in: *mut *mut ffi::c_void,
    arg_out: *mut *mut ffi::c_void,
    arg_val: *mut *mut ffi::c_void,
);
type Finalizer = unsafe extern "C" fn(
    arg_in: *mut ffi::c_void,
    arg_out: *mut ffi::c_void,
    arg_val: *mut ffi::c_void,
);
type Evaluator = unsafe extern "C" fn(arg_in: *mut ffi::c_void, arg_out: *mut ffi::c_void) -> f64;
type Validator =
    unsafe extern "C" fn(arg_val: *const ffi::c_void, arg_out: *const ffi::c_void) -> bool;

unsafe impl Sync for Workspace {}

// TODO: input_ptr and validation_ptr can be shared between threads
struct Workspace {
    input_ptr: *mut ffi::c_void, // const after initialization
    output_ptr: *mut ffi::c_void,
    validation_ptr: Option<*mut ffi::c_void>, // const after initialization
}

impl Workspace {
    fn new(validation: bool) -> Self {
        Workspace {
            input_ptr: ptr::null_mut(),
            output_ptr: ptr::null_mut(),
            validation_ptr: if validation {
                Some(ptr::null_mut())
            } else {
                None
            },
        }
    }
}

#[inline]
fn round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

fn compile(
    temp_dir: &TempDir,
    sources: &[String],
    metadata: &Metadata,
    instance: Option<&Instance>,
) -> anyhow::Result<Library> {
    let path = temp_dir
        .path()
        .join(thread::current().name().unwrap_or("temp"));
    let mut compiler = Command::new(&metadata.compiler);
    let compiler = compiler
        .arg("-shared")
        .arg("-o")
        .arg(&path)
        .args(sources)
        .args(&metadata.compiler_arguments);
    if let Some(instance) = instance {
        compiler.args(instance.compiler_arguments());
    }
    let mut compiler = compiler.spawn()?;
    compiler.wait()?;

    let lib = unsafe { Library::new(&path) }?;
    Ok(lib)
}

fn initialize(
    temp_dir: &TempDir,
    sources: &[String],
    metadata: &Metadata,
    workspace: &mut Workspace,
) -> anyhow::Result<()> {
    let lib = compile(temp_dir, sources, metadata, None)?;
    let initializer: Symbol<Initializer> = unsafe { lib.get(metadata.initializer.as_bytes()) }?;
    unsafe {
        initializer(
            &mut workspace.input_ptr,
            &mut workspace.output_ptr,
            if let Some(ptr) = workspace.validation_ptr.as_mut() {
                ptr
            } else {
                ptr::null_mut()
            },
        );
    }
    Ok(())
}

fn finalize(
    temp_dir: &TempDir,
    sources: &[String],
    metadata: &Metadata,
    workspace: &Workspace,
) -> anyhow::Result<()> {
    if let None = metadata.finalizer {
        return Ok(());
    }

    let lib = compile(temp_dir, sources, metadata, None)?;
    let finalizer: Symbol<Finalizer> =
        unsafe { lib.get(metadata.finalizer.as_ref().unwrap().as_bytes()) }?;
    unsafe {
        finalizer(
            workspace.input_ptr,
            workspace.output_ptr,
            if let Some(ptr) = workspace.validation_ptr {
                ptr
            } else {
                ptr::null_mut()
            },
        );
    }
    Ok(())
}

fn evaluate(
    temp_dir: &TempDir,
    sources: &[String],
    metadata: &Metadata,
    instance: &Instance,
    repetition: usize,
    workspace: &Workspace,
) -> anyhow::Result<f64> {
    let lib = compile(temp_dir, sources, metadata, Some(instance))?;
    let evaluator: Symbol<Evaluator> = unsafe { lib.get(metadata.evaluator.as_bytes()) }?;
    let mut fitnesses = Vec::with_capacity(repetition);
    for _ in 0..repetition {
        let fitness = unsafe {
            let result = register_unchecked(SIGSEGV, |_| {
                // can we do better than this?
                println!("Segmentation fault occurred during evaluation");
                std::process::exit(1);
            });
            let fitness = evaluator(workspace.input_ptr, workspace.output_ptr);
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
    fitnesses.sort_by(|a, b| a.total_cmp(b));

    if let Some(block) = workspace.validation_ptr {
        let validator: Symbol<Validator> =
            unsafe { lib.get(metadata.validator.as_ref().unwrap().as_bytes()) }?;
        if !unsafe { validator(block, workspace.output_ptr) } {
            return Ok(f64::INFINITY);
        }
    }

    Ok(fitnesses[fitnesses.len() / 2])
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
        .build_global()?;

    let cores = core_affinity::get_core_ids().unwrap();
    let num_cores = cores.len();

    let metadata = serde_json::from_str::<Metadata>(
        &fs::read_to_string(args.metadata).expect("Failed to read kernel metadata file"),
    )
    .expect("Failed to parse kernel metadata");

    let mut cache = Cache::new(args.cache_size);

    let mut instances = Vec::new();
    for _ in 0..args.initial {
        let instance = metadata.profile.random();
        instances.push(Arc::new(instance));
    }

    let temp_dir = TempDir::new("autotuner")?;

    let mut workspaces = Vec::with_capacity(args.parallelism);
    for _ in 0..args.parallelism {
        let mut workspace = Workspace::new(metadata.validator.is_some());
        initialize(&temp_dir, &args.sources, &metadata, &mut workspace)?;
        workspaces.push(workspace);
    }

    let mut results: Vec<(f64, usize)> = Vec::with_capacity(args.initial);
    let mut rng = rand::rng();
    for i in 0..args.limit {
        if !results.is_empty() {
            let min = results
                .iter()
                .filter(|(x, _)| !x.is_infinite())
                .fold(f64::INFINITY, |a, &b| a.min(b.0));
            let max = results
                .iter()
                .filter(|(x, _)| !x.is_infinite())
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b.0));
            println!("min = {}, max = {}", min, max);

            let mut inversed = results.clone();
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

            for result in &mut results {
                if result.0.is_infinite() {
                    result.0 = min;
                    continue;
                }

                result.0 = match args.direction {
                    Direction::Minimize => max - result.0,
                    Direction::Maximize => result.0,
                };
            }
            results.shuffle(&mut rng);

            let mut children = Vec::with_capacity(args.ngeneration);
            for _ in 0..args.ngeneration {
                let result = stochastic_universal_sampling(&results, 2);
                let child = Instance::crossover(&instances[result[0]], &instances[result[1]]);
                let child = child.mutate();
                children.push(child);
            }

            for (index, instance) in children.into_iter().enumerate() {
                instances[holes[index]] = Arc::new(instance);
            }

            results.clear();
        }

        println!("#{}", i + 1);

        let len = instances.len();
        let mut fresh_instances = Vec::new();
        for index in 0..len {
            if let Some(&fitness) = cache.get(&instances[index]) {
                results.push((fitness, index));
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

            let chunk: Vec<anyhow::Result<(f64, usize)>> = fresh_instances
                .par_iter()
                .map(|(i, instance)| {
                    let tid = rayon::current_thread_index().unwrap_or(0);
                    let index = tid
                        + args.parallelism * rand::random_range(0..(num_cores / args.parallelism));
                    core_affinity::set_for_current(cores[index]);

                    let instance = instance.clone();
                    let workspace = &workspaces[tid];
                    let value = evaluate(
                        &temp_dir,
                        &args.sources,
                        &metadata,
                        &instance,
                        args.repetition,
                        workspace,
                    )?;
                    Ok((value, *i))
                })
                .collect();

            for result in chunk {
                let result = result?;
                cache.insert(instances[result.1].clone(), result.0);
                results.push(result);
            }
        }
    }

    for workspace in workspaces {
        finalize(&temp_dir, &args.sources, &metadata, &workspace)?;
    }

    temp_dir.close()?;

    results.sort_by(|a, b| a.0.total_cmp(&b.0));
    let results = results
        .into_iter()
        .map(|x| (format!("{}", instances[x.1]), x.0))
        .collect::<Vec<_>>();
    drop(instances);

    fs::write(
        "results.json",
        serde_json::to_string_pretty(&results).expect("Failed to serialize instances"),
    )
    .expect("Failed to write results to file");

    let mut results = Vec::new();
    for (instance, fitness) in cache {
        results.push((format!("{}", instance), fitness));
    }
    results.sort_by(|a, b| a.1.total_cmp(&b.1));

    fs::write(
        "results_cache.json",
        serde_json::to_string_pretty(&results).expect("Failed to serialize results"),
    )
    .expect("Failed to write results to file");

    Ok(())
}
