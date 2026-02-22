use serde::{Deserialize, Serialize};
use std::cmp;

pub(crate) trait Sort<T> {
    fn sort(&self, results: &mut Vec<T>);
}

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
    pub(crate) fn worst(&self) -> f64 {
        match self {
            Direction::Minimize => f64::INFINITY,
            Direction::Maximize => f64::NEG_INFINITY,
        }
    }

    pub(crate) fn compare(&self, a: f64, b: f64) -> cmp::Ordering {
        match self {
            Direction::Minimize => b.total_cmp(&a),
            Direction::Maximize => a.total_cmp(&b),
        }
    }

    pub(crate) fn boundaries(&self, iter: impl Iterator<Item = f64>) -> (f64, f64) {
        let (min, max) = iter.fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
            (min.min(v), max.max(v))
        });
        match self {
            Direction::Minimize => (min, max),
            Direction::Maximize => (max, min),
        }
    }
}

impl Sort<(f64, usize)> for Direction {
    fn sort(&self, results: &mut Vec<(f64, usize)>) {
        results.sort_by(|a, b| self.compare(a.0, b.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare() {
        assert_eq!(
            Direction::Minimize.compare(1.0, 2.0),
            cmp::Ordering::Greater
        );
        assert_eq!(Direction::Minimize.compare(2.0, 1.0), cmp::Ordering::Less);
        assert_eq!(Direction::Minimize.compare(1.0, 1.0), cmp::Ordering::Equal);

        assert_eq!(Direction::Maximize.compare(1.0, 2.0), cmp::Ordering::Less);
        assert_eq!(
            Direction::Maximize.compare(2.0, 1.0),
            cmp::Ordering::Greater
        );
        assert_eq!(Direction::Maximize.compare(1.0, 1.0), cmp::Ordering::Equal);
    }
}
