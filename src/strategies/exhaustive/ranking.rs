use crate::{
    direction::Direction, heap::Heap, parameter::Individual,
    strategies::exhaustive::execution_result::ExecutionResult,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_ranking() {
        let mut ranking = Ranking::new(&Direction::Minimize, 3);
        ranking.push(Individual::new(BTreeMap::new()), 1.0);
        ranking.push(Individual::new(BTreeMap::new()), 2.0);
        ranking.push(Individual::new(BTreeMap::new()), 3.0);
        ranking.push(Individual::new(BTreeMap::new()), 0.5);
        ranking.push(Individual::new(BTreeMap::new()), 4.0);

        let mut results = ranking.to_vec();
        results.reverse();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1, 0.5);
        assert_eq!(results[1].1, 1.0);
        assert_eq!(results[2].1, 2.0);
    }
}
