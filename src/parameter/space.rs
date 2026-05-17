use crate::{
    configuration::Mutation,
    parameter::{Space, Value},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum Integer {
    Sequence(u32, u32),
    Candidates(Vec<u32>),
}

impl Space for Integer {
    #[inline]
    fn random(&self) -> Value {
        match self {
            Integer::Sequence(start, end) => Value::Integer(rand::random_range(*start..=*end)),
            Integer::Candidates(candidates) => {
                Value::Index(rand::random_range(0..candidates.len()))
            }
        }
    }

    fn adjust(&self, value: &mut Value) {
        match (self, value) {
            (Integer::Sequence(start, end), Value::Integer(n)) => {
                if *n < *start {
                    *n = *start;
                } else if *n > *end {
                    *n = *end;
                }
            }
            (Integer::Candidates(_), _) => {}
            _ => unreachable!(),
        }
    }

    fn crossover(&self, a: Value, b: Value) -> Value {
        match (self, a, b) {
            (Integer::Sequence(_, _), Value::Integer(a), Value::Integer(b)) => {
                Value::Integer((a + b) / 2)
            }
            (Integer::Candidates(_), Value::Index(a), Value::Index(b)) => {
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
                        (Integer::Sequence(start, end), Value::Integer(n)) => {
                            let mut variation = ((end - start) as f64 * variation.value) as i32;
                            if variation == 0 {
                                variation = 1;
                            }

                            let mutated = (*n as i32) + rand::random_range(-variation..=variation);
                            *n = if mutated < 0 { 0 } else { mutated as u32 };
                        }
                        (Integer::Candidates(candidates), Value::Index(i)) => {
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

pub(crate) struct Switch {}

impl Space for Switch {
    #[inline]
    fn random(&self) -> Value {
        Value::Switch(rand::random())
    }

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

#[derive(Serialize, Deserialize)]
pub(crate) struct Keyword(pub(crate) Vec<String>);

impl Space for Keyword {
    #[inline]
    fn random(&self) -> Value {
        Value::Index(rand::random_range(0..self.0.len()))
    }

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
