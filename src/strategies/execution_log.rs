use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct ExecutionLog(pub(crate) String, pub(crate) f64);
