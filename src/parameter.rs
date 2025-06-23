use crate::interner::Interner;
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

#[derive(Serialize, Deserialize)]
pub enum Range {
    Sequence(i32, i32),
}

#[derive(Serialize, Deserialize)]
pub enum Parameter {
    Integer {
        is_even: bool,
        range: Range,
        condition: Option<String>,
    },
    Switch,
}

impl Parameter {
    pub const TYPES: [&str; 2] = ["Integer", "Switch"];

    fn sanitize(&self, value: &mut Value) {
        match (self, value) {
            (
                Parameter::Integer {
                    is_even,
                    range,
                    condition,
                },
                Value::Integer(n),
            ) => {
                if condition.as_ref().is_some_and(|x| !x.is_empty()) {
                    todo!()
                }
                #[allow(irrefutable_let_patterns)]
                if let Range::Sequence(start, end) = range {
                    if *is_even && *n % 2 != 0 {
                        if *n == *start { *n += 1 } else { *n -= 1 }
                    }
                    if *n < *start {
                        *n = *start;
                    }
                    if *n > *end {
                        *n = *end;
                    }
                }
            }
            (Parameter::Switch, Value::Switch(_)) => {}
            _ => unreachable!(),
        }
    }

    pub fn random(&self) -> Value {
        let mut value = match self {
            Parameter::Integer {
                is_even: _,
                range,
                condition: _,
            } => {
                let value = match range {
                    Range::Sequence(start, end) => rand::random_range(*start..=*end),
                };
                Value::Integer(value)
            }
            Parameter::Switch => Value::Switch(rand::random()),
        };
        self.sanitize(&mut value);
        value
    }
}

#[derive(Serialize, Deserialize)]
pub enum Value {
    Integer(i32),
    Switch(bool),
}

impl Value {
    fn crossover(a: &Value, b: &Value) -> Value {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Value::Integer((*a + *b) / 2),
            (Value::Switch(a), Value::Switch(b)) => {
                if *a == *b {
                    Value::Switch(*a)
                } else {
                    Value::Switch(rand::random())
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&mut self) {
        match self {
            Value::Integer(n) => {
                // variation in -10% ~ +10% of the value
                let range = (*n as f64 * 0.1) as i32;
                *n += rand::random_range(-range..=range);
            }
            Value::Switch(b) => {
                // 10% chance to flip the switch
                if rand::random_ratio(1, 10) {
                    *b = rand::random();
                }
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Switch(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(_) => write!(f, "Integer({})", self),
            Value::Switch(_) => write!(f, "Switch({})", self),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile(pub FxHashMap<Arc<str>, Parameter>);

impl Profile {
    pub fn sanitize(&self, instance: &mut Instance) {
        for (name, parameter) in &self.0 {
            parameter.sanitize(instance.1.get_mut(name).unwrap());
        }
        instance.0 = None;
    }

    pub fn random(&self) -> Instance {
        Instance::from(
            self.0
                .iter()
                .map(|(name, parameter)| (name.clone(), parameter.random()))
                .collect::<FxHashMap<Arc<str>, Value>>(),
        )
    }
}

#[derive(Debug)]
pub struct Instance(Option<Arc<str>>, FxHashMap<Arc<str>, Value>);

impl Instance {
    pub fn crossover(a: &Instance, b: &Instance) -> Instance {
        let mut parameters = FxHashMap::default();
        for parameter in &a.1 {
            parameters.insert(
                parameter.0.clone(),
                Value::crossover(&a.1[parameter.0], &b.1[parameter.0]),
            );
        }
        Instance::from(parameters)
    }

    pub fn mutate(&mut self) {
        for parameter in &mut self.1 {
            parameter.1.mutate();
        }
        self.0 = None;
    }

    pub fn get_identifier(&mut self) -> Arc<str> {
        if let None = self.0 {
            self.0 = Some(Interner::intern(
                &self
                    .1
                    .iter()
                    .map(|(name, value)| format!("{}={}", name, value))
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        self.0.clone().unwrap()
    }

    pub fn compiler_arguments(&self) -> Vec<String> {
        let mut arguments = Vec::new();
        for parameter in &self.1 {
            match parameter.1 {
                Value::Integer(x) => {
                    arguments.push(format!("-D{}={}", parameter.0, x));
                }
                Value::Switch(x) => {
                    if *x {
                        arguments.push(format!("-D{}", parameter.0));
                    }
                }
            }
        }
        arguments
    }
}

impl From<FxHashMap<Arc<str>, Value>> for Instance {
    fn from(values: FxHashMap<Arc<str>, Value>) -> Self {
        Instance(None, values)
    }
}
