use crate::{
    parameter::{Individual, Profile},
    strategies::execution_log::ExecutionLog,
};
use std::{cmp, rc::Rc};

#[derive(Clone)]
pub(crate) struct ExecutionResult(pub(crate) Rc<Individual>, pub(crate) f64);

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
    pub(crate) fn log(&self, profile: &Profile) -> ExecutionLog {
        ExecutionLog(profile.stringify(&self.0), self.1)
    }
}
