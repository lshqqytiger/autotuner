use crate::parameter::Profile;
use serde::Serialize;

pub(crate) trait IntoLogs {
    fn into_logs(self, profile: &Profile) -> Vec<ExecutionLog>;
}

#[derive(Serialize)]
pub(crate) struct ExecutionLog(pub(crate) String, pub(crate) f64);
