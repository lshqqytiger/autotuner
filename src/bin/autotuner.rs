use argh::FromArgs;
use autotuner::kernel::{Kernel, Parameters};
use std::fs;

#[derive(FromArgs)]
/// CLI Arguments
struct Arguments {
    #[argh(positional)]
    filename_source: String,
    #[argh(positional)]
    filename_parameters: String,
}

fn main() -> Result<(), ()> {
    let args: Arguments = argh::from_env();

    let parameters = serde_json::from_str::<Parameters>(
        &fs::read_to_string(args.filename_parameters).expect("Failed to read kernel metadata file"),
    )
    .expect("Failed to parse kernel metadata");
    let kernel = Kernel::new(args.filename_source, parameters);

    Ok(())
}
