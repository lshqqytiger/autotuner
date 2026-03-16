use crate::{
    direction::Direction,
    parameter::{Individual, Profile},
    strategies::{
        execution_log::{Log, SortBy},
        genetic::GenerationSummary,
        output::IntoJson,
    },
};
use fxhash::FxHashMap;
use std::sync::Arc;

pub(crate) struct Output {
    pub(crate) ranking: Ranking,
    pub(crate) history: Vec<GenerationSummary>,
}

impl Output {
    pub(crate) fn new(direction: Direction, capacity: usize) -> Self {
        Output {
            ranking: Ranking::new(direction, capacity),
            history: Vec::new(),
        }
    }
}

impl IntoJson for Output {
    fn into_json(self, profile: &Profile) -> serde_json::Value {
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
    direction: Direction,
    capacity: usize,
    data: FxHashMap<Arc<Individual>, f64>,
    best: Option<Arc<Individual>>,
    worst: Option<Arc<Individual>>,
}

impl Ranking {
    fn new(direction: Direction, capacity: usize) -> Self {
        Ranking {
            direction,
            capacity,
            data: FxHashMap::default(),
            best: None,
            worst: None,
        }
    }

    pub(crate) fn push(&mut self, individual: Arc<Individual>, fitness: f64) {
        if self.data.is_empty() {
            self.best = Some(individual.clone());
            self.worst = Some(individual.clone());
            self.data.insert(individual, fitness);
            return;
        }

        let worst = self.worst.as_ref().unwrap();
        if self.data.len() == self.capacity
            && self.direction.compare(fitness, self.data[worst]).is_le()
        {
            // reject if the new result is worse than the worst result in the ranking
            return;
        }

        let previous = self.data.get(&individual);
        if let Some(previous) = previous {
            if self.direction.compare(fitness, *previous).is_le() {
                // reject if the new result is worse than the previous result of the same individual
                return;
            }
        }

        let best = self.best.as_ref().unwrap();
        if self.direction.compare(fitness, self.data[best]).is_gt() {
            self.best = Some(individual.clone());
        }
        if self.data.len() == self.capacity {
            self.data.remove(worst);
            self.worst = Some(
                self.data
                    .iter()
                    .min_by(|&(_, &a), &(_, &b)| self.direction.compare(a, b))
                    .unwrap()
                    .0
                    .clone(),
            );
        } else {
            if self.direction.compare(fitness, self.data[worst]).is_lt() {
                self.worst = Some(individual.clone());
            }
        }

        self.data.insert(individual, fitness);
    }

    #[inline]
    pub(crate) fn best(&self) -> Option<(Arc<Individual>, f64)> {
        self.best
            .as_ref()
            .map(|best| (best.clone(), self.data[best]))
    }
}

impl IntoJson for Ranking {
    fn into_json(self, profile: &Profile) -> serde_json::Value {
        let mut vec = self
            .data
            .into_iter()
            .map(|result| result.log(profile))
            .collect::<Vec<_>>();
        vec.sort_by_direction(self.direction);
        serde_json::to_value(vec).unwrap()
    }
}
