use crate::parameter::Instance;
use serde::Serialize;
use std::cmp;
use std::sync::Arc;

#[derive(Serialize, Clone)]
pub(crate) struct ExecutionResult(pub(crate) Arc<Instance>, pub(crate) f64);

impl PartialEq for ExecutionResult {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for ExecutionResult {}

impl PartialOrd for ExecutionResult {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for ExecutionResult {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1.total_cmp(&other.1)
    }
}
