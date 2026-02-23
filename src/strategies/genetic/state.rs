use crate::parameter::{Individual, Profile, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) generation: usize,
    pub(crate) count: usize,
    pub(crate) population: Vec<Rc<Individual>>,
}

impl State {
    pub(crate) fn sample(profile: &Profile) -> Rc<Individual> {
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
            population: individuals,
        }
    }
}
