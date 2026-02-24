use crate::{
    criterion::Criterion, direction::Direction, helper, hook, parameter::Profile,
    strategies::Strategy,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Configuration {
    #[serde(default)]
    pub(crate) unit: Option<String>,
    pub(crate) direction: Direction,
    pub(crate) criterion: Criterion,
    pub(crate) strategy: Strategy,
    pub(crate) profile: Profile,
    pub(crate) helper: helper::Configuration,
    pub(crate) runner: String,
    #[serde(default)]
    pub(crate) hooks: hook::Configuration,
    pub(crate) compiler: String,
    pub(crate) compiler_arguments: Vec<String>,
}
