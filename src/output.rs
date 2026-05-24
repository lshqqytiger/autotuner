use crate::{
    direction::Direction,
    genetic::GenerationSummary,
    individual::{Fitness, Individual},
    parameter::{IntoJson, Profile},
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
        serialized.insert("history".to_string(), self.history.into_json(profile));
        serde_json::Value::Object(serialized)
    }
}

pub(crate) struct Ranking {
    direction: Direction,
    capacity: usize,
    data: FxHashMap<Arc<str>, Individual>,
}

impl Ranking {
    pub(crate) fn new(direction: Direction, capacity: usize) -> Self {
        Ranking {
            direction,
            capacity,
            data: FxHashMap::default(),
        }
    }

    fn better(&self, lhs: f64, rhs: f64) -> bool {
        self.direction.compare(lhs, rhs).is_gt()
    }

    fn worst_key(&self) -> Option<Arc<str>> {
        self.data
            .iter()
            .filter_map(|(id, individual)| match individual.fitness {
                Fitness::Valid(fitness) => Some((id, fitness)),
                _ => None,
            })
            .min_by(|(_, lhs), (_, rhs)| self.direction.compare(*lhs, *rhs))
            .map(|(id, _)| id.clone())
    }

    pub(crate) fn push(&mut self, individual: &Individual) {
        if self.capacity == 0 {
            return;
        }

        let direction = self.direction;

        let fitness = match individual.fitness {
            Fitness::Valid(fitness) => fitness,
            _ => return,
        };

        if let Some(current) = self.data.get_mut(individual.id.as_ref()) {
            if let Fitness::Valid(current_fitness) = current.fitness {
                if direction.compare(fitness, current_fitness).is_gt() {
                    *current = individual.clone();
                }
            }
            return;
        }

        if self.data.len() < self.capacity {
            self.data.insert(individual.id.clone(), individual.clone());
            return;
        }

        if let Some(worst_id) = self.worst_key() {
            let worst_fitness = match self.data.get(worst_id.as_ref()) {
                Some(Individual {
                    fitness: Fitness::Valid(fitness),
                    ..
                }) => *fitness,
                _ => return,
            };

            if self.better(fitness, worst_fitness) {
                self.data.remove(worst_id.as_ref());
                self.data.insert(individual.id.clone(), individual.clone());
            }
        }
    }

    pub(crate) fn best(&self) -> Option<&Individual> {
        self.data
            .values()
            .filter_map(|individual| match individual.fitness {
                Fitness::Valid(fitness) => Some((individual, fitness)),
                _ => None,
            })
            .max_by(|(_, lhs), (_, rhs)| self.direction.compare(*lhs, *rhs))
            .map(|(individual, _)| individual)
    }
}

impl IntoJson for Ranking {
    fn into_json(self, profile: &Profile) -> serde_json::Value {
        let mut ranking = self
            .data
            .into_values()
            .filter_map(|individual| match individual.fitness {
                Fitness::Valid(fitness) => {
                    Some((profile.individual_to_string(&individual), fitness))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        ranking.sort_by(|&(_, lhs), &(_, rhs)| self.direction.compare(rhs, lhs));

        serde_json::to_value(ranking).unwrap()
    }
}
