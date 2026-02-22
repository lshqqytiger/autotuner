use serde::{Deserialize, Serialize};

fn default_initial() -> usize {
    128
}

fn default_remain() -> usize {
    4
}

fn default_generate() -> usize {
    96
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GeneticSearchOptions {
    #[serde(default = "default_initial")]
    pub(crate) initial: usize,
    #[serde(default = "default_remain")]
    pub(crate) remain: usize,
    #[serde(default = "default_generate")]
    pub(crate) generate: usize,
    pub(crate) terminate: TerminationOptions,
    pub(crate) mutate: MutationOptions,
    #[serde(default)]
    pub(crate) history: Option<String>,
}

fn default_mutation_probability() -> f64 {
    0.1
}

fn default_mutation_variation() -> f64 {
    0.1
}

#[derive(Serialize, Deserialize)]
pub(crate) struct MutationOptions {
    #[serde(default = "default_mutation_probability")]
    pub(crate) probability: f64,
    #[serde(default = "default_mutation_variation")]
    pub(crate) variation: f64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TerminationOptions {
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    #[serde(default)]
    pub(crate) endure: Option<usize>,
}
