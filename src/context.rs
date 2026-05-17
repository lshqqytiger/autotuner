use crate::{individual::Individual, parameter::Profile};

pub(crate) struct Context<'a> {
    pub(crate) profile: &'a Profile,
    pub(crate) individual: &'a mut Individual,
    pub(crate) working_directory: &'a [u8],
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        profile: &'a Profile,
        individual: &'a mut Individual,
        working_directory: &'a [u8],
    ) -> Context<'a> {
        Context {
            profile,
            individual,
            working_directory,
        }
    }
}
