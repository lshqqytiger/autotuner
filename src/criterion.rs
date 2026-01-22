use argh::FromArgValue;

pub(crate) enum Criterion {
    Maximum,
    Minimum,
    Median,
}

impl FromArgValue for Criterion {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "maximum" => Ok(Criterion::Maximum),
            "minimum" => Ok(Criterion::Minimum),
            "median" => Ok(Criterion::Median),
            _ => Err(format!("Invalid criterion: {}", value)),
        }
    }
}

impl Criterion {
    pub(crate) fn extract_representative(&self, mut values: Vec<f64>) -> f64 {
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
