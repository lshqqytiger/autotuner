use anyhow::anyhow;
use argh::{FromArgValue, FromArgs};
use autotuner::{metadata::Metadata, parameter::Instance};
use hashlru::Cache;
use lazy_static::lazy_static;
use libloading::{Library, Symbol};
use libnuma::{
    masks::indices::NodeIndex,
    memories::{Memory, NumaMemory},
};
use rand::seq::SliceRandom;
use std::{
    ffi, fs,
    process::Command,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

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

    #[argh(option, short = 'n', default = "24")]
    /// number of instances that will be made at each generation (default: 24)
    ngeneration: usize,

    #[argh(option, short = 'l', default = "64")]
    /// maximum number of generations (default: 64)
    limit: usize,

    #[argh(option, default = "4096")]
    /// cache size in number of entries (default: 4096)
    cache_size: usize,
}

type Initializer =
    unsafe extern "C" fn(args_in: *const *mut ffi::c_void, arg_val: *mut ffi::c_void);
type Evaluator =
    unsafe extern "C" fn(args_in: *const *mut ffi::c_void, arg_out: *mut ffi::c_void) -> f64;
type Validator =
    unsafe extern "C" fn(arg_val: *const ffi::c_void, arg_out: *const ffi::c_void) -> bool;

fn compile(
    sources: &[String],
    metadata: &Metadata,
    instance: Option<&Instance>,
) -> anyhow::Result<Library> {
    let mut compiler = Command::new(&metadata.compiler);
    let compiler = compiler
        .arg("-shared")
        .arg("-o")
        .arg("./.temp")
        .args(sources)
        .args(&metadata.compiler_arguments);
    if let Some(instance) = instance {
        compiler.args(instance.compiler_arguments());
    }
    let mut compiler = compiler.spawn()?;
    compiler.wait()?;

    let lib = unsafe { Library::new("./.temp") }?;
    Ok(lib)
}

fn initialize(
    sources: &[String],
    metadata: &Metadata,
    input_blocks: &[NumaMemory],
    validation_block: Option<&NumaMemory>,
) -> anyhow::Result<()> {
    let lib = compile(sources, metadata, None)?;

    let mut input_addresses = Vec::with_capacity(input_blocks.len());
    for block in input_blocks {
        input_addresses.push(block.address());
    }

    let initializer: Symbol<Initializer> = unsafe { lib.get(metadata.initializer.as_bytes()) }?;
    unsafe {
        initializer(
            input_addresses.as_ptr(),
            if let Some(block) = validation_block {
                block.address()
            } else {
                ptr::null_mut()
            },
        );
    }
    Ok(())
}

lazy_static! {
    static ref CRASHED: AtomicBool = AtomicBool::new(false);
}

extern "C" fn sigsegv(_: libc::c_int) {
    CRASHED.store(true, Ordering::Relaxed);
}

fn evaluate(
    sources: &[String],
    metadata: &Metadata,
    instance: &Instance,
    input_blocks: &[NumaMemory],
    output_block: &NumaMemory,
    validation_block: Option<&NumaMemory>,
) -> anyhow::Result<f64> {
    let lib = compile(sources, metadata, Some(instance))?;

    let mut input_addresses = Vec::with_capacity(input_blocks.len());
    for block in input_blocks {
        input_addresses.push(block.address());
    }

    let evaluator: Symbol<Evaluator> = unsafe { lib.get(metadata.evaluator.as_bytes()) }?;
    let fitness = unsafe {
        libc::signal(libc::SIGSEGV, sigsegv as _);
        let fitness = evaluator(input_addresses.as_ptr(), output_block.address());
        libc::signal(libc::SIGSEGV, libc::SIG_DFL);
        fitness
    };

    if CRASHED.load(Ordering::Relaxed) {
        CRASHED.store(false, Ordering::Relaxed);
        return Ok(f64::INFINITY);
    }

    if let Some(block) = validation_block {
        let validator: Symbol<Validator> =
            unsafe { lib.get(metadata.validator.as_ref().unwrap().as_bytes()) }?;
        if !unsafe { validator(block.address(), output_block.address()) } {
            return Ok(f64::INFINITY);
        }
    }

    Ok(fitness)
}

