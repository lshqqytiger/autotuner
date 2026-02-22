use crate::{
    criterion::Criterion,
    parameter::{Individual, Profile},
};

pub(crate) enum Result {
    Valid(f64),
    Invalid,
    Unknown,
}

impl Result {
    pub(crate) fn unwrap(&self, criterion: &Criterion) -> f64 {
        match self {
            Result::Valid(x) => *x,
            Result::Invalid => criterion.invalid(),
            Result::Unknown => panic!("No result returned"),
        }
    }
}

pub(crate) struct Context<'a> {
    pub(crate) profile: &'a Profile,
    pub(crate) individual: &'a Individual,
    pub(crate) temp_dir: &'a [u8],
    pub(crate) arguments: Vec<String>,
    pub(crate) result: Result,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        profile: &'a Profile,
        individual: &'a Individual,
        temp_dir: &'a [u8],
    ) -> Context<'a> {
        Context {
            profile,
            individual,
            temp_dir,
            arguments: Vec::new(),
            result: Result::Unknown,
        }
    }
}
