use crate::results::Results;
use autotuner::parameter::{Code, Instance};
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub(crate) struct SavedState {
    pub(crate) i: usize,
    pub(crate) instances: Vec<FxHashMap<Arc<str>, Code>>,
    pub(crate) results: Vec<(FxHashMap<Arc<str>, Code>, f64)>,
}

impl SavedState {
    pub(crate) fn new(i: usize, instances: &Vec<Arc<Instance>>, results: &Results) -> Self {
        let instances = instances
            .iter()
            .map(|x| x.parameters.clone())
            .collect::<Vec<_>>();
        let results = results
            .iter()
            .filter_map(|(instance, fitness)| {
                if fitness.is_infinite() {
                    None
                } else {
                    Some((instance.parameters.clone(), *fitness))
                }
            })
            .collect::<Vec<_>>();
        SavedState {
            i,
            instances,
            results,
        }
    }
}
