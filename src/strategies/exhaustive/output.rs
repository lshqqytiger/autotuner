use crate::{
    direction::Direction,
    individual::Individual,
    parameter::Profile,
    strategies::{
        execution_log::IntoLog, exhaustive::execution_result::ExecutionResult, output::IntoJson,
    },
};
use std::{cmp, collections::BinaryHeap};

pub(crate) struct Ranking {
    heap: Heap,
    capacity: usize,
}

impl Ranking {
    pub(crate) fn new(direction: Direction, capacity: usize) -> Self {
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
}

impl IntoJson for Ranking {
    fn into_json(self, profile: &Profile) -> serde_json::Value {
        let mut vec = self
            .heap
            .into_iter()
            .map(|result| result.into_log(profile))
            .collect::<Vec<_>>();
        vec.reverse();
        serde_json::to_value(vec).unwrap()
    }
}

pub(crate) enum Heap {
    Min(BinaryHeap<cmp::Reverse<ExecutionResult>>),
    Max(BinaryHeap<ExecutionResult>),
}

impl Heap {
    pub(crate) fn min() -> Self {
        Heap::Min(BinaryHeap::new())
    }

    pub(crate) fn max() -> Self {
        Heap::Max(BinaryHeap::new())
    }

    #[inline]
    pub(crate) fn push(&mut self, value: ExecutionResult) {
        match self {
            Heap::Min(heap) => heap.push(cmp::Reverse(value)),
            Heap::Max(heap) => heap.push(value),
        }
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> Option<ExecutionResult> {
        match self {
            Heap::Min(heap) => heap.pop().map(|rev| rev.0),
            Heap::Max(heap) => heap.pop(),
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        match self {
            Heap::Min(heap) => heap.len(),
            Heap::Max(heap) => heap.len(),
        }
    }

    #[inline]
    pub(crate) fn into_iter<'a>(self) -> Box<dyn Iterator<Item = ExecutionResult> + 'a> {
        match self {
            Heap::Min(heap) => Box::new(heap.into_iter().map(|rev| rev.0)),
            Heap::Max(heap) => Box::new(heap.into_iter()),
        }
    }
}
