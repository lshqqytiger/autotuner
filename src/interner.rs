use fxhash::FxHashSet;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref INTERNER: Mutex<Interner> = Mutex::new(Interner(FxHashSet::default()));
}

pub struct Interner(FxHashSet<Arc<str>>);

impl Interner {
    pub fn intern(raw: &str) -> Arc<str> {
        let interner = &mut *INTERNER.lock().unwrap();
        interner._intern(raw)
    }

    fn _intern(&mut self, raw: &str) -> Arc<str> {
        if let Some(interned) = self.0.get(raw) {
            interned.clone()
        } else {
            let arc: Arc<str> = Arc::from(raw);
            self.0.insert(arc.clone());
            arc
        }
    }
}
