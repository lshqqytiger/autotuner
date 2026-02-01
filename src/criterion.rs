use serde::{Deserialize, Serialize};

pub(crate) enum Criterion {
    Maximum,
    Minimum,
    Median,
}

impl Serialize for Criterion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            Criterion::Maximum => "maximum",
            Criterion::Minimum => "minimum",
            Criterion::Median => "median",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Criterion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "maximum" => Ok(Criterion::Maximum),
            "minimum" => Ok(Criterion::Minimum),
            "median" => Ok(Criterion::Median),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["maximum", "minimum", "median"],
            )),
        }
    }
}

impl Criterion {
    pub(crate) fn invalid(&self) -> f64 {
        match self {
            Criterion::Maximum => f64::NEG_INFINITY,
            Criterion::Minimum => f64::INFINITY,
            Criterion::Median => f64::INFINITY,
        }
    }

    pub(crate) fn representative(&self, mut values: Vec<f64>) -> f64 {
        match self {
            Criterion::Maximum => values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b)),
            Criterion::Minimum => values.iter().fold(f64::INFINITY, |a, b| a.min(*b)),
            Criterion::Median => {
                values.sort_by(|a, b| a.total_cmp(b));
                values[values.len() / 2]
            }
        }
    }
}
