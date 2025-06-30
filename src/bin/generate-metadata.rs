use autotuner::{
    interner::Interner,
    metadata::Metadata,
    parameter::{Mapping, Parameter, Profile, Range},
};
use fxhash::FxHashMap;
use inquire::{CustomType, Select, Text, validator::Validation};
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
                let mapping = Mapping::from(
                    Text::new("How to represent this parameter")
                        .with_help_message("Enter a formulaic form of the parameter (optional)")
                        .prompt_skippable()?,
                );

                Parameter::Integer { mapping, range }
            }
            "Switch" => Parameter::Switch,
            _ => unreachable!(),
        };
        profile.insert(Interner::intern(&name), parameter);
    }

    let profile = Profile::new(profile);

    let initializer = Text::new("Initializer")
        .with_help_message("Enter the name of the initializer function")
        .prompt()?;

    let finalizer = Text::new("Finalizer")
        .with_help_message("Enter the name of the finalizer function (optional)")
        .prompt_skippable()?;

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
        initializer,
        finalizer,
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
