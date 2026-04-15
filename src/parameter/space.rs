use crate::parameter::{Space, Value};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum Integer {
    Sequence(u32, u32),
    Candidates(Vec<u32>),
}

impl Space for Integer {
    #[inline]
    fn first(&self) -> Value {
        match self {
            Integer::Sequence(start, _) => Value::Integer(*start),
            Integer::Candidates(_) => Value::Index(0),
        }
    }

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

    #[inline]
    fn len(&self) -> usize {
        match self {
            Integer::Sequence(start, end) => (*end - *start + 1) as usize,
            Integer::Candidates(candidates) => candidates.len(),
        }
    }
}

pub(crate) struct Switch {}

impl Space for Switch {
    #[inline]
    fn first(&self) -> Value {
        Value::Switch(false)
    }

    #[inline]
    fn random(&self) -> Value {
        Value::Switch(rand::random())
    }

    #[inline]
    fn len(&self) -> usize {
        2
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Keyword(pub(crate) Vec<String>);

impl Space for Keyword {
    #[inline]
    fn first(&self) -> Value {
        Value::Index(0)
    }

    #[inline]
    fn random(&self) -> Value {
        Value::Index(rand::random_range(0..self.0.len()))
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}
