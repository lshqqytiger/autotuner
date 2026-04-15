use crate::{
    parameter::{Combination, Value, space},
    utils::interner::Intern,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) enum Object {
    Constant(Value),
    Parameter(String),
}

impl Object {
    fn resolve(&self, combination: &Combination) -> Value {
        match self {
            Object::Constant(v) => *v,
            Object::Parameter(name) => combination[&name.intern()],
        }
    }

    fn get_dependences(&self) -> Vec<&str> {
        match self {
            Object::Constant(_) => vec![],
            Object::Parameter(name) => vec![name.as_str()],
        }
    }
}

#[derive(Deserialize)]
pub(crate) enum Integer {
    MultipleOf(Object),
    LessOrEqualTo(Object),
}

impl Integer {
    pub(crate) fn next(
        &self,
        space: &space::Integer,
        combination: &Combination,
        value: u32,
    ) -> Option<u32> {
        if let space::Integer::Sequence(_, end) = space {
            match self {
                Integer::MultipleOf(object) => {
                    let b = object.resolve(combination);
                    if let Value::Integer(b) = b {
                        if value + b <= *end {
                            Some(value + b)
                        } else {
                            None
                        }
                    } else {
                        unreachable!()
                    }
                }
                Integer::LessOrEqualTo(object) => {
                    let b = if let Value::Integer(b) = object.resolve(combination) {
                        b.min(*end)
                    } else {
                        unreachable!()
                    };
                    if value < b { Some(value + 1) } else { None }
                }
            }
        } else {
            unreachable!()
        }
    }

    pub(crate) fn adjust(&self, name: &str, combination: &mut Combination) {
        match self {
            Integer::MultipleOf(object) => {
                let b = object.resolve(combination);
                let a = combination.get_mut(name).unwrap();
                if let (Value::Integer(a), Value::Integer(b)) = (a, b) {
                    let remainder = *a % b;
                    if remainder != 0 {
                        let d = if remainder * 2 == b {
                            rand::random()
                        } else if remainder * 2 > b {
                            true
                        } else {
                            false
                        };
                        if d {
                            *a += b - remainder;
                        } else {
                            *a -= remainder;
                        }
                    }
                } else {
                    unreachable!()
                }
            }
            Integer::LessOrEqualTo(object) => {
                let b = object.resolve(combination);
                let a = combination.get_mut(name).unwrap();
                if let (Value::Integer(a), Value::Integer(b)) = (a, b) {
                    if *a > b {
                        *a = b;
                    }
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub(crate) fn get_dependences(&self) -> Vec<&str> {
        match self {
            Integer::MultipleOf(object) => object.get_dependences(),
            Integer::LessOrEqualTo(object) => object.get_dependences(),
        }
    }
}
