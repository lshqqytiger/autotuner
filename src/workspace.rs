use std::ffi;

use fxhash::FxHashMap;

pub(crate) struct Workspace<'s>(pub(crate) FxHashMap<&'s str, *mut ffi::c_void>);

impl<'s> Workspace<'s> {
    pub(crate) fn new() -> Self {
        Workspace(FxHashMap::default())
    }
}

unsafe impl<'s> Sync for Workspace<'s> {}
