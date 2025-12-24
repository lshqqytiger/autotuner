use autotuner::parameter::{Instance, Profile, Specification, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct Iter {
    names: Vec<Arc<str>>,
    values: Vec<Value>,
    specifications: Vec<Arc<Specification>>,
    done: bool,
}

impl From<&Profile> for Iter {
    fn from(profile: &Profile) -> Self {
        let names = profile.0.keys().cloned().collect::<Vec<Arc<str>>>();
        let specifications = names
            .iter()
            .map(|name| profile.0.get(name).unwrap().clone())
            .collect::<Vec<Arc<Specification>>>();
        let values = specifications
            .iter()
            .map(|specification| specification.default())
            .collect::<Vec<Value>>();
        Iter {
            names,
            values,
            specifications,
            done: false,
        }
    }
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
            if let Some(next_value) = self.specifications[index].next(&self.values[index]) {
                self.values[index] = next_value;
                for reset_index in index + 1..self.values.len() {
                    self.values[reset_index] = self.specifications[reset_index].default();
                }
                return Some(instance);
            }
        }

        self.done = true;
        Some(instance)
    }
}
