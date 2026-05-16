use crate::parameter::{Space, Value};
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
}

pub(crate) struct Switch {}

impl Space for Switch {
    #[inline]
    fn random(&self) -> Value {
        Value::Switch(rand::random())
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Keyword(pub(crate) Vec<String>);

impl Space for Keyword {
    #[inline]
    fn random(&self) -> Value {
        Value::Index(rand::random_range(0..self.0.len()))
    }
}
