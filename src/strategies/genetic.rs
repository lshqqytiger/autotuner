use crate::execution_result::ExecutionResult;
use crate::parameter::{
    Instance, IntegerSpace, KeywordSpace, Profile, Space, Specification, SwitchSpace, Value,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, sync::Arc};

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(crate) generation: usize,
    pub(crate) instances: Vec<Arc<Instance>>,
}

impl State {
    pub(crate) fn new(profile: &Profile, initial: usize) -> Self {
        let mut instances = Vec::with_capacity(initial);
        for _ in 0..initial {
            instances.push(Arc::new(random(profile)));
        }
        State {
            generation: 0,
            instances,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct GenerationSummary {
    pub(crate) best_overall: Option<ExecutionResult>,
    pub(crate) current_best: f64,
    pub(crate) current_worst: f64,
}

impl fmt::Display for GenerationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(best_overall) = &self.best_overall {
            writeln!(f, "Best overall: {} ms", best_overall.1)?;
        }
        writeln!(f, "Best: {} ms", self.current_best)?;
        writeln!(f, "Worst: {} ms", self.current_worst)
    }
}

impl GenerationSummary {
    pub(crate) fn new(
        best_overall: Option<ExecutionResult>,
        (current_best, current_worst): (f64, f64),
    ) -> Self {
        GenerationSummary {
            best_overall,
            current_best,
            current_worst,
        }
    }
}

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
    fn mutate(&self, value: &mut Value);
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

    fn mutate(&self, code: &mut Value) {
        match (self, code) {
            (IntegerSpace::Sequence(start, end), Value::Integer(n)) => {
                // 10% chance to completely randomize the value
                if rand::random_bool(0.1) {
                    *n = rand::random_range(*start..=*end);
                    return;
                }

                // variation in -20% ~ +20%
                let mut variation = ((end - start) as f64 * 0.2) as i32;
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
                // 20% chance
                if rand::random_bool(0.2) {
                    *i = rand::random_range(0..candidates.len());
                }
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

    fn mutate(&self, code: &mut Value) {
        // 10% chance to completely randomize the switch
        if rand::random_bool(0.1) {
            *code = self.random();
            return;
        }

        // 20% chance to flip the switch
        if rand::random_bool(0.2) {
            if let Value::Switch(b) = code {
                *b = !*b;
            }
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

    fn mutate(&self, code: &mut Value) {
        // 20% chance to change the keyword
        if rand::random_bool(0.2) {
            *code = self.random();
        }
    }
}

pub(crate) fn random(profile: &Profile) -> Instance {
    Instance::new(
        profile
            .0
            .iter()
            .map(|(name, parameter)| (name.clone(), parameter.get_space().random()))
            .collect::<BTreeMap<Arc<str>, Value>>(),
    )
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

pub(crate) fn mutate(profile: &Profile, instance: &mut Instance) {
    for (name, parameter) in &mut instance.parameters {
        profile
            .0
            .get(name)
            .unwrap()
            .get_genetic_space()
            .mutate(parameter);
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
