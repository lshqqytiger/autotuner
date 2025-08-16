use crate::interner::Intern;
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, fmt::Display, hash::Hash, str::FromStr, sync::Arc};

#[derive(Serialize, Deserialize)]
pub enum Range {
    Sequence(i32, i32),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IntegerTransformer(String);

impl IntegerTransformer {
    fn apply<T: ToString>(&self, x: T) -> String {
        let stringified = x.to_string();
        self.0.replace("$x", &stringified)
    }
}

impl FromStr for IntegerTransformer {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(IntegerTransformer(s.to_string()))
    }
}

impl ToString for IntegerTransformer {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize)]
pub enum Parameter {
    Integer {
        transformer: Option<IntegerTransformer>,
        range: Range,
    },
    Switch,
    Keyword {
        options: Vec<String>,
    },
}

impl Parameter {
    pub const TYPES: [&str; 3] = ["Integer", "Switch", "Keyword"];

    pub fn random(&self) -> Code {
        match self {
            Parameter::Integer {
                transformer: _,
                range,
            } => {
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
                    transformer: _,
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
                    transformer: _,
                    range,
                },
                Code::Integer(n),
            ) => {
                // variation in -10% ~ +10% of the value
                let mut variation = (*n as f64 * 0.1) as i32;
                if variation == 0 {
                    variation = 1;
                }
                *n += rand::random_range(-variation..=variation);

                #[allow(irrefutable_let_patterns)]
                if let Range::Sequence(start, end) = range {
                    if *n < *start {
                        *n = *start;
                    } else if *n > *end {
                        *n = *end;
                    }
                }
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

#[derive(Serialize, Deserialize, Clone)]
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
    pub parameters: FxHashMap<Arc<str>, Code>,
}

impl Instance {
    pub fn new(profile: Arc<Profile>, parameters: FxHashMap<Arc<str>, Code>) -> Self {
        Instance {
            id: parameters
                .iter()
                .map(|(name, code)| format!("{}${}", name, code.to_string()))
                .collect::<Vec<_>>()
                .join(";")
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

    pub fn compiler_arguments(&self) -> Vec<String> {
        let mut arguments = Vec::new();
        for (name, code) in &self.parameters {
            match (self.profile.get_unchecked(name), code) {
                (
                    Parameter::Integer {
                        transformer: Some(transformer),
                        range: _,
                    },
                    Code::Integer(x),
                ) => {
                    arguments.push(format!("-D{}=({})", name, transformer.apply(x)));
                }
                (
                    Parameter::Integer {
                        transformer: None,
                        range: _,
                    },
                    Code::Integer(x),
                ) => {
                    arguments.push(format!("-D{}={}", name, x));
                }

                (Parameter::Switch, Code::Switch(x)) => {
                    if *x {
                        arguments.push(format!("-D{}", name));
                    }
                }

                (Parameter::Keyword { options }, Code::Keyword(i)) => {
                    arguments.push(format!("-D{}={}", name, options[*i]));
                }

                _ => unreachable!(),
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
                    let value = match (self.profile.get_unchecked(name), value) {
                        (
                            Parameter::Integer {
                                transformer: Some(transformer),
                                range: _,
                            },
                            Code::Integer(x),
                        ) => transformer.apply(x),
                        (
                            Parameter::Integer {
                                transformer: None,
                                range: _,
                            },
                            Code::Integer(x),
                        ) => x.to_string(),

                        (Parameter::Switch, Code::Switch(x)) => x.to_string(),

                        (Parameter::Keyword { options }, Code::Keyword(i)) => options[*i].clone(),

                        _ => unreachable!(),
                    };
                    format!("{}={}", name, value)
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
