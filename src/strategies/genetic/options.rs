use crate::strategies::options::{self, Step};
use serde::{Deserialize, Serialize};

fn default_initial() -> usize {
    128
}

fn default_remain() -> usize {
    0
}

fn default_generate() -> options::Usize {
    64.into()
}

fn default_delete() -> options::Usize {
    64.into()
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Options {
    #[serde(default = "default_initial")]
    pub(crate) initial: usize,
    #[serde(default = "default_remain")]
    pub(crate) remain: usize,
    #[serde(default = "default_generate")]
    pub(crate) generate: options::Usize,
    #[serde(default = "default_delete")]
    pub(crate) delete: options::Usize,
    pub(crate) terminate: Termination,
    pub(crate) mutate: Mutation,
    #[serde(default)]
    pub(crate) history: Option<String>,
}

impl Step for Options {
    fn step(&mut self) {
        self.mutate.step();
    }
}

fn default_mutation_probability() -> options::Real {
    0.1.into()
}

fn default_mutation_variation() -> options::Real {
    0.1.into()
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Mutation {
    #[serde(default = "default_mutation_probability")]
    pub(crate) probability: options::Real,
    #[serde(default = "default_mutation_variation")]
    pub(crate) variation: options::Real,
}

impl Step for Mutation {
    fn step(&mut self) {
        self.probability.step();
        self.variation.step();
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Termination {
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    #[serde(default)]
    pub(crate) endure: Option<usize>,
}
