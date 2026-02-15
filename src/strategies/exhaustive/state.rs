use crate::parameter::{Instance, Specification, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) names: Vec<Arc<str>>,
    pub(crate) values: Vec<Value>,
    pub(crate) specifications: Vec<Rc<Specification>>,
    pub(crate) done: bool,
}

impl Iterator for State {
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
                    self.values[reset_index] = self.specifications[reset_index].get_space().first();
                }
                return Some(instance);
            }
        }

        self.done = true;
        Some(instance)
    }
}
