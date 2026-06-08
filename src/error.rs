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
}
