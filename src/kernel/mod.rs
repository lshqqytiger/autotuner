pub mod parameter;
use parameter::Parameter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Parameters(HashMap<String, Parameter>);

pub struct Kernel {
    filename: String,
    parameters: Parameters,
}

impl Kernel {
    pub fn new(filename: String, parameters: Parameters) -> Self {
        Kernel {
            filename,
            parameters,
        }
    }
}
