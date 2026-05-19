use crate::{
    criterion::Criterion,
    parameter::{Combination, Profile, Value},
    utils::interner::Intern,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::Display,
    hash::{self, Hash},
    sync::Arc,
};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Fitness {
    Valid(f64),
    Invalid,
    Unknown,
}

impl PartialOrd for Fitness {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Fitness::Valid(a), Fitness::Valid(b)) => a.partial_cmp(b),
            _ => {
                panic!("invalid comparison")
            }
        }
    }
}

impl Display for Fitness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fitness::Valid(x) => write!(f, "{}", x),
            Fitness::Invalid => write!(f, "invalid"),
            Fitness::Unknown => write!(f, "unknown"),
        }
    }
}

impl Fitness {
    #[inline]
    pub(crate) fn is_nan(&self) -> bool {
        matches!(self, Fitness::Valid(x) if x.is_nan())
    }

    #[inline]
    pub(crate) fn is_valid(&self) -> bool {
        matches!(self, Fitness::Valid(_))
    }

    #[inline]
    pub(crate) fn into_f64(self, criterion: Criterion) -> f64 {
        match self {
            Fitness::Valid(x) => x,
            Fitness::Invalid => criterion.invalid(),
            Fitness::Unknown => panic!("tried to get fitness of unevaluated individual"),
        }
    }
}

pub(crate) trait Representative<T> {
    fn representative(&self, criterion: Criterion) -> T;
}

impl Representative<Fitness> for Vec<Fitness> {
    fn representative(&self, criterion: Criterion) -> Fitness {
        match criterion {
            Criterion::Maximum => self.iter().fold(Fitness::Invalid, |a, b| match (a, b) {
                (Fitness::Valid(x), Fitness::Valid(y)) => Fitness::Valid(x.max(*y)),
                (Fitness::Valid(x), _) => Fitness::Valid(x),
                (_, Fitness::Valid(y)) => Fitness::Valid(*y),
                _ => Fitness::Invalid,
            }),
            Criterion::Minimum => self.iter().fold(Fitness::Invalid, |a, b| match (a, b) {
                (Fitness::Valid(x), Fitness::Valid(y)) => Fitness::Valid(x.min(*y)),
                (Fitness::Valid(x), _) => Fitness::Valid(x),
                (_, Fitness::Valid(y)) => Fitness::Valid(*y),
                _ => Fitness::Invalid,
            }),
            Criterion::Median => {
                let mut values = self
                    .iter()
                    .filter_map(|f| match f {
                        Fitness::Valid(x) => Some(*x),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    Fitness::Invalid
                } else {
                    values.sort_by(|a, b| a.total_cmp(b));
                    Fitness::Valid(values[values.len() / 2])
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct Individual {
    pub(crate) id: Arc<str>,
    pub(crate) parameters: Combination,

    // for compilation
    pub(crate) arguments: Vec<String>,

    // for evaluation
    pub(crate) fitness: Fitness,
}

impl Hash for Individual {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Individual {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Individual {}

impl Serialize for Individual {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.parameters.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Individual {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized = BTreeMap::<String, Value>::deserialize(deserializer)?;
        let mut parameters = BTreeMap::new();
        for (name, code) in deserialized {
            parameters.insert(name.intern(), code);
        }
        Ok(Individual::new(parameters))
    }
}

impl Individual {
    pub(crate) fn new(parameters: BTreeMap<Arc<str>, Value>) -> Self {
        Individual {
            id: Sha256::digest(serde_json::to_vec(&parameters).unwrap())
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
                .intern(),
            parameters,
            arguments: Vec::new(),
            fitness: Fitness::Unknown,
        }
    }

    pub(crate) fn random(profile: &Profile) -> Self {
        let mut individual = Self::new(
            profile
                .0
                .iter()
                .map(|(name, parameter)| (name.clone(), parameter.get_space().random()))
                .collect::<BTreeMap<Arc<str>, Value>>(),
        );
        profile.adjust(&mut individual);
        individual
    }

    pub(crate) fn reset(&mut self) {
        self.arguments.clear();
        self.fitness = Fitness::Unknown;
    }
}
