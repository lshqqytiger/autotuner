use crate::SavedState;
use std::{error, fmt};

pub(crate) enum Error {
    FileNotFound(String),
    InvalidMetadata,
    InvalidSaveFile,
    Saved(SavedState),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FileNotFound(path) => write!(f, "File not found: {}", path),
            Error::InvalidMetadata => write!(f, "Invalid metadata format"),
            Error::InvalidSaveFile => write!(f, "Invalid save file format"),
            Error::Saved(_) => unimplemented!(),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Saved(_) => write!(f, "Saved(...)"),
            _ => fmt::Debug::fmt(self, f),
        }
    }
}
