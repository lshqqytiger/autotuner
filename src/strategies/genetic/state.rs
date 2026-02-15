use crate::parameter::{Instance, Profile, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) generation: usize,
    pub(crate) instances: Vec<Rc<Instance>>,
}

impl State {
    fn sample(profile: &Profile) -> Rc<Instance> {
        Rc::new(Instance::new(
            profile
                .0
                .iter()
                .map(|(name, parameter)| (name.clone(), parameter.get_space().random()))
                .collect::<BTreeMap<Arc<str>, Value>>(),
        ))
    }

    pub(crate) fn new(profile: &Profile, initial: usize) -> Self {
        let mut instances = Vec::with_capacity(initial);
        for _ in 0..initial {
            instances.push(Self::sample(profile));
        }
        State {
            generation: 1,
            instances,
        }
    }

    pub(crate) fn regenerate(&mut self, profile: &Profile, index: usize) {
        self.instances[index] = Self::sample(profile);
    }
}
