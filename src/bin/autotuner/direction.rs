use crate::execution_result::ExecutionResult;

pub(crate) enum Direction {
    Minimize,
    Maximize,
}

impl Direction {
    pub(crate) fn best(&self, iter: impl Iterator<Item = f64>) -> f64 {
        match self {
            Direction::Minimize => iter.fold(f64::INFINITY, |a, b| a.min(b)),
            Direction::Maximize => iter.fold(f64::NEG_INFINITY, |a, b| a.max(b)),
        }
    }

    pub(crate) fn worst(&self, iter: impl Iterator<Item = f64>) -> f64 {
        match self {
            Direction::Minimize => iter.fold(f64::NEG_INFINITY, |a, b| a.max(b)),
            Direction::Maximize => iter.fold(f64::INFINITY, |a, b| a.min(b)),
        }
    }

    pub(crate) fn sort(&self, results: &mut Vec<ExecutionResult>) {
        match self {
            Direction::Minimize => results.sort_by(|a, b| a.1.total_cmp(&b.1)),
            Direction::Maximize => results.sort_by(|a, b| b.1.total_cmp(&a.1)),
        }
    }
}
