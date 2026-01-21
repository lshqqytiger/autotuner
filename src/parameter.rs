use crate::utils::interner::Intern;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, convert::Infallible, hash::Hash, str::FromStr, sync::Arc};

pub(crate) trait Space {
    fn default(&self) -> Value;
    fn random(&self) -> Value;
    fn next(&self, current: &Value) -> Option<Value>;
    fn len(&self) -> usize;
}

#[derive(Serialize, Deserialize)]
pub(crate) enum IntegerSpace {
    Sequence(i32, i32),
    Candidates(Vec<i32>),
}

impl Space for IntegerSpace {
    #[inline]
    fn default(&self) -> Value {
        match self {
            IntegerSpace::Sequence(start, _) => Value::Integer(*start),
            IntegerSpace::Candidates(_) => Value::Index(0),
        }
    }

    #[inline]
    fn random(&self) -> Value {
        match self {
            IntegerSpace::Sequence(start, end) => Value::Integer(rand::random_range(*start..=*end)),
            IntegerSpace::Candidates(candidates) => {
                Value::Index(rand::random_range(0..candidates.len()))
            }
        }
    }

    fn next(&self, current: &Value) -> Option<Value> {
        match (self, current) {
            (IntegerSpace::Sequence(_, end), Value::Integer(n)) => {
                if *n < *end {
                    Some(Value::Integer(n + 1))
                } else {
                    None
                }
            }
            (IntegerSpace::Candidates(candidates), Value::Index(i)) => {
                if *i + 1 < candidates.len() {
                    Some(Value::Index(i + 1))
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        match self {
            IntegerSpace::Sequence(start, end) => (*end - *start + 1) as usize,
            IntegerSpace::Candidates(candidates) => candidates.len(),
        }
    }
}

pub(crate) struct SwitchSpace {}

impl Space for SwitchSpace {
    #[inline]
    fn default(&self) -> Value {
        Value::Switch(false)
    }

    #[inline]
    fn random(&self) -> Value {
        Value::Switch(rand::random())
    }

    fn next(&self, current: &Value) -> Option<Value> {
        match current {
            Value::Switch(b) => {
                if !*b {
                    Some(Value::Switch(true))
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        2
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct KeywordSpace(pub(crate) Vec<String>);

impl Space for KeywordSpace {
    #[inline]
    fn default(&self) -> Value {
        Value::Index(0)
    }

    #[inline]
    fn random(&self) -> Value {
        Value::Index(rand::random_range(0..self.0.len()))
    }

    fn next(&self, current: &Value) -> Option<Value> {
        match current {
            Value::Index(i) => {
                if *i + 1 < self.0.len() {
                    Some(Value::Index(i + 1))
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct IntegerTransformer(String);

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
pub(crate) enum Specification {
    Integer {
        transformer: Option<IntegerTransformer>,
        space: IntegerSpace,
    },
    Switch,
    Keyword(KeywordSpace),
}

impl Specification {
    pub(crate) const SWITCH_SPACE: SwitchSpace = SwitchSpace {};

    #[inline]
    pub(crate) fn get_space(&self) -> &dyn Space {
        match self {
            Specification::Integer {
                transformer: _,
                space,
            } => space,
            Specification::Switch => &Self::SWITCH_SPACE,
            Specification::Keyword(options) => options,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum Value {
    Integer(i32),
    Switch(bool),
    Index(usize),
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Integer(n) => format!("{}", n),
            Value::Switch(b) => format!("{}", if *b { "true" } else { "false" }),
            Value::Index(i) => format!("{}", i),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Profile(pub(crate) BTreeMap<Arc<str>, Arc<Specification>>);

impl Profile {
    pub(crate) fn compiler_arguments(&self, instance: &Instance) -> Vec<String> {
        let mut arguments = Vec::new();
        for (name, value) in &instance.parameters {
            match (self.0.get(name).unwrap().as_ref(), value) {
                (
                    Specification::Integer {
                        transformer: Some(transformer),
                        space: IntegerSpace::Sequence(_, _),
                    },
                    Value::Integer(x),
                ) => {
                    arguments.push(format!("-D{}=({})", name, transformer.apply(x)));
                }
                (
                    Specification::Integer {
                        transformer: None,
                        space: IntegerSpace::Sequence(_, _),
                    },
                    Value::Integer(x),
                ) => {
                    arguments.push(format!("-D{}={}", name, x));
                }
                (
                    Specification::Integer {
                        transformer: Some(_),
                        space: IntegerSpace::Candidates(_),
                    },
                    Value::Index(_),
                ) => unimplemented!(),
                (
                    Specification::Integer {
                        transformer: None,
                        space: IntegerSpace::Candidates(candidates),
                    },
                    Value::Index(i),
                ) => {
                    arguments.push(format!("-D{}={}", name, candidates[*i]));
                }

                (Specification::Switch, Value::Switch(x)) => {
                    if *x {
                        arguments.push(format!("-D{}", name));
                    }
                }

                (Specification::Keyword(KeywordSpace(options)), Value::Index(i)) => {
                    arguments.push(format!("-D{}={}", name, options[*i]));
                }

                _ => unreachable!(),
            }
        }
        arguments
    }

    pub(crate) fn display(&self, instance: &Instance) -> String {
        instance
            .parameters
            .iter()
            .map(|(name, value)| {
                let value = match (self.0.get(name).unwrap().as_ref(), value) {
                    (
                        Specification::Integer {
                            transformer: Some(transformer),
                            space: IntegerSpace::Sequence(_, _),
                        },
                        Value::Integer(x),
                    ) => transformer.apply(x),
                    (
                        Specification::Integer {
                            transformer: None,
                            space: IntegerSpace::Sequence(_, _),
                        },
                        Value::Integer(x),
                    ) => x.to_string(),
                    (
                        Specification::Integer {
                            transformer: Some(_),
                            space: IntegerSpace::Candidates(_),
                        },
                        Value::Index(_),
                    ) => unimplemented!(),
                    (
                        Specification::Integer {
                            transformer: None,
                            space: IntegerSpace::Candidates(candidates),
                        },
                        Value::Index(i),
                    ) => candidates[*i].to_string(),

                    (Specification::Switch, Value::Switch(x)) => x.to_string(),

                    (Specification::Keyword(KeywordSpace(options)), Value::Index(i)) => {
                        options[*i].clone()
                    }

                    _ => unreachable!(),
                };
                format!("{}={}", name, value)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(crate) fn len(&self) -> usize {
        let mut size = 1;
        for specification in self.0.values() {
            size *= specification.get_space().len();
        }
        size
    }
}

pub(crate) struct Instance {
    pub(crate) id: Arc<str>,
    pub(crate) parameters: BTreeMap<Arc<str>, Value>,
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
    pub(crate) fn new(parameters: BTreeMap<Arc<str>, Value>) -> Self {
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
