use crate::execution_result::ExecutionResult;
use serde::{Deserialize, Serialize};

pub(crate) enum Direction {
    Minimize,
    Maximize,
}

impl Serialize for Direction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            Direction::Minimize => "minimize",
            Direction::Maximize => "maximize",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Direction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "minimize" => Ok(Direction::Minimize),
            "maximize" => Ok(Direction::Maximize),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["minimize", "maximize"],
            )),
        }
    }
}

impl Direction {
    pub(crate) fn minmax(&self, iter: impl Iterator<Item = f64>) -> (f64, f64) {
        let (min, max) = iter.fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
            (min.min(v), max.max(v))
        });
        match self {
            Direction::Minimize => (min, max),
            Direction::Maximize => (max, min),
        }
    }

    pub(crate) fn sort(&self, results: &mut Vec<ExecutionResult>) {
        match self {
            Direction::Minimize => results.sort_by(|a, b| a.1.total_cmp(&b.1)),
            Direction::Maximize => results.sort_by(|a, b| b.1.total_cmp(&a.1)),
        }
    }
}
