use crate::{criterion::Criterion, direction::Direction, helper, hook, parameter::Profile};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Metadata {
    pub(crate) direction: Direction,
    pub(crate) criterion: Criterion,
    pub(crate) profile: Profile,
    pub(crate) helper: helper::Configuration,
    pub(crate) runner: String,
    pub(crate) hooks: hook::Configuration,
    pub(crate) compiler: String,
    pub(crate) compiler_arguments: Vec<String>,
}