fn stochastic_universal_sampling(roulette: &[(f64, usize)], n: usize) -> Vec<usize> {
    let start = rand::random_range(0.0..1.0);
    let pointers: Vec<f64> = (0..n)
        .map(|i| start + (i as f64) * (1.0 / n as f64))
        .collect();

    let mut selected = Vec::with_capacity(n);
    let mut i = 0;
    let mut sum_fitness = 0.0;

    for (fitness, index) in roulette {
        sum_fitness += fitness;

        while i < n && sum_fitness >= pointers[i] {
            selected.push(*index);
            i += 1;
        }

        if i == n {
            break;
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

    let metadata = serde_json::from_str::<Metadata>(
        &fs::read_to_string(args.metadata).expect("Failed to read kernel metadata file"),
    )
    .expect("Failed to parse kernel metadata");

    let mut cache = Cache::new(args.cache_size);

    let mut instances = Vec::new();
    for _ in 0..args.initial {
        let instance = metadata.profile.random();
        instances.push(Box::new(instance));
    }

    let numa_node = metadata.numa_node.map(NodeIndex::new);
    let mut input_blocks = Vec::with_capacity(metadata.input_blocks.len());
    for block in &metadata.input_blocks {
        let memory = if let Some(node) = numa_node {
            NumaMemory::allocate_on_node(*block, node)
        } else {
            NumaMemory::allocate_local(*block)
        };
        input_blocks.push(memory);
    }
    let output_block = if let Some(node) = numa_node {
        NumaMemory::allocate_on_node(metadata.output_block, node)
    } else {
        NumaMemory::allocate_local(metadata.output_block)
    };
    let validation_block = metadata.validator.as_ref().map(|_| {
        if let Some(node) = numa_node {
            NumaMemory::allocate_on_node(metadata.output_block, node)
        } else {
            NumaMemory::allocate_local(metadata.output_block)
        }
    });

    initialize(
        &args.sources,
        &metadata,
        &input_blocks,
        validation_block.as_ref(),
    )?;

    let mut rng = rand::rng();
    for i in 0..args.limit {
        println!("#{}", i + 1);
        let mut fitnesses = Vec::new();
        for instance in &instances {
            let value = if let Some(&value) = cache.get(&instance.get_identifier()) {
                value
            } else {
                println!("Running kernel {}", instance.get_identifier());
                let value = evaluate(
                    &args.sources,
                    &metadata,
                    instance,
                    &input_blocks,
                    &output_block,
                    validation_block.as_ref(),
                )?;
                cache.insert(instance.get_identifier(), value);
                value
            };
            if value.is_nan() {
                return Err(anyhow!("NaN value encountered"));
            }
            if value.is_infinite() {
                continue;
            }
            fitnesses.push(value);
        }

        let min = fitnesses.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = fitnesses.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        println!("min = {}, max = {}", min, max);

        let mut scaled_fitnesses = Vec::new();
        for (index, fitness) in fitnesses.iter().enumerate() {
            scaled_fitnesses.push((
                match args.direction {
                    Direction::Minimize => (fitness - min) / (max - min),
                    Direction::Maximize => (max - fitness) / (max - min),
                },
                index,
            ));
        }
        scaled_fitnesses.shuffle(&mut rng);
        let holes = stochastic_universal_sampling(&scaled_fitnesses, args.ngeneration);

        for tuple in scaled_fitnesses.iter_mut() {
            tuple.0 = 1.0 - tuple.0;
        }
        scaled_fitnesses.shuffle(&mut rng);

        let mut children = Vec::with_capacity(args.ngeneration);
        for _ in 0..args.ngeneration {
            let result = stochastic_universal_sampling(&scaled_fitnesses, 2);
            let mut child = Instance::crossover(&instances[result[0]], &instances[result[1]]);
            child.mutate();
            metadata.profile.sanitize(&mut child);
            children.push(child);
        }

        for (index, instance) in children.into_iter().enumerate() {
            *instances[holes[index]] = instance;
        }

        for _ in 0..(args.initial - instances.len()) {
            let instance = metadata.profile.random();
            instances.push(Box::new(instance));
        }
    }

    drop(input_blocks);
    drop(output_block);
    drop(validation_block);

    let mut results = Vec::new();
    for v in cache {
        results.push(v);
    }
    results.sort_by(|a, b| a.1.total_cmp(&b.1));

    fs::write(
        "results_cache.json",
        serde_json::to_string_pretty(&results).expect("Failed to serialize results"),
    )
    .expect("Failed to write results to file");

    Ok(())
}
