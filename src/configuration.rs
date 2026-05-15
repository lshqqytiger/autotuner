use crate::{
    criterion::Criterion, direction::Direction, helper, hook, parameter::Profile,
    strategies::Strategy,
};
use serde::Deserialize;

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

#[derive(Deserialize)]
pub(crate) struct Configuration {
    #[serde(default)]
    pub(crate) unit: Option<String>,
    pub(crate) direction: Direction,
    pub(crate) criterion: Criterion,
    #[serde(default)]
    pub(crate) stop_action: StopAction,
    pub(crate) strategy: Strategy,
    pub(crate) profile: Profile,
    pub(crate) helper: helper::Configuration,
    pub(crate) runner: String,
    #[serde(default)]
    pub(crate) hooks: hook::Configuration,
    pub(crate) compiler: String,
    #[serde(default)]
    pub(crate) compiler_arguments: Vec<String>,
}
