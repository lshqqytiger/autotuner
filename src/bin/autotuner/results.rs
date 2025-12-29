use argh::FromArgValue;
use autotuner::parameter::Instance;
use std::{cmp, collections::BinaryHeap, result, sync::Arc};

pub(crate) enum Direction {
    Minimize,
    Maximize,
}

impl FromArgValue for Direction {
    fn from_arg_value(value: &str) -> result::Result<Self, String> {
        match value.to_lowercase().as_str() {
            "minimize" => Ok(Direction::Minimize),
            "maximize" => Ok(Direction::Maximize),
            _ => Err(format!("Invalid direction: {}", value)),
        }
    }
}

pub(crate) struct Result(pub(crate) Arc<Instance>, pub(crate) f64);

impl PartialEq for Result {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for Result {}

impl PartialOrd for Result {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for Result {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1.total_cmp(&other.1)
    }
}

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

    fn push(&mut self, value: T) {
        match self {
            Heap::Min(heap) => heap.push(value),
            Heap::Max(heap) => heap.push(cmp::Reverse(value)),
        }
    }

    fn pop(&mut self) -> Option<T> {
        match self {
            Heap::Min(heap) => heap.pop(),
            Heap::Max(heap) => heap.pop().map(|rev| rev.0),
        }
    }

    fn len(&self) -> usize {
        match self {
            Heap::Min(heap) => heap.len(),
            Heap::Max(heap) => heap.len(),
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self {
            Heap::Min(heap) => Box::new(heap.iter()),
            Heap::Max(heap) => Box::new(heap.iter().map(|rev| &rev.0)),
        }
    }
}

pub(crate) struct Results {
    heap: Heap<Result>,
    size: usize,
}

impl Results {
    pub(crate) fn new(direction: &Direction, size: usize) -> Self {
        Results {
            heap: Heap::new(direction),
            size,
        }
    }

    pub(crate) fn push(&mut self, instance: Arc<Instance>, fitness: f64) {
        let result = Result(instance, fitness);
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

    pub(crate) fn best(&self) -> Option<&Result> {
        self.iter().min()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Result> {
        self.heap.iter()
    }
}
