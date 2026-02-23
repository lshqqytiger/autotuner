pub(crate) mod execution_result;
pub(crate) mod options;
pub(crate) mod output;
pub(crate) mod state;

use crate::{
    parameter::{Profile, Specification, Value},
    strategies::exhaustive::state::State,
};
use std::sync::Arc;

pub(crate) trait Exhaustive {
    fn iter(&self) -> State;
}

impl Exhaustive for Profile {
    fn iter(&self) -> State {
        let names = self.0.keys().cloned().collect::<Vec<Arc<str>>>();
        let specifications = names
            .iter()
            .map(|name| self.0.get(name).unwrap().clone())
            .collect::<Vec<Arc<Specification>>>();
        let values = specifications
            .iter()
            .map(|specification| specification.get_space().first())
            .collect::<Vec<Value>>();
        State {
            names,
            values,
            specifications,
            done: false,
        }
    }
}
