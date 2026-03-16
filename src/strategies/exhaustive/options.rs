use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Options {
    pub(crate) repeat: usize,
    #[serde(default)]
    pub(crate) iterative: bool,
}
