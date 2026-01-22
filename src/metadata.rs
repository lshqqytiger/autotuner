use crate::parameter::Profile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Metadata {
    pub(crate) profile: Profile,
    pub(crate) initializer: Option<String>,
    pub(crate) finalizer: Option<String>,
    pub(crate) evaluator: String,
    pub(crate) validator: Option<String>,
    pub(crate) hooks: Vec<String>,
    pub(crate) compiler: String,
    pub(crate) compiler_arguments: Vec<String>,
}
