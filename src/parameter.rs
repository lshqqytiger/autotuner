use crate::interner::Intern;
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, hash::Hash, sync::Arc};

#[derive(Serialize, Deserialize)]
pub enum Range {
    Sequence(i32, i32),
}

#[derive(Serialize, Deserialize)]
pub struct Mapping(Option<String>);

impl Mapping {
    fn map<T: ToString>(&self, x: T) -> String {
        let stringified = x.to_string();
        if let Some(mapping) = &self.0 {
            mapping.replace("$x", &stringified)
        } else {
            stringified
        }
    }
}

impl From<Option<String>> for Mapping {
    fn from(value: Option<String>) -> Self {
        Mapping(value)
    }
}

#[derive(Serialize, Deserialize)]
pub enum Parameter {
    Integer { mapping: Mapping, range: Range },
    Switch,
    Keyword { options: Vec<String> },
}

impl Parameter {
    pub const TYPES: [&str; 3] = ["Integer", "Switch", "Keyword"];

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
            Parameter::Keyword { options } => Code::Keyword(rand::random_range(0..options.len())),
        }
    }

    fn crossover(&self, a: &Code, b: &Code) -> Code {
        match (self, a, b) {
            (
                Parameter::Integer {
                    mapping: _,
                    range: _,
                },
                Code::Integer(a),
                Code::Integer(b),
            ) => Code::Integer((*a + *b) / 2),
            (Parameter::Switch, Code::Switch(a), Code::Switch(b)) => {
                if *a == *b {
                    Code::Switch(*a)
                } else {
                    Code::Switch(rand::random())
                }
            }
            (Parameter::Keyword { options: _ }, Code::Keyword(a), Code::Keyword(b)) => {
                if *a == *b {
                    Code::Keyword(*a)
                } else {
                    self.random()
                }
            }
            _ => unreachable!(),
        }
    }

    fn mutate(&self, code: &mut Code) {
        match (self, code) {
            (
                Parameter::Integer {
                    mapping: _,
                    range: _,
                },
                Code::Integer(n),
            ) => {
                // variation in -10% ~ +10% of the value
                let mut range = (*n as f64 * 0.1) as i32;
                if range == 0 {
                    range = 1;
                }
                *n += rand::random_range(-range..=range);
            }
            (Parameter::Switch, Code::Switch(b)) => {
                // 10% chance to flip the switch
                if rand::random_ratio(1, 10) {
                    *b = rand::random();
                }
            }
            (Parameter::Keyword { options }, Code::Keyword(i)) => {
                // 10% chance to change the keyword
                if rand::random_ratio(1, 10) {
                    *i = rand::random_range(0..options.len());
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub enum Code {
    Integer(i32),
    Switch(bool),
    Keyword(usize),
}

impl ToString for Code {
    fn to_string(&self) -> String {
        match self {
            Code::Integer(n) => format!("{}", n),
            Code::Switch(b) => format!("{}", if *b { "true" } else { "false" }),
            Code::Keyword(i) => format!("{}", i),
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
            id: parameters
                .iter()
                .map(|(name, code)| format!("{}={}", name, code.to_string()))
                .collect::<Vec<_>>()
                .join(",")
                .intern(),
            profile,
            parameters,
        }
    }

    pub fn crossover(a: &Instance, b: &Instance) -> Instance {
        let mut parameters = FxHashMap::default();
        for parameter in &a.parameters {
            parameters.insert(
                parameter.0.clone(),
                a.profile
                    .get_unchecked(parameter.0)
                    .crossover(&a.parameters[parameter.0], &b.parameters[parameter.0]),
            );
        }
        Instance::new(a.profile.clone(), parameters)
    }

    pub fn mutate(self) -> Instance {
        let mut parameters = self.parameters.clone();
        for (name, parameter) in &mut parameters {
            self.profile.get_unchecked(&name).mutate(parameter);
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

    pub fn compiler_arguments(&self) -> Vec<String> {
        let mut arguments = Vec::new();
        for (name, code) in &self.parameters {
            match code {
                Code::Integer(x) => {
                    if let Parameter::Integer { mapping, range: _ } =
                        self.profile.get_unchecked(name)
                    {
                        arguments.push(format!("-D{}=({})", name, mapping.map(x)));
                    } else {
                        unreachable!()
                    }
                }
                Code::Switch(x) => {
                    if *x {
                        arguments.push(format!("-D{}", name));
                    }
                }
                Code::Keyword(i) => {
                    if let Parameter::Keyword { options } = self.profile.get_unchecked(name) {
                        arguments.push(format!("-D{}={}", name, options[*i]));
                    } else {
                        unreachable!()
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
            self.parameters
                .iter()
                .map(|(name, value)| {
                    let value = match value {
                        Code::Integer(x) => {
                            if let Parameter::Integer { mapping, range: _ } =
                                self.profile.get_unchecked(name)
                            {
                                mapping.map(x)
                            } else {
                                unreachable!()
                            }
                        }
                        Code::Switch(x) => x.to_string(),
                        Code::Keyword(i) => {
                            if let Parameter::Keyword { options } = self.profile.get_unchecked(name)
                            {
                                options[*i].clone()
                            } else {
                                unreachable!()
                            }
                        }
                    };
                    format!("{}={}", name, value)
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
