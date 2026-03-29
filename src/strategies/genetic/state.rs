use crate::{individual::Individual, parameter::Profile};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) generation: usize,
    pub(crate) count: usize,
    pub(crate) population: Vec<Arc<Individual>>,
}

impl State {
    pub(crate) fn new(profile: &Profile, initial: usize) -> Self {
        let population = (0..initial)
            .into_par_iter()
            .map(|_| Arc::new(Individual::random(profile)))
            .collect::<Vec<_>>();
        State {
            generation: 1,
            count: 0,
            population,
        }
    }
}
