use crate::parameter::Profile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub profile: Profile,
    pub initializer: String,
    pub finalizer: Option<String>,
    pub evaluator: String,
    pub validator: Option<String>,
    pub compiler: String,
    pub compiler_arguments: Vec<String>,
}
