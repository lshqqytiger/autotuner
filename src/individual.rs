use crate::{
    parameter::{Combination, Profile, Value},
    utils::interner::Intern,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    hash::{self, Hash},
    sync::Arc,
};

#[derive(Clone)]
pub(crate) struct Individual {
    pub(crate) id: Arc<str>,
    pub(crate) parameters: Combination,
}

impl Hash for Individual {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Individual {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Individual {}

impl Serialize for Individual {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.parameters.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Individual {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized = BTreeMap::<String, Value>::deserialize(deserializer)?;
        let mut parameters = BTreeMap::new();
        for (name, code) in deserialized {
            parameters.insert(name.intern(), code);
        }
        Ok(Individual::new(parameters))
    }
}

impl Individual {
    pub(crate) fn new(parameters: BTreeMap<Arc<str>, Value>) -> Self {
        Individual {
            id: Sha256::digest(serde_json::to_vec(&parameters).unwrap())
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
                .intern(),
            parameters,
        }
    }

    pub(crate) fn random(profile: &Profile) -> Self {
        let mut individual = Self::new(
            profile
                .0
                .iter()
                .map(|(name, parameter)| (name.clone(), parameter.get_space().random()))
                .collect::<BTreeMap<Arc<str>, Value>>(),
        );
        profile.adjust(&mut individual);
        individual
    }
}
