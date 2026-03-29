mod condition;

pub(crate) mod space;

use crate::individual::Individual;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) trait Space {
    fn first(&self) -> Value;
    fn random(&self) -> Value;
    fn next(&self, current: Value) -> Option<Value>;
    fn adjust(&self, _: &mut Value) {}
    fn len(&self) -> usize;
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

    pub(crate) fn stringify(&self, value: Value) -> String {
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

    pub(crate) fn stringify(&self, individual: &Individual) -> String {
        individual
            .parameters
            .par_iter()
            .map(|(name, &value)| format!("{}={}", name, self.0[name].stringify(value)))
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Value {
    Integer(u32),
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

pub(crate) type Combination = BTreeMap<Arc<str>, Value>;
