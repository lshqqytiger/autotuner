use autotuner::parameter::{Instance, Profile, Specification, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) trait Exhaustive {
    fn iter(&self) -> Iter;
}

impl Exhaustive for Profile {
    fn iter(&self) -> Iter {
        let names = self.0.keys().cloned().collect::<Vec<Arc<str>>>();
        let specifications = names
            .iter()
            .map(|name| self.0.get(name).unwrap().clone())
            .collect::<Vec<Arc<Specification>>>();
        let values = specifications
            .iter()
            .map(|specification| specification.get_space().default())
            .collect::<Vec<Value>>();
        Iter {
            names,
            values,
            specifications,
            done: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Iter {
    names: Vec<Arc<str>>,
    values: Vec<Value>,
    specifications: Vec<Arc<Specification>>,
    done: bool,
}

impl Iterator for Iter {
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
