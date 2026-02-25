use fxhash::FxHashSet;
use lazy_static::lazy_static;
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

lazy_static! {
    static ref INTERNER: Mutex<Interner> = Mutex::new(Interner(FxHashSet::default()));
}

struct Interner(FxHashSet<Arc<str>>);

impl Interner {
    fn intern(&mut self, raw: &str) -> Arc<str> {
        if let Some(interned) = self.0.get(raw) {
            interned.clone()
        } else {
            let arc: Arc<str> = Arc::from(raw);
            self.0.insert(arc.clone());
            arc
        }
    }
}

pub(crate) trait Intern {
    fn intern(&self) -> Arc<str>;
}

impl Intern for String {
    fn intern(&self) -> Arc<str> {
        let interner = &mut *INTERNER.lock().unwrap();
        interner.intern(self)
    }
}

impl<'a> Intern for Cow<'a, str> {
    fn intern(&self) -> Arc<str> {
        let interner = &mut *INTERNER.lock().unwrap();
        interner.intern(self)
    }
}
