use crate::{
    execution_log::{ExecutionLog, IntoLogs},
    parameter::{Individual, Profile},
};
use std::cmp;

pub(crate) struct ExecutionResult(pub(crate) Individual, pub(crate) f64);

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
        ExecutionLog(profile.stringify(&self.0), self.1)
    }
}

impl IntoLogs for Vec<ExecutionResult> {
    fn into_logs(self, profile: &Profile) -> Vec<ExecutionLog> {
        self.into_iter()
            .map(|result| result.into_log(profile))
            .collect()
    }
}
