use serde::{Deserialize, Serialize};

pub(crate) mod exhaustive;
pub(crate) mod genetic;

#[derive(Serialize, Deserialize)]
pub(crate) enum Checkpoint {
    Exhaustive(exhaustive::State),
    Genetic(genetic::State),
}

impl From<exhaustive::State> for Checkpoint {
    fn from(state: exhaustive::State) -> Self {
        Checkpoint::Exhaustive(state)
    }
}

impl From<genetic::State> for Checkpoint {
    fn from(state: genetic::State) -> Self {
        Checkpoint::Genetic(state)
    }
}
