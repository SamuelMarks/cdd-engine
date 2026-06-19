#![deny(missing_docs)]

//! Error handling module for cdd-engine.

use derive_more::{Display, Error, From};

/// The central Error enum for cdd-engine.
#[derive(Debug, Display, Error, From)]
pub enum CddEngineError {
    /// Standard I/O error.
    #[display("I/O Error: {_0}")]
    Io(std::io::Error),

    /// JSON serialization/deserialization error.
    #[display("JSON Error: {_0}")]
    Json(serde_json::Error),

    /// Error loading or parsing configuration.
    #[display("Configuration Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Config(String),

    /// Error related to WASM execution.
    #[display("WASM Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Wasm(String),

    /// System command execution failure.
    #[display("System Command Failed: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Command(String),

    /// Failure to spawn a language daemon process.
    #[display("Process Spawn Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    ProcessSpawn(String),

    /// A violation of the Model Context Protocol (MCP) spec.
    #[display("MCP Protocol Violation: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    ProtocolViolation(String),

    /// General MCP related error.
    #[display("MCP Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Mcp(String),

    /// Data validation error.
    #[display("Validation Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Validation(String),

    /// Requested resource not found.
    #[display("Not Found: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    NotFound(String),

    /// Internal server error.
    #[display("Internal Server Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Internal(String),

    /// Error originating from the Wasmtime engine.
    #[display("Wasmtime Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Wasmtime(String),

    /// Error originating from QuickJS execution.
    #[display("Quickjs Error: {_0}")]
    Quickjs(rquickjs::Error),
}

impl From<wasmtime::Error> for CddEngineError {
    fn from(e: wasmtime::Error) -> Self {
        CddEngineError::Wasmtime(e.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for CddEngineError {
    fn from(e: std::sync::PoisonError<T>) -> Self {
        CddEngineError::Internal(e.to_string())
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for CddEngineError {
    fn from(e: tokio::sync::oneshot::error::RecvError) -> Self {
        CddEngineError::Mcp(e.to_string())
    }
}

impl From<config::ConfigError> for CddEngineError {
    fn from(e: config::ConfigError) -> Self {
        CddEngineError::Config(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() -> Result<(), CddEngineError> {
        let wasm_err = wasmtime::Error::msg("test");
        let _engine_err: CddEngineError = wasm_err.into();

        let lock_res: Result<std::sync::MutexGuard<'_, ()>, _> =
            Err(std::sync::PoisonError::new(()));
        if let Err(e) = lock_res {
            let _engine_err: CddEngineError = e.into();
        }

        let config_err = config::ConfigError::NotFound("test".into());
        let _engine_err: CddEngineError = config_err.into();
        Ok(())
    }

    #[tokio::test]
    async fn test_recv_error() -> Result<(), CddEngineError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        drop(tx);
        if let Err(e) = rx.await {
            let _engine_err: CddEngineError = e.into();
        }
        Ok(())
    }
}
