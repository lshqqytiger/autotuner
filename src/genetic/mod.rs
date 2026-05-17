pub(crate) mod output;
pub(crate) mod state;

use crate::configuration::Mutation;
use crate::individual::Individual;
use crate::parameter::Profile;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::time::SystemTime;

#[derive(Serialize)]
pub(crate) struct GenerationSummary {
    pub(crate) timestamp: u64,
    pub(crate) global_best: Individual,
    pub(crate) current_best: f64,
    pub(crate) current_worst: f64,
}

impl GenerationSummary {
    pub(crate) fn print(&self, unit: &Option<String>) {
        let unit = unit.as_deref().unwrap_or("");
        println!("Best overall: {} {}", self.global_best.fitness, unit);
        println!("Best: {} {}", self.current_best, unit);
        println!("Worst: {} {}", self.current_worst, unit);
    }
}

impl GenerationSummary {
    pub(crate) fn new(global_best: &Individual, (current_best, current_worst): (f64, f64)) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let global_best = global_best.clone();
        GenerationSummary {
            timestamp,
            global_best,
            current_best,
            current_worst,
        }
    }
}

pub(crate) fn crossover(profile: &Profile, a: &Individual, b: &Individual) -> Individual {
    let parameters = a
        .parameters
        .par_iter()
        .fold(
            || BTreeMap::new(),
            |mut parameters, parameter| {
                let specification = profile.0.get(parameter.0).unwrap();
                let space = specification.get_space();
                let value = space.crossover(a.parameters[parameter.0], b.parameters[parameter.0]);
                parameters.insert(parameter.0.clone(), value);
                parameters
            },
        )
        .reduce(
            || BTreeMap::new(),
            |mut acc, parameters| {
                acc.extend(parameters);
                acc
            },
        );
    Individual::new(parameters)
}

pub(crate) fn mutate(profile: &Profile, options: &Mutation, individual: &mut Individual) {
    individual
        .parameters
        .par_iter_mut()
        .for_each(|(name, parameter)| {
            let specification = profile.0.get(name).unwrap();
            let space = specification.get_space();
            space.mutate(options, parameter);
        });
    profile.adjust(individual);
}

pub(crate) fn stochastic_universal_sampling(roulette: &[(f64, usize)], n: usize) -> Vec<usize> {
    assert!(!roulette.is_empty());
    assert_ne!(n, 0);
    assert!(n <= roulette.len());

    let total_fitness: f64 = roulette.iter().map(|(fitness, _)| fitness).sum();

    let distance = total_fitness / n as f64;
    let start = rand::random::<f64>() * distance;

    let mut cumulative = Vec::with_capacity(roulette.len());
    let mut acc = 0.0;
    for (fitness, _) in roulette {
        acc += *fitness;
        cumulative.push(acc);
    }

    let mut selected_positions = HashSet::with_capacity(n);
    let mut selected = Vec::with_capacity(n);

    for i in 0..n {
        let pointer = start + i as f64 * distance;

        let mut position = cumulative.partition_point(|sum| *sum < pointer);
        if position >= roulette.len() {
            position = roulette.len() - 1;
        }

        while !selected_positions.insert(position) {
            position = (position + 1) % roulette.len();
        }

        selected.push(roulette[position].1);
    }

    selected
}
