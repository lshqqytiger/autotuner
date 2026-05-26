mod condition;

pub(crate) mod space;

use crate::{configuration::Mutation, individual::Individual};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) trait Space {
    fn random(&self) -> Value;
    fn adjust(&self, _: &mut Value) {}
    fn crossover(&self, a: Value, b: Value) -> Value;
    fn mutate(&self, mutations: &Mutation, value: &mut Value);
}

#[derive(Deserialize)]
pub(crate) enum Specification {
    Integer {
        space: space::Integer,
        #[serde(default)]
        condition: Option<condition::Integer>,
    },
    Switch,
    Keyword(space::Keyword),
}

impl Specification {
    pub(crate) const SWITCH_SPACE: space::Switch = space::Switch {};

    #[inline]
    pub(crate) fn get_space(&self) -> &dyn Space {
        match self {
            Specification::Integer {
                space,
                condition: _,
            } => space,
            Specification::Switch => &Self::SWITCH_SPACE,
            Specification::Keyword(options) => options,
        }
    }

    pub(crate) fn value_to_string(&self, value: Value) -> String {
        match (self, value) {
            (
                Specification::Integer {
                    space: space::Integer::Sequence(_, _),
                    condition: _,
                },
                Value::Integer(x),
            ) => x.to_string(),
            (
                Specification::Integer {
                    space: space::Integer::Candidates(candidates),
                    condition: _,
                },
                Value::Index(i),
            ) => candidates[i].to_string(),
            (Specification::Switch, Value::Switch(x)) => x.to_string(),
            (Specification::Keyword(space::Keyword(options)), Value::Index(i)) => {
                options[i].clone()
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn string_to_value(&self, s: &str) -> Value {
        match self {
            Specification::Integer {
                space: space::Integer::Sequence(_, _),
                condition: _,
            } => Value::Integer(s.parse().unwrap()),
            Specification::Integer {
                space: space::Integer::Candidates(candidates),
                condition: _,
            } => {
                let v: u32 = s.parse().unwrap();
                Value::Index(
                    candidates
                        .iter()
                        .position(|&candidate| candidate == v)
                        .unwrap(),
                )
            }
            Specification::Switch => Value::Switch(s.parse().unwrap()),
            Specification::Keyword(space::Keyword(options)) => {
                Value::Index(options.iter().position(|option| option == s).unwrap())
            }
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct Profile(pub(crate) BTreeMap<Arc<str>, Arc<Specification>>);

impl Profile {
    fn adjust_by(&self, name: &str, combination: &mut Combination) {
        match self.0[name].as_ref() {
            Specification::Integer {
                space,
                condition: Some(condition),
            } => {
                for dependence in condition.get_dependences() {
                    self.adjust_by(dependence, combination);
                }
                condition.adjust(name, combination);
                space.adjust(combination.get_mut(name).unwrap());
            }
            _ => {}
        }
    }

    pub(crate) fn adjust(&self, individual: &mut Individual) {
        for name in self.0.keys() {
            self.adjust_by(name, &mut individual.parameters);
        }
    }

    pub(crate) fn individual_to_string(&self, individual: &Individual) -> String {
        individual
            .parameters
            .par_iter()
            .map(|(name, &value)| format!("{}={}", name, self.0[name].value_to_string(value)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(crate) fn string_to_individual(&self, s: &str) -> Individual {
        let parameters = s
            .split(", ")
            .map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let name = parts.next().unwrap();
                let value = parts.next().unwrap();
                (name, value)
            })
            .collect::<BTreeMap<&str, &str>>();
        let parameters = self
            .0
            .iter()
            .map(|(name, spec)| {
                let value = if let Some(&value) = parameters.get(name.as_ref()) {
                    spec.string_to_value(value)
                } else {
                    eprintln!("warning: parameter '{}' is missing", name);
                    spec.get_space().random()
                };
                (name.clone(), value)
            })
            .collect::<BTreeMap<Arc<str>, Value>>();
        Individual::new(parameters)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Value {
    Integer(u32),
    Switch(bool),
    Index(usize),
}

pub(crate) type Combination = BTreeMap<Arc<str>, Value>;

pub(crate) trait IntoJson {
    fn into_json(self, profile: &Profile) -> serde_json::Value;
}
