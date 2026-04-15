use crate::{
    individual::Individual,
    parameter::Profile,
    strategies::genetic::options::{Hyperparameters, Options},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) generation: usize,
    pub(crate) count: usize,
    pub(crate) hyperparameters: Hyperparameters,
    pub(crate) population: Vec<Arc<Individual>>,
}

impl State {
    pub(crate) fn new(options: &Options, profile: &Profile) -> Self {
        let hyperparameters = options.hyperparameters.clone();
        let population = (0..options.hyperparameters.initial_population).into_par_iter();
        let population = if let Some(ref initial) = options.hyperparameters.initial {
            let individual = profile.string_to_individual(initial);
            population
                .map(|_| Arc::new(individual.clone()))
                .collect::<Vec<_>>()
        } else {
            population
                .map(|_| Arc::new(Individual::random(profile)))
                .collect::<Vec<_>>()
        };

        State {
            generation: 1,
            count: 0,
            hyperparameters,
            population,
        }
    }

    pub(crate) fn step(&mut self) {
        self.hyperparameters.step();
    }
}
