use crate::{
    direction::Direction,
    parameter::{Individual, Profile},
    strategies::{exhaustive::execution_result::ExecutionResult, heap::Heap},
};

pub(crate) struct Ranking {
    heap: Heap<ExecutionResult>,
    capacity: usize,
}

impl Ranking {
    pub(crate) fn new(direction: &Direction, capacity: usize) -> Self {
        Ranking {
            heap: match direction {
                Direction::Minimize => Heap::max(),
                Direction::Maximize => Heap::min(),
            },
            capacity,
        }
    }

    pub(crate) fn push(&mut self, individual: Individual, fitness: f64) {
        let result = ExecutionResult(individual, fitness);
        self.heap.push(result);
        if self.heap.len() > self.capacity {
            self.heap.pop();
        }
    }

    pub(crate) fn into_json(self, profile: &Profile) -> serde_json::Value {
        let mut vec = self
            .heap
            .into_iter()
            .map(|result| result.into_log(profile))
            .collect::<Vec<_>>();
        vec.reverse();
        serde_json::to_value(vec).unwrap()
    }
}
