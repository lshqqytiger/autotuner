use crate::{
    direction::{Direction, Sort},
    execution_log::{ExecutionLog, IntoLogs},
    parameter::{Instance, Profile},
};
use std::cmp;

pub(crate) struct ExecutionResult(pub(crate) Instance, pub(crate) f64);

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

impl Sort<ExecutionResult> for Direction {
    fn sort(&self, results: &mut Vec<ExecutionResult>) {
        match self {
            Direction::Minimize => results.sort_by(|a, b| a.1.total_cmp(&b.1)),
            Direction::Maximize => results.sort_by(|a, b| b.1.total_cmp(&a.1)),
        }
    }
}

impl IntoLogs for Vec<ExecutionResult> {
    fn into_logs(self, profile: &Profile) -> Vec<ExecutionLog> {
        self.into_iter()
            .map(|result| result.into_log(profile))
            .collect()
    }
}
