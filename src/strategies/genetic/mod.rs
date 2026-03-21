pub(crate) mod options;
pub(crate) mod output;
pub(crate) mod state;

use crate::parameter::{
    Individual, IntegerSpace, KeywordSpace, Profile, Space, Specification, SwitchSpace, Value,
};
use crate::strategies::execution_log::ExecutionLog;
use crate::strategies::genetic::options::Mutation;
use rand::seq::IteratorRandom;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::time::SystemTime;

trait Genetic {
    fn get_genetic_space(&self) -> &dyn GeneticSpace;
}

impl Genetic for Specification {
    fn get_genetic_space(&self) -> &dyn GeneticSpace {
        match self {
            Specification::Integer {
                transformer: _,
                space,
            } => space,
            Specification::Switch => &Self::SWITCH_SPACE,
            Specification::Keyword(options) => options,
        }
    }
}

trait GeneticSpace {
    fn crossover(&self, a: Value, b: Value) -> Value;
    fn mutate(&self, mutations: &Mutation, value: &mut Value);
}

impl GeneticSpace for IntegerSpace {
    fn crossover(&self, a: Value, b: Value) -> Value {
        match (self, a, b) {
            (IntegerSpace::Sequence(_, _, step), Value::Integer(a), Value::Integer(b)) => {
                if let Some(step) = step {
                    Value::Integer(((a + b) / 2 / step) * step)
                } else {
                    Value::Integer((a + b) / 2)
                }
            }
            (IntegerSpace::Candidates(_), Value::Index(a), Value::Index(b)) => {
                if a == b {
                    Value::Index(a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, mutations: &Mutation, code: &mut Value) {
        for mutation in &mutations.integer {
            if rand::random_bool(mutation.probability.value) {
                if let Some(variation) = &mutation.variation {
                    match (self, code) {
                        (IntegerSpace::Sequence(start, end, step), Value::Integer(n)) => {
                            let mut variation = (end - start) as f64 * variation.value;
                            if let Some(step) = step {
                                variation = (variation / *step as f64).round() * *step as f64;
                            }
                            let variation = if (variation as i32) == 0 {
                                1
                            } else {
                                variation as i32
                            };

                            let step = step.unwrap_or(1);
                            let iter = (-variation..=variation).step_by(step as usize).into_iter();
                            *n += iter.choose(&mut rand::rng()).unwrap();

                            if *n < *start {
                                *n = *start;
                            } else if *n > *end {
                                *n = *end;
                            }
                        }
                        (IntegerSpace::Candidates(candidates), Value::Index(i)) => {
                            *i = rand::random_range(0..candidates.len());
                        }
                        _ => unreachable!(),
                    }
                } else {
                    *code = self.random();
                }
                return;
            }
        }
    }
}

impl GeneticSpace for SwitchSpace {
    fn crossover(&self, a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Switch(a), Value::Switch(b)) => {
                if a == b {
                    Value::Switch(a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, mutations: &Mutation, code: &mut Value) {
        if let Some(options) = &mutations.switch {
            if !rand::random_bool(options.probability.value) {
                return;
            }

            if let Value::Switch(b) = code {
                *b = !*b;
            }
        }
    }
}

impl GeneticSpace for KeywordSpace {
    fn crossover(&self, a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Index(a), Value::Index(b)) => {
                if a == b {
                    Value::Index(a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, mutations: &Mutation, code: &mut Value) {
        if let Some(options) = &mutations.keyword {
            if rand::random_bool(options.probability.value) {
                *code = self.random();
            }
        }
    }
}

#[derive(Serialize)]
pub(crate) struct GenerationSummary {
    pub(crate) timestamp: u64,
    pub(crate) global_best: ExecutionLog,
    pub(crate) current_best: f64,
    pub(crate) current_worst: f64,
}

impl GenerationSummary {
    pub(crate) fn print(&self, unit: &Option<String>) {
        let unit = unit.as_deref().unwrap_or("");
        println!("Best overall: {} {}", self.global_best.1, unit);
        println!("Best: {} {}", self.current_best, unit);
        println!("Worst: {} {}", self.current_worst, unit);
    }
}

impl GenerationSummary {
    pub(crate) fn new(
        global_best: ExecutionLog,
        (current_best, current_worst): (f64, f64),
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
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
                let space = specification.get_genetic_space();
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
            let space = specification.get_genetic_space();
            space.mutate(options, parameter);
        });
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
