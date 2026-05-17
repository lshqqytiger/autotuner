use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
enum UsizeScaler {
    Linear { factor: isize, limit: usize },
    Exponential(f64),
}

impl UsizeScaler {
    pub(crate) fn next(&self, value: usize) -> usize {
        match self {
            UsizeScaler::Linear { factor, limit } => {
                let value = ((value as isize) + factor) as usize;
                if *factor > 0 && value >= *limit {
                    *limit
                } else if *factor < 0 && value <= *limit {
                    *limit
                } else {
                    value
                }
            }
            UsizeScaler::Exponential(factor) => ((value as f64) * factor) as usize,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Usize {
    pub(crate) value: usize,
    #[serde(default)]
    scaler: Option<UsizeScaler>,
}

impl From<usize> for Usize {
    fn from(value: usize) -> Self {
        Usize {
            value,
            scaler: None,
        }
    }
}

impl Usize {
    pub(crate) fn step(&mut self) {
        if let Some(scaler) = &self.scaler {
            self.value = scaler.next(self.value);
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum RealScaler {
    Linear { factor: f64, limit: f64 },
    Exponential(f64),
}

impl RealScaler {
    pub(crate) fn next(&self, value: f64) -> f64 {
        match self {
            RealScaler::Linear { factor, limit } => {
                let value = value + factor;
                if *factor > 0.0 && value >= *limit {
                    *limit
                } else if *factor < 0.0 && value <= *limit {
                    *limit
                } else {
                    value
                }
            }
            RealScaler::Exponential(factor) => value * factor,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Real {
    pub(crate) value: f64,
    #[serde(default)]
    scaler: Option<RealScaler>,
}

impl From<f64> for Real {
    fn from(value: f64) -> Self {
        Real {
            value,
            scaler: None,
        }
    }
}

impl Real {
    pub(crate) fn step(&mut self) {
        if let Some(scaler) = &self.scaler {
            self.value = scaler.next(self.value);
        }
    }
}
