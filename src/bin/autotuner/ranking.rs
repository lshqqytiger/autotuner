use crate::{direction::Direction, execution_result::ExecutionResult};
use autotuner::parameter::Instance;
use std::{cmp, collections::BinaryHeap, sync::Arc};

enum Heap<T: Ord> {
    Min(BinaryHeap<T>),
    Max(BinaryHeap<cmp::Reverse<T>>),
}

impl<T: Ord> Heap<T> {
    fn new(direction: &Direction) -> Self {
        match direction {
            Direction::Minimize => Heap::Min(BinaryHeap::new()),
            Direction::Maximize => Heap::Max(BinaryHeap::new()),
        }
    }

    #[inline]
    fn push(&mut self, value: T) {
        match self {
            Heap::Min(heap) => heap.push(value),
            Heap::Max(heap) => heap.push(cmp::Reverse(value)),
        }
    }

    #[inline]
    fn pop(&mut self) -> Option<T> {
        match self {
            Heap::Min(heap) => heap.pop(),
            Heap::Max(heap) => heap.pop().map(|rev| rev.0),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        match self {
            Heap::Min(heap) => heap.len(),
            Heap::Max(heap) => heap.len(),
        }
    }

    #[inline]
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self {
            Heap::Min(heap) => Box::new(heap.iter()),
            Heap::Max(heap) => Box::new(heap.iter().map(|rev| &rev.0)),
        }
    }

    #[inline]
    fn into_iter<'a>(self) -> Box<dyn Iterator<Item = T> + 'a>
    where
        T: 'a,
    {
        match self {
            Heap::Min(heap) => Box::new(heap.into_iter()),
            Heap::Max(heap) => Box::new(heap.into_iter().map(|rev| rev.0)),
        }
    }
}

pub(crate) struct Ranking {
    heap: Heap<ExecutionResult>,
    size: usize,
}

impl Ranking {
    pub(crate) fn new(direction: &Direction, size: usize) -> Self {
        Ranking {
            heap: Heap::new(direction),
            size,
        }
    }

    pub(crate) fn push(&mut self, instance: Arc<Instance>, fitness: f64) {
        let result = ExecutionResult(instance, fitness);
        if self.heap.len() < self.size {
            self.heap.push(result);
        } else {
            match self.heap.pop() {
                Some(top) => {
                    if fitness < top.1 {
                        self.heap.push(result);
                    } else {
                        self.heap.push(top);
                    }
                }
                None => {}
            }
        }
    }

    #[inline]
    pub(crate) fn best(&self) -> Option<&ExecutionResult> {
        self.iter().min()
    }

    #[inline]
    fn iter(&self) -> impl Iterator<Item = &ExecutionResult> {
        self.heap.iter()
    }

    #[inline]
    fn into_iter<'a>(self) -> impl Iterator<Item = ExecutionResult> + 'a
    where
        ExecutionResult: 'a,
    {
        self.heap.into_iter()
    }

    pub(crate) fn to_vec(self) -> Vec<ExecutionResult> {
        self.into_iter().collect()
    }
}
