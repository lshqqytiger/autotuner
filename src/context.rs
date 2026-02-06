use crate::{
    criterion::Criterion,
    parameter::{Instance, Profile},
};
use anyhow::anyhow;

pub(crate) enum Result {
    Valid(f64),
    Invalid,
    Unknown,
}

impl Result {
    pub(crate) fn anyhow(&self, criterion: &Criterion) -> anyhow::Result<f64> {
        match self {
            Result::Valid(x) => Ok(*x),
            Result::Invalid => Ok(criterion.invalid()),
            Result::Unknown => Err(anyhow!("No result returned")),
        }
    }
}

pub(crate) struct Context<'a> {
    pub(crate) profile: &'a Profile,
    pub(crate) instance: &'a Instance,
    pub(crate) temp_dir: &'a [u8],
    pub(crate) arguments: Vec<String>,
    pub(crate) result: Result,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        profile: &'a Profile,
        instance: &'a Instance,
        temp_dir: &'a [u8],
    ) -> Context<'a> {
        Context {
            profile,
            instance,
            temp_dir,
            arguments: Vec::new(),
            result: Result::Unknown,
        }
    }
}
