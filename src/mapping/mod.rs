mod ast;
mod parser;

use serde::{Deserialize, Serialize};
use std::str::FromStr;

type Function<T> = unsafe extern "C" fn(x: T) -> T;

#[derive(Clone)]
pub struct Mapping<T>(String, Option<Function<T>>);

impl<T> Mapping<T> {
    pub fn new(mapping: String) -> Self {
        Mapping(mapping, None)
    }

    pub fn map(&self, value: T) -> T {
        if let None = self.1 {
            todo!()
        }

        unsafe { self.1.unwrap()(value) }
    }
}

impl<T> FromStr for Mapping<T> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Mapping::new(s.to_string()))
    }
}

impl<T> ToString for Mapping<T> {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl<T> Serialize for Mapping<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, T> Deserialize<'de> for Mapping<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Mapping::new(s))
    }
}
