use autotuner::parameter::{Instance, Profile, Specification, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct SearchState {
    names: Vec<Arc<str>>,
    values: Vec<Value>,
    specifications: Vec<Arc<Specification>>,
    done: bool,
}

impl Iterator for SearchState {
    type Item = Instance;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let parameters = self
            .names
            .iter()
            .cloned()
            .zip(self.values.iter().cloned())
            .collect::<BTreeMap<Arc<str>, Value>>();
        let instance = Instance::new(parameters);

        for index in (0..self.values.len()).rev() {
            if let Some(next_value) = self.specifications[index]
                .get_space()
                .next(&self.values[index])
            {
                self.values[index] = next_value;
                for reset_index in index + 1..self.values.len() {
                    self.values[reset_index] =
                        self.specifications[reset_index].get_space().default();
                }
                return Some(instance);
            }
        }

        self.done = true;
        Some(instance)
    }
}

pub(crate) trait Exhaustive {
    fn iter(&self) -> SearchState;
}

impl Exhaustive for Profile {
    fn iter(&self) -> SearchState {
        let names = self.0.keys().cloned().collect::<Vec<Arc<str>>>();
        let specifications = names
            .iter()
            .map(|name| self.0.get(name).unwrap().clone())
            .collect::<Vec<Arc<Specification>>>();
        let values = specifications
            .iter()
            .map(|specification| specification.get_space().default())
            .collect::<Vec<Value>>();
        SearchState {
            names,
            values,
            specifications,
            done: false,
        }
    }
}
