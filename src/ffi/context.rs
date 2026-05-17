use crate::{Autotuner, individual::Individual};

pub(crate) struct Context<'a> {
    pub(crate) inner: &'a Autotuner<'a>,
    pub(crate) individual: &'a mut Individual,
}

impl<'a> Context<'a> {
    pub(crate) fn new(inner: &'a Autotuner<'a>, individual: &'a mut Individual) -> Context<'a> {
        Context { inner, individual }
    }
}
