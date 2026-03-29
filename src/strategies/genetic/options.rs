use crate::strategies::options::{Real, Usize};
use serde::Deserialize;

fn default_initial() -> usize {
    128
}

fn default_remain() -> usize {
    0
}

fn default_generate() -> Usize {
    64.into()
}

fn default_delete() -> Usize {
    64.into()
}

fn default_infuse() -> Usize {
    0.into()
}

#[derive(Deserialize, Clone)]
pub(crate) struct Options {
    #[serde(default = "default_initial")]
    pub(crate) initial: usize,
    #[serde(default = "default_remain")]
    pub(crate) remain: usize,
    #[serde(default = "default_generate")]
    pub(crate) generate: Usize,
    #[serde(default = "default_delete")]
    pub(crate) delete: Usize,
    #[serde(default = "default_infuse")]
    pub(crate) infuse: Usize,
    pub(crate) terminate: Termination,
    #[serde(default)]
    pub(crate) mutate: Mutation,
}

impl Options {
    pub(crate) fn step(&mut self) {
        self.generate.step();
        self.delete.step();
        self.infuse.step();
        self.mutate.step();
    }
}

#[derive(Deserialize, Default, Clone)]
pub(crate) struct Mutation {
    #[serde(default)]
    pub(crate) integer: Vec<IntegerMutation>,
    #[serde(default)]
    pub(crate) switch: Option<SwitchMutation>,
    #[serde(default)]
    pub(crate) keyword: Option<KeywordMutation>,
}

impl Mutation {
    pub(crate) fn step(&mut self) {
        for integer in &mut self.integer {
            integer.probability.step();
            if let Some(variation) = &mut integer.variation {
                variation.step();
            }
        }
        if let Some(switch) = &mut self.switch {
            switch.probability.step();
        }
        if let Some(keyword) = &mut self.keyword {
            keyword.probability.step();
        }
    }
}

fn default_integer_mutation_probability() -> Real {
    0.1.into()
}

#[derive(Deserialize, Clone)]
pub(crate) struct IntegerMutation {
    #[serde(default = "default_integer_mutation_probability")]
    pub(crate) probability: Real,
    #[serde(default)]
    pub(crate) variation: Option<Real>,
}

fn default_switch_mutation_probability() -> Real {
    0.1.into()
}

#[derive(Deserialize, Clone)]
pub(crate) struct SwitchMutation {
    #[serde(default = "default_switch_mutation_probability")]
    pub(crate) probability: Real,
}

fn default_keyword_mutation_probability() -> Real {
    0.1.into()
}

#[derive(Deserialize, Clone)]
pub(crate) struct KeywordMutation {
    #[serde(default = "default_keyword_mutation_probability")]
    pub(crate) probability: Real,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Termination {
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    #[serde(default)]
    pub(crate) endure: Option<usize>,
}
