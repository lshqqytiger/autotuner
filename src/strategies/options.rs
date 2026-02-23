use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Usize {
    pub(crate) value: usize,
    #[serde(default)]
    scaler: Option<Scaler>,
}

impl From<usize> for Usize {
    fn from(value: usize) -> Self {
        Usize {
            value,
            scaler: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Real {
    pub(crate) value: f64,
    #[serde(default)]
    scaler: Option<Scaler>,
}

impl From<f64> for Real {
    fn from(value: f64) -> Self {
        Real {
            value,
            scaler: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum Scaler {
    Exponential(f64),
}

pub(crate) trait Step {
    fn step(&mut self);
}

impl Step for Usize {
    fn step(&mut self) {
        match self.scaler {
            Some(Scaler::Exponential(factor)) => {
                self.value = ((self.value as f64) * factor) as usize;
            }
            None => {}
        }
    }
}

impl Step for Real {
    fn step(&mut self) {
        match self.scaler {
            Some(Scaler::Exponential(factor)) => {
                self.value *= factor;
            }
            None => {}
        }
    }
}
