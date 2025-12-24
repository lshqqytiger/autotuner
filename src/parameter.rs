use crate::interner::Intern;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, convert::Infallible, hash::Hash, str::FromStr, sync::Arc};

#[derive(Serialize, Deserialize)]
pub enum Range {
    Sequence(i32, i32),
}

impl Range {
    fn random(&self) -> i32 {
        match self {
            Range::Sequence(start, end) => rand::random_range(*start..=*end),
        }
    }
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
pub enum Specification {
    Integer {
        transformer: Option<IntegerTransformer>,
        range: Range,
    },
    Switch,
    Keyword {
        options: Vec<String>,
    },
}

impl Specification {
    pub const TYPES: [&str; 3] = ["Integer", "Switch", "Keyword"];

    pub fn default(&self) -> Value {
        match self {
            Specification::Integer {
                transformer: _,
                range,
            } => Value::Integer(match range {
                Range::Sequence(start, _) => *start,
            }),
            Specification::Switch => Value::Switch(false),
            Specification::Keyword { options: _ } => Value::Keyword(0),
        }
    }

    pub fn random(&self) -> Value {
        match self {
            Specification::Integer {
                transformer: _,
                range,
            } => Value::Integer(range.random()),
            Specification::Switch => Value::Switch(rand::random()),
            Specification::Keyword { options } => {
                Value::Keyword(rand::random_range(0..options.len()))
            }
        }
    }

    pub fn next(&self, current: &Value) -> Option<Value> {
        match (self, current) {
            (
                Specification::Integer {
                    transformer: _,
                    range,
                },
                Value::Integer(n),
            ) => match range {
                Range::Sequence(_, end) => {
                    if *n < *end {
                        Some(Value::Integer(n + 1))
                    } else {
                        None
                    }
                }
            },
            (Specification::Switch, Value::Switch(b)) => {
                if !*b {
                    Some(Value::Switch(true))
                } else {
                    None
                }
            }
            (Specification::Keyword { options }, Value::Keyword(i)) => {
                if *i + 1 < options.len() {
                    Some(Value::Keyword(i + 1))
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn crossover(&self, a: &Value, b: &Value) -> Value {
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

    pub fn mutate(&self, code: &mut Value) {
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

#[derive(Serialize, Deserialize, Clone)]
pub enum Value {
    Integer(i32),
    Switch(bool),
    Keyword(usize),
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Integer(n) => format!("{}", n),
            Value::Switch(b) => format!("{}", if *b { "true" } else { "false" }),
            Value::Keyword(i) => format!("{}", i),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile(pub BTreeMap<Arc<str>, Arc<Specification>>);

impl Profile {
    pub fn new(profile: BTreeMap<Arc<str>, Arc<Specification>>) -> Self {
        Profile(profile)
    }

    pub fn compiler_arguments(&self, instance: &Instance) -> Vec<String> {
        let mut arguments = Vec::new();
        for (name, value) in &instance.parameters {
            match (self.0.get(name).unwrap().as_ref(), value) {
                (
                    Specification::Integer {
                        transformer: Some(transformer),
                        range: _,
                    },
                    Value::Integer(x),
                ) => {
                    arguments.push(format!("-D{}=({})", name, transformer.apply(x)));
                }
                (
                    Specification::Integer {
                        transformer: None,
                        range: _,
                    },
                    Value::Integer(x),
                ) => {
                    arguments.push(format!("-D{}={}", name, x));
                }

                (Specification::Switch, Value::Switch(x)) => {
                    if *x {
                        arguments.push(format!("-D{}", name));
                    }
                }

                (Specification::Keyword { options }, Value::Keyword(i)) => {
                    arguments.push(format!("-D{}={}", name, options[*i]));
                }

                _ => unreachable!(),
            }
        }
        arguments
    }

    pub fn display(&self, instance: &Instance) -> String {
        instance
            .parameters
            .iter()
            .map(|(name, value)| {
                let value = match (self.0.get(name).unwrap().as_ref(), value) {
                    (
                        Specification::Integer {
                            transformer: Some(transformer),
                            range: _,
                        },
                        Value::Integer(x),
                    ) => transformer.apply(x),
                    (
                        Specification::Integer {
                            transformer: None,
                            range: _,
                        },
                        Value::Integer(x),
                    ) => x.to_string(),

                    (Specification::Switch, Value::Switch(x)) => x.to_string(),

                    (Specification::Keyword { options }, Value::Keyword(i)) => options[*i].clone(),

                    _ => unreachable!(),
                };
                format!("{}={}", name, value)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub struct Instance {
    pub id: Arc<str>,
    pub parameters: BTreeMap<Arc<str>, Value>,
}

impl Serialize for Instance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut pairs = Vec::new();
        for (name, code) in &self.parameters {
            pairs.push((name.to_string(), code));
        }
        pairs.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Instance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let pairs = Vec::<(String, Value)>::deserialize(deserializer)?;
        let mut parameters = BTreeMap::new();
        for (name, code) in pairs {
            parameters.insert(name.intern(), code);
        }
        Ok(Instance::new(parameters))
    }
}

impl Instance {
    pub fn new(parameters: BTreeMap<Arc<str>, Value>) -> Self {
        Instance {
            id: Sha256::digest(serde_json::to_vec(&parameters).unwrap())
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
                .intern(),
            parameters,
        }
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
