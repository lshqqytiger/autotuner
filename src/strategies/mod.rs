use serde::{Deserialize, Serialize};

pub(crate) mod genetic;

pub(crate) mod options;
pub(crate) mod output;

#[derive(Deserialize)]
pub(crate) enum Strategy {
    Genetic(genetic::options::Options),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Checkpoint {
    Genetic(genetic::state::State),
}

impl From<genetic::state::State> for Checkpoint {
    fn from(state: genetic::state::State) -> Self {
        Checkpoint::Genetic(state)
    }
}
