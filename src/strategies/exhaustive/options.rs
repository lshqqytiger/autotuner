use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug, Clone)]
/// exhaustive search options
#[argh(subcommand, name = "exhaustive")]
pub(crate) struct ExhaustiveSearchOptions {}
