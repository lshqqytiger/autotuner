use crate::execution_result::ExecutionResult;
use argh::FromArgValue;

pub(crate) enum Direction {
    Minimize,
    Maximize,
}

impl FromArgValue for Direction {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "minimize" => Ok(Direction::Minimize),
            "maximize" => Ok(Direction::Maximize),
            _ => Err(format!("Invalid direction: {}", value)),
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
