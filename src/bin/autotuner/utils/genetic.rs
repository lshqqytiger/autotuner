use autotuner::parameter::{Instance, Profile, Range, Specification, Value};
use std::{collections::BTreeMap, sync::Arc};

trait Genetic {
    fn crossover(&self, a: &Value, b: &Value) -> Value;
    fn mutate(&self, value: &mut Value);
}

impl Genetic for Specification {
    fn crossover(&self, a: &Value, b: &Value) -> Value {
        match (self, a, b) {
            (
                Specification::Integer {
                    transformer: _,
                    range: _,
                },
                Value::Integer(a),
                Value::Integer(b),
            ) => Value::Integer((*a + *b) / 2),
            (Specification::Switch, Value::Switch(a), Value::Switch(b)) => {
                if *a == *b {
                    Value::Switch(*a)
                } else {
                    Value::Switch(rand::random())
                }
            }
            (Specification::Keyword { options: _ }, Value::Keyword(a), Value::Keyword(b)) => {
                if *a == *b {
                    Value::Keyword(*a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, code: &mut Value) {
        match (self, code) {
            (
                Specification::Integer {
                    transformer: _,
                    range,
                },
                Value::Integer(n),
            ) => {
                // 10% chance to completely randomize the value
                if rand::random_bool(0.1) {
                    *n = range.random();
                    return;
                }

                match range {
                    Range::Sequence(start, end) => {
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
                }
            }
            (Specification::Switch, Value::Switch(b)) => {
                // 10% chance to completely randomize the switch
                if rand::random_bool(0.1) {
                    *b = rand::random();
                    return;
                }

                // 20% chance to flip the switch
                if rand::random_bool(0.2) {
                    *b = !*b;
                }
            }
            (Specification::Keyword { options }, Value::Keyword(i)) => {
                // 20% chance to change the keyword
                if rand::random_bool(0.2) {
                    *i = rand::random_range(0..options.len());
                }
            }
            _ => unreachable!(),
        }
    }
}

pub(crate) fn random(profile: &Profile) -> Instance {
    Instance::new(
        profile
            .0
            .iter()
            .map(|(name, parameter)| (name.clone(), parameter.random()))
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
                .crossover(&a.parameters[parameter.0], &b.parameters[parameter.0]),
        );
    }
    Instance::new(parameters)
}

pub(crate) fn mutate(profile: &Profile, instance: &mut Instance) {
    for (name, parameter) in &mut instance.parameters {
        profile.0.get(name).unwrap().mutate(parameter);
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
