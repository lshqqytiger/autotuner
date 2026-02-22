pub(crate) mod execution_result;
pub(crate) mod options;
pub(crate) mod ranking;
pub(crate) mod state;

use crate::execution_log::ExecutionLog;
use crate::parameter::{
    Instance, IntegerSpace, KeywordSpace, Profile, Space, Specification, SwitchSpace, Value,
};
use crate::strategies::genetic::options::MutationOptions;
use serde::Serialize;
use std::time::SystemTime;
use std::{collections::BTreeMap, fmt};

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
    fn crossover(&self, a: &Value, b: &Value) -> Value;
    fn mutate(&self, options: &MutationOptions, value: &mut Value);
}

impl GeneticSpace for IntegerSpace {
    fn crossover(&self, a: &Value, b: &Value) -> Value {
        match (self, a, b) {
            (IntegerSpace::Sequence(_, _), Value::Integer(a), Value::Integer(b)) => {
                Value::Integer((*a + *b) / 2)
            }
            (IntegerSpace::Candidates(_), Value::Index(a), Value::Index(b)) => {
                if *a == *b {
                    Value::Index(*a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, options: &MutationOptions, code: &mut Value) {
        if !rand::random_bool(options.probability) {
            return;
        }
        match (self, code) {
            (IntegerSpace::Sequence(start, end), Value::Integer(n)) => {
                let mut variation = ((end - start) as f64 * options.variation) as i32;
                if variation == 0 {
                    variation = 1;
                }

                *n += rand::random_range(-variation..=variation);

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
    }
}

impl GeneticSpace for SwitchSpace {
    fn crossover(&self, a: &Value, b: &Value) -> Value {
        match (a, b) {
            (Value::Switch(a), Value::Switch(b)) => {
                if *a == *b {
                    Value::Switch(*a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, options: &MutationOptions, code: &mut Value) {
        if !rand::random_bool(options.probability) {
            return;
        }

        if let Value::Switch(b) = code {
            *b = !*b;
        }
    }
}

impl GeneticSpace for KeywordSpace {
    fn crossover(&self, a: &Value, b: &Value) -> Value {
        match (a, b) {
            (Value::Index(a), Value::Index(b)) => {
                if *a == *b {
                    Value::Index(*a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, options: &MutationOptions, code: &mut Value) {
        if rand::random_bool(options.probability) {
            *code = self.random();
        }
    }
}

#[derive(Serialize)]
pub(crate) struct GenerationSummary {
    pub(crate) timestamp: u64,
    pub(crate) best_overall: ExecutionLog,
    pub(crate) current_best: f64,
    pub(crate) current_worst: f64,
}

impl fmt::Display for GenerationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Best overall: ")?;
        writeln!(f, "{} ms", self.best_overall.1)?;
        writeln!(f, "Best: {} ms", self.current_best)?;
        writeln!(f, "Worst: {} ms", self.current_worst)
    }
}

impl GenerationSummary {
    pub(crate) fn new(
        best_overall: ExecutionLog,
        (current_best, current_worst): (f64, f64),
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        GenerationSummary {
            timestamp,
            best_overall,
            current_best,
            current_worst,
        }
    }
}

pub(crate) fn crossover(profile: &Profile, a: &Instance, b: &Instance) -> Instance {
    let mut parameters = BTreeMap::new();
    for parameter in &a.parameters {
        parameters.insert(
            parameter.0.clone(),
            profile
                .0
                .get(parameter.0)
                .unwrap()
                .get_genetic_space()
                .crossover(&a.parameters[parameter.0], &b.parameters[parameter.0]),
        );
    }
    Instance::new(parameters)
}

pub(crate) fn mutate(profile: &Profile, options: &MutationOptions, instance: &mut Instance) {
    for (name, parameter) in &mut instance.parameters {
        profile
            .0
            .get(name)
            .unwrap()
            .get_genetic_space()
            .mutate(options, parameter);
    }
}

pub(crate) fn stochastic_universal_sampling(roulette: &[(f64, usize)], n: usize) -> Vec<usize> {
    assert!(!roulette.is_empty());
    assert_ne!(n, 0);

    assert!(roulette.iter().all(|(f, _)| *f >= 0.0));

    let total_fitness: f64 = roulette.iter().map(|(fitness, _)| fitness).sum();
    assert!(total_fitness > 0.0);

    let distance = total_fitness / n as f64;
    let start = rand::random::<f64>() * distance;

    let mut selected = Vec::with_capacity(n);

    let mut current_sum = 0.0;
    let mut current_index = 0usize;

    for i in 0..n {
        let pointer = start + i as f64 * distance;

        while current_index < roulette.len() && current_sum < pointer {
            current_sum += roulette[current_index].0;
            current_index += 1;
        }

        if current_index == 0 {
            selected.push(roulette[0].1);
        } else if current_index <= roulette.len() {
            selected.push(roulette[current_index - 1].1);
        } else {
            selected.push(roulette[roulette.len() - 1].1);
        }
    }

    selected
}
