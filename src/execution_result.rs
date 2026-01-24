use crate::parameter::{Instance, Profile};
use serde::Serialize;
use std::cmp;
use std::sync::Arc;

#[derive(Clone)]
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

impl ExecutionResult {
    pub(crate) fn into_log(self, profile: &Profile) -> ExecutionLog {
        ExecutionLog(profile.display(&self.0), self.1)
    }
}

pub(crate) trait IntoLogs {
    fn into_logs(self, profile: &Profile) -> Vec<ExecutionLog>;
}

impl IntoLogs for Vec<ExecutionResult> {
    fn into_logs(self, profile: &Profile) -> Vec<ExecutionLog> {
        self.into_iter()
            .map(|result| result.into_log(profile))
            .collect()
    }
}

#[derive(Serialize)]
pub(crate) struct ExecutionLog(pub(crate) String, pub(crate) f64);
