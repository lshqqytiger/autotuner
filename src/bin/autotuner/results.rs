use autotuner::parameter::Instance;
use hashlru::Cache;
use std::{
    cmp,
    collections::{BinaryHeap, binary_heap},
    sync::Arc,
};

pub struct Result(Arc<Instance>, f64);

impl Result {
    fn into_tuple(self) -> (Arc<Instance>, f64) {
        (self.0, self.1)
    }
}

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

pub enum Results {
    Cache(Cache<Arc<Instance>, f64>),
    Trace(BinaryHeap<Result>),
}

impl Results {
    pub fn new(cache_size: usize) -> Self {
        if cache_size > 0 {
            Results::Cache(Cache::new(cache_size))
        } else {
            Results::Trace(BinaryHeap::new())
        }
    }

    pub fn get(&mut self, key: &Arc<Instance>) -> Option<&f64> {
        match self {
            Results::Cache(cache) => cache.get(key),
            Results::Trace(_) => None,
        }
    }

    pub fn insert(&mut self, key: Arc<Instance>, value: f64) {
        match self {
            Results::Cache(cache) => {
                cache.insert(key, value);
            }
            Results::Trace(trace) => {
                trace.push(Result(key, value));
            }
        }
    }
}

impl IntoIterator for Results {
    type Item = (Arc<Instance>, f64);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Results::Cache(cache) => IntoIter::Cache(cache.into_iter()),
            Results::Trace(trace) => IntoIter::Trace(trace.into_iter()),
        }
    }
}

pub enum IntoIter {
    Cache(hashlru::IntoIter<Arc<Instance>, f64>),
    Trace(binary_heap::IntoIter<Result>),
}

impl Iterator for IntoIter {
    type Item = (Arc<Instance>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IntoIter::Cache(iter) => iter.next(),
            IntoIter::Trace(iter) => iter.next().map(Result::into_tuple),
        }
    }
}
