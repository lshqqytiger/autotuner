use crate::parameter::{Individual, Profile, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) generation: usize,
    pub(crate) count: usize,
    pub(crate) individuals: Vec<Rc<Individual>>,
}

impl State {
    fn sample(profile: &Profile) -> Rc<Individual> {
        Rc::new(Individual::new(
            profile
                .0
                .iter()
                .map(|(name, parameter)| (name.clone(), parameter.get_space().random()))
                .collect::<BTreeMap<Arc<str>, Value>>(),
        ))
    }

    pub(crate) fn new(profile: &Profile, initial: usize) -> Self {
        let mut individuals = Vec::with_capacity(initial);
        for _ in 0..initial {
            individuals.push(Self::sample(profile));
        }
        State {
            generation: 1,
            count: 0,
            individuals,
        }
    }

    pub(crate) fn regenerate(&mut self, profile: &Profile, index: usize) {
        self.individuals[index] = Self::sample(profile);
    }
}
