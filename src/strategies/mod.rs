use serde::{Deserialize, Serialize};

pub(crate) mod exhaustive;
pub(crate) mod genetic;

pub(crate) mod options;

mod execution_log;
mod heap;

#[derive(Serialize, Deserialize)]
pub(crate) enum Strategy {
    Exhaustive(exhaustive::options::Options),
    Genetic(genetic::options::Options),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Checkpoint {
    Exhaustive(exhaustive::state::State),
    Genetic(genetic::state::State),
}

impl From<exhaustive::state::State> for Checkpoint {
    fn from(state: exhaustive::state::State) -> Self {
        Checkpoint::Exhaustive(state)
    }
}

impl From<genetic::state::State> for Checkpoint {
    fn from(state: genetic::state::State) -> Self {
        Checkpoint::Genetic(state)
    }
}
