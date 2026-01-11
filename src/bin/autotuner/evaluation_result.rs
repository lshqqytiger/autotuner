use autotuner::parameter::Instance;
use std::cmp;
use std::sync::Arc;

pub(crate) struct EvaluationResult(pub(crate) Arc<Instance>, pub(crate) f64);

impl PartialEq for EvaluationResult {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for EvaluationResult {}

impl PartialOrd for EvaluationResult {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for EvaluationResult {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1.total_cmp(&other.1)
    }
}
