use crate::{
    direction::Direction,
    parameter::{Individual, Profile},
    strategies::{
        genetic::{execution_result::ExecutionResult, GenerationSummary},
        heap::Heap,
    },
};
use std::rc::Rc;

pub(crate) struct Output {
    pub(crate) ranking: Ranking,
    pub(crate) history: Vec<GenerationSummary>,
}

impl Output {
    pub(crate) fn new(direction: &Direction, capacity: usize) -> Self {
        Output {
            ranking: Ranking::new(direction, capacity),
            history: Vec::new(),
        }
    }

    pub(crate) fn into_json(self, profile: &Profile) -> serde_json::Value {
        let mut serialized = serde_json::Map::new();
        serialized.insert("ranking".to_string(), self.ranking.into_json(profile));
        serialized.insert(
            "history".to_string(),
            serde_json::to_value(self.history).unwrap(),
        );
        serde_json::Value::Object(serialized)
    }
}

pub(crate) struct Ranking {
    heap: Heap<ExecutionResult>,
    capacity: usize,
}

impl Ranking {
    fn new(direction: &Direction, capacity: usize) -> Self {
        Ranking {
            heap: match direction {
                Direction::Minimize => Heap::max(),
                Direction::Maximize => Heap::min(),
            },
            capacity,
        }
    }

    pub(crate) fn push(&mut self, individual: Rc<Individual>, fitness: f64) {
        let result = ExecutionResult(individual, fitness);
        self.heap.push(result);
        if self.heap.len() > self.capacity {
            self.heap.pop();
        }
    }

    #[inline]
    pub(crate) fn best(&self) -> Option<&ExecutionResult> {
        self.heap.iter().min()
    }

    pub(crate) fn into_json(self, profile: &Profile) -> serde_json::Value {
        let mut vec = self
            .heap
            .into_iter()
            .map(|result| result.log(profile))
            .collect::<Vec<_>>();
        vec.reverse();
        serde_json::to_value(vec).unwrap()
    }
}
