use crate::{
    parameter::{Combination, Value},
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
