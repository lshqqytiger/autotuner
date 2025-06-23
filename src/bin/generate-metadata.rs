use autotuner::{
    interner::Interner,
    metadata::Metadata,
    parameter::{Parameter, Profile, Range},
};
use fxhash::FxHashMap;
use inquire::{Confirm, CustomType, Select, Text, validator::Validation};
use std::fs;

#[cfg(target_arch = "aarch64")]
const DEFAULT_COMPILER: &str = "armclang";
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const DEFAULT_COMPILER: &str = "icc";

fn main() -> anyhow::Result<()> {
    let mut profile = FxHashMap::default();
    while let Some(name) = Text::new("Parameter name")
        .with_help_message("Enter the name of the parameter (e.g., A, B, C)")
        .prompt_skippable()?
    {
        if name.is_empty() {
            break;
        }

        let typ = Select::new("Parameter type", Parameter::TYPES.to_vec()).prompt()?;
        let parameter = match typ {
            "Integer" => {
                let is_even = Confirm::new(&format!("Should {} be an even number?", name))
                    .with_default(false)
                    .prompt()?;
                let range = match Select::new("Range Type", vec!["Sequence"]).prompt()? {
                    "Sequence" => {
                        let start: i32 = CustomType::<i32>::new("- Minimum").prompt()?;
                        let end: i32 = CustomType::<i32>::new("- Maximum")
                            .with_validator(move |x: &i32| {
                                if *x > start {
                                    Ok(Validation::Valid)
                                } else {
                                    Ok(Validation::Invalid(
                                        "Maximum must be greater than minimum".into(),
                                    ))
                                }
                            })
                            .prompt()?;
                        Range::Sequence(start, end)
                    }
                    _ => unreachable!(),
                };
                let condition = Text::new("Additional conditions")
                    .with_help_message("Enter the boolean expression (e.g., A + B < 256)")
                    .prompt_skippable()?;

                Parameter::Integer {
                    is_even,
                    range,
                    condition,
                }
            }
            "Switch" => Parameter::Switch,
            _ => unreachable!(),
        };
        profile.insert(Interner::intern(&name), parameter);
    }

    let profile = Profile(profile);

    let num_input_blocks = CustomType::<usize>::new("Number of input blocks")
        .with_help_message("Enter the number of input blocks of the kernel")
        .with_default(2)
        .prompt()?;
    let mut input_blocks = Vec::with_capacity(num_input_blocks);
    for i in 0..num_input_blocks {
        let block_size = CustomType::<usize>::new(&format!("Size of input block #{}", i + 1))
            .with_help_message("Enter the size of the input block in bytes")
            .prompt()?;
        input_blocks.push(block_size);
    }

    let output_block = CustomType::<usize>::new("Size of output block")
        .with_help_message("Enter the size of the output block in bytes")
        .prompt()?;

    let numa_node = CustomType::<u8>::new("NUMA node")
        .with_help_message("Enter the NUMA node to allocate blocks on")
        .prompt_skippable()?;

    let initializer = Text::new("Initializer")
        .with_help_message("Enter the name of the initializer function")
        .prompt()?;

    let evaluator = Text::new("Evaluator")
        .with_help_message("Enter the name of the evaluator function")
        .prompt()?;

    let validator = Text::new("Validator")
        .with_help_message("Enter the name of the validator function (optional)")
        .prompt_skippable()?;

    let compiler = Text::new("Compiler")
        .with_help_message("Enter the compiler to use (e.g., icc, armclang)")
        .with_default(DEFAULT_COMPILER)
        .prompt()?;

    let compiler_arguments = Text::new("Compiler arguments")
        .with_help_message("Enter the compiler arguments (e.g., -O3, -Iinclude)")
        .prompt_skippable()?;
    let compiler_arguments = if let Some(arguments) = compiler_arguments {
        arguments.split_whitespace().map(String::from).collect()
    } else {
        Vec::new()
    };

    let metadata = Metadata {
        profile,
        input_blocks,
        output_block,
        numa_node,
        initializer,
        evaluator,
        validator,
        compiler,
        compiler_arguments,
    };

    let filename = Text::new("Save as")
        .with_help_message("Enter the filename (e.g., kernel.meta)")
        .with_default("kernel.meta")
        .prompt()?;
    let json = serde_json::to_string_pretty(&metadata)?;
    fs::write(&filename, json)?;

    Ok(())
}
