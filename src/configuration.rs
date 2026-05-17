use crate::{
    criterion::Criterion,
    direction::Direction,
    ffi::{helper, hook},
    parameter::Profile,
    utils::scaling::{Real, Usize},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, PartialEq, Eq)]
pub(crate) enum StopAction {
    SaveState,
    Terminate,
}

impl Default for StopAction {
    fn default() -> Self {
        Self::Terminate
    }
}

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

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Hyperparameters {
    #[serde(default)]
    pub(crate) initial: Option<String>,
    #[serde(default = "default_initial")]
    pub(crate) initial_population: usize,
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

impl Hyperparameters {
    pub(crate) fn step(&mut self) {
        self.generate.step();
        self.delete.step();
        self.infuse.step();
        self.mutate.step();
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct IntegerMutation {
    pub(crate) probability: Real,
    #[serde(default)]
    pub(crate) variation: Option<Real>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SwitchMutation {
    pub(crate) probability: Real,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct KeywordMutation {
    pub(crate) probability: Real,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Termination {
    #[serde(default)]
    pub(crate) goal: Option<f64>,
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    #[serde(default)]
    pub(crate) endure: Option<usize>,
}

#[derive(Deserialize)]
pub(crate) struct Configuration {
    #[serde(default)]
    pub(crate) unit: Option<String>,
    pub(crate) direction: Direction,
    pub(crate) criterion: Criterion,
    #[serde(default)]
    pub(crate) stop_action: StopAction,
    pub(crate) hyperparameters: Hyperparameters,
    pub(crate) profile: Profile,
    pub(crate) helper: helper::Configuration,
    pub(crate) runner: String,
    #[serde(default)]
    pub(crate) hooks: hook::Configuration,
    pub(crate) compiler: String,
    #[serde(default)]
    pub(crate) compiler_arguments: Vec<String>,
}
