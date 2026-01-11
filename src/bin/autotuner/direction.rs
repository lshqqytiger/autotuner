pub(crate) enum Direction {
    Minimize,
    Maximize,
}

impl Direction {
    pub(crate) fn best(&self, iter: impl Iterator<Item = f64>) -> f64 {
        match self {
            Direction::Minimize => iter.fold(f64::INFINITY, |a, b| a.min(b)),
            Direction::Maximize => iter.fold(f64::NEG_INFINITY, |a, b| a.max(b)),
        }
    }

    pub(crate) fn worst(&self, iter: impl Iterator<Item = f64>) -> f64 {
        match self {
            Direction::Minimize => iter.fold(f64::NEG_INFINITY, |a, b| a.max(b)),
            Direction::Maximize => iter.fold(f64::INFINITY, |a, b| a.min(b)),
        }
    }
}
