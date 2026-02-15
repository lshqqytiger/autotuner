use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug, Clone)]
/// genetic search options
#[argh(subcommand, name = "genetic")]
pub(crate) struct GeneticSearchOptions {
    #[argh(option, short = 'i', default = "128")]
    /// initial population size (default: 128)
    pub(crate) initial: usize,

    #[argh(option, short = 'r', default = "4")]
    /// number of instances that will remain at each generation (default: 4)
    pub(crate) remain: usize,

    #[argh(option, short = 'g', default = "96")]
    /// number of instances that will be made at each generation (default: 96)
    pub(crate) generate: usize,

    #[argh(option, short = 'l', default = "128")]
    /// maximum number of generations (default: 128)
    pub(crate) limit: usize,

    #[argh(option)]
    /// output file
    pub(crate) history: Option<String>,
}
