pub mod mapping;

use crate::{interner::Interner, parameter::mapping::Mapping};
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, hash::Hash, sync::Arc};

#[derive(Serialize, Deserialize)]
pub enum Range {
    Sequence(i32, i32),
}

#[derive(Serialize, Deserialize)]
pub enum Parameter {
    Integer {
        mapping: Option<Mapping>,
        range: Range,
    },
    Switch,
}

impl Parameter {
    pub const TYPES: [&str; 2] = ["Integer", "Switch"];

    fn sanitize(&self, code: Code) -> Code {
        match (self, code) {
            (Parameter::Integer { mapping: _, range }, Code::Integer(n)) => {
                #[allow(irrefutable_let_patterns)]
                if let Range::Sequence(start, end) = range {
                    if n < *start {
                        return Code::Integer(*start);
                    }
                    if n > *end {
                        return Code::Integer(*end);
                    }
                }
                Code::Integer(n)
            }
            (Parameter::Switch, Code::Switch(x)) => Code::Switch(x),
            _ => unreachable!(),
        }
    }

    pub fn random(&self) -> Code {
        match self {
            Parameter::Integer { mapping: _, range } => {
                let value = match range {
                    Range::Sequence(start, end) => rand::random_range(*start..=*end),
                };
                Code::Integer(value)
            }
            Parameter::Switch => Code::Switch(rand::random()),
        }
    }
}

#[derive(Clone)]
pub enum Code {
    Integer(i32),
    Switch(bool),
}

impl Code {
    fn crossover(a: &Code, b: &Code) -> Code {
        match (a, b) {
            (Code::Integer(a), Code::Integer(b)) => Code::Integer((*a + *b) / 2),
            (Code::Switch(a), Code::Switch(b)) => {
                if *a == *b {
                    Code::Switch(*a)
                } else {
                    Code::Switch(rand::random())
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&mut self) {
        match self {
            Code::Integer(n) => {
                // variation in -10% ~ +10% of the value
                let mut range = (*n as f64 * 0.1) as i32;
                if range == 0 {
                    range = 1;
                }
                *n += rand::random_range(-range..=range);
            }
            Code::Switch(b) => {
                // 10% chance to flip the switch
                if rand::random_ratio(1, 10) {
                    *b = rand::random();
                }
            }
        }
    }
}

impl Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Code::Integer(n) => write!(f, "{}", n),
            Code::Switch(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        }
    }
}

enum Value {
    Integer(i32),
    Switch(bool),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Switch(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile(FxHashMap<Arc<str>, Parameter>);

impl Profile {
    pub fn new(profile: FxHashMap<Arc<str>, Parameter>) -> Arc<Self> {
        Arc::new(Profile(profile))
    }

    pub fn random(self: &Arc<Profile>) -> Instance {
        Instance::new(
            self.clone(),
            self.0
                .iter()
                .map(|(name, parameter)| (name.clone(), parameter.random()))
                .collect::<FxHashMap<Arc<str>, Code>>(),
        )
    }

    fn get_unchecked(&self, name: &str) -> &Parameter {
        self.0.get(name).unwrap()
    }
}

pub struct Instance {
    id: Arc<str>,
    profile: Arc<Profile>,
    parameters: FxHashMap<Arc<str>, Code>,
}

impl Instance {
    pub fn new(profile: Arc<Profile>, parameters: FxHashMap<Arc<str>, Code>) -> Self {
        Instance {
            id: Interner::intern(
                &parameters
                    .iter()
                    .map(|(name, code)| format!("{}={}", name, code))
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            profile,
            parameters,
        }
    }

    pub fn crossover(a: &Instance, b: &Instance) -> Instance {
        let mut parameters = FxHashMap::default();
        for parameter in &a.parameters {
            parameters.insert(
                parameter.0.clone(),
                Code::crossover(&a.parameters[parameter.0], &b.parameters[parameter.0]),
            );
        }
        Instance::new(a.profile.clone(), parameters)
    }

    pub fn mutate(self) -> Instance {
        let mut parameters = self.parameters.clone();
        for parameter in parameters.values_mut() {
            parameter.mutate();
        }
        Instance::new(self.profile.clone(), parameters)
    }

    pub fn sanitize(self) -> Instance {
        let mut parameters = FxHashMap::default();
        for (name, parameter) in self.parameters {
            parameters.insert(
                name.clone(),
                self.profile.get_unchecked(&name).sanitize(parameter),
            );
        }
        Instance::new(self.profile.clone(), parameters)
    }

    fn parameters(&self) -> impl Iterator<Item = (&Arc<str>, Value)> {
        self.parameters.iter().map(|(name, code)| match code {
            Code::Integer(x) => (
                name,
                Value::Integer(
                    if let Parameter::Integer {
                        mapping: Some(mapping),
                        range: _,
                    } = self.profile.get_unchecked(name)
                    {
                        mapping.map(*x)
                    } else {
                        *x
                    },
                ),
            ),
            Code::Switch(x) => (name, Value::Switch(*x)),
        })
    }

    pub fn compiler_arguments(&self) -> Vec<String> {
        let mut arguments = Vec::new();
        for (name, value) in self.parameters() {
            match value {
                Value::Integer(x) => arguments.push(format!("-D{}={}", name, x)),
                Value::Switch(x) => {
                    if x {
                        arguments.push(format!("-D{}", name));
                    }
                }
            }
        }
        arguments
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Instance {}

impl Hash for Instance {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.parameters()
                .map(|(name, value)| format!("{}={}", name, value))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
