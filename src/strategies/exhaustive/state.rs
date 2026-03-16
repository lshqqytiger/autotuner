use crate::parameter::{Individual, Profile, Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct State(pub(crate) Option<BTreeMap<Arc<str>, Value>>);

impl State {
    pub(crate) fn new(profile: &Profile) -> Self {
        State(Some(
            profile
                .0
                .iter()
                .map(|(name, specification)| (name.clone(), specification.get_space().first()))
                .collect(),
        ))
    }

    pub(crate) fn next(&mut self, profile: &Profile) -> Option<Individual> {
        if let Some(current) = &self.0 {
            let individual = Individual::new(current.clone());
            let mut next = current.clone();

            for (name, specification) in profile.0.iter().rev() {
                if let Some(&current_value) = next.get(name) {
                    if let Some(next_value) = specification.get_space().next(current_value) {
                        next.insert(name.clone(), next_value);
                        self.0 = Some(next);
                        return Some(individual);
                    }
                    next.insert(name.clone(), specification.get_space().first());
                }
            }

            self.0 = None;
            Some(individual)
        } else {
            None
        }
    }
}
