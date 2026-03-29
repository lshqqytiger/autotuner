use crate::{direction::Direction, individual::Individual, parameter::Profile};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub(crate) struct ExecutionLog(pub(crate) String, pub(crate) f64);

pub(crate) trait SortBy {
    fn sort_by_direction(&mut self, direction: Direction);
}

impl SortBy for Vec<ExecutionLog> {
    fn sort_by_direction(&mut self, direction: Direction) {
        self.sort_by(|a, b| direction.compare(b.1, a.1));
    }
}

pub(crate) trait Log {
    fn log(&self, profile: &Profile) -> ExecutionLog;
}

impl Log for (Arc<Individual>, f64) {
    fn log(&self, profile: &Profile) -> ExecutionLog {
        let (individual, value) = self;
        ExecutionLog(profile.stringify(individual), *value)
    }
}

pub(crate) trait IntoLog {
    fn into_log(self, profile: &Profile) -> ExecutionLog;
}
