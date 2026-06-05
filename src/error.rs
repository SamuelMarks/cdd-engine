#![cfg(not(tarpaulin_include))]
#![allow(missing_docs)]
use derive_more::Display;
use derive_more::Error;

/// The central Error enum for cdd-engine.
#[derive(Debug, Display, Error)]
pub enum CddError {
    #[display("I/O Error: {_0}")]
    Io(std::io::Error),

    #[display("JSON Error: {_0}")]
    Json(serde_json::Error),

    #[display("Configuration Error: {_0}")]
    #[error(ignore)]
    Config(String),

    #[display("WASM Error: {_0}")]
    #[error(ignore)]
    Wasm(String),

    #[display("System Command Failed: {_0}")]
    #[error(ignore)]
    Command(String),

    #[display("Validation Error: {_0}")]
    #[error(ignore)]
    Validation(String),

    #[display("Not Found: {_0}")]
    #[error(ignore)]
    NotFound(String),

    #[display("Internal Server Error: {_0}")]
    #[error(ignore)]
    Internal(String),
}

impl From<std::io::Error> for CddError {
    fn from(err: std::io::Error) -> Self {
        CddError::Io(err)
    }
}

impl From<serde_json::Error> for CddError {
    fn from(err: serde_json::Error) -> Self {
        CddError::Json(err)
    }
}
