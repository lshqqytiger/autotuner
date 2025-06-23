use crate::parameter::Profile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub profile: Profile,
    pub input_blocks: Vec<usize>,
    pub output_block: usize,
    pub numa_node: Option<u8>,
    pub initializer: String,
    pub evaluator: String,
    pub validator: Option<String>,
    pub compiler: String,
    pub compiler_arguments: Vec<String>,
}
