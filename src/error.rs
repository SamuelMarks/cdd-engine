#![deny(missing_docs)]

//! Error handling module for `cdd-engine`.
//!
//! Defines the single unified [`CddEngineError`] enum using `derive_more` (`Display`, `Error`, `From`)
//! along with deterministic JSON-RPC 2.0 error code mappings.

use derive_more::{Display, Error, From};

/// Standard JSON-RPC 2.0 parse error code (`-32700`).
pub const JSONRPC_PARSE_ERROR: i32 = -32700;
/// Standard JSON-RPC 2.0 invalid request error code (`-32600`).
pub const JSONRPC_INVALID_REQUEST: i32 = -32600;
/// Standard JSON-RPC 2.0 method not found error code (`-32601`).
pub const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
/// Standard JSON-RPC 2.0 invalid method parameters error code (`-32602`).
pub const JSONRPC_INVALID_PARAMS: i32 = -32602;
/// Standard JSON-RPC 2.0 internal error code (`-32603`).
pub const JSONRPC_INTERNAL_ERROR: i32 = -32603;
/// Server-defined JSON-RPC error code for process or WASM execution failures (`-32000`).
pub const JSONRPC_EXECUTION_ERROR: i32 = -32000;

/// The central Error enum for `cdd-engine`.
#[derive(Debug, Display, Error, From)]
pub enum CddEngineError {
    /// Standard I/O error.
    #[display("I/O Error: {_0}")]
    Io(std::io::Error),

    /// JSON serialization or deserialization error.
    #[display("JSON Error: {_0}")]
    Json(serde_json::Error),

    /// Structured configuration loading error from the `config` crate.
    #[display("Configuration Error: {_0}")]
    ConfigLoad(config::ConfigError),

    /// Domain configuration error message.
    #[display("Configuration Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Config(String),

    /// Error related to WASM module loading or execution.
    #[display("WASM Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Wasm(String),

    /// System command execution failure.
    #[display("System Command Failed: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Command(String),

    /// Failure to spawn or communicate with a language daemon process.
    #[display("Process Spawn Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    ProcessSpawn(String),

    /// A violation of the Model Context Protocol (MCP) specification.
    #[display("MCP Protocol Violation: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    ProtocolViolation(String),

    /// General MCP orchestration error.
    #[display("MCP Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Mcp(String),

    /// Channel receive error when awaiting an asynchronous MCP response.
    #[display("MCP Channel Receive Error: {_0}")]
    Recv(tokio::sync::oneshot::error::RecvError),

    /// Data or parameter validation error.
    #[display("Validation Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Validation(String),

    /// Unrecognized or malformed input specification (Swagger 2.0, `OpenAPI` 3.x, Arazzo 1.x, or Google Discovery).
    #[display("Invalid Specification: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    InvalidSpecification(String),

    /// UTF-8 string conversion error when reading from WASM memory or process output.
    #[display("UTF-8 Validation Error: {_0}")]
    Utf8(std::string::FromUtf8Error),

    /// Integer conversion error when validating pointers or lengths.
    #[display("Integer Conversion Error: {_0}")]
    TryFromInt(std::num::TryFromIntError),

    /// Requested resource or daemon target not found.
    #[display("Not Found: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    NotFound(String),

    /// Internal server or synchronization error.
    #[display("Internal Server Error: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    Internal(String),

    /// Error originating from the `wasmtime` engine.
    #[display("Wasmtime Error: {_0}")]
    #[error(ignore)]
    Wasmtime(wasmtime::Error),

    /// Error accessing linear memory inside a `wasmtime` guest.
    #[display("Wasmtime Memory Access Error: {_0}")]
    MemoryAccess(wasmtime::MemoryAccessError),

    /// Error originating from `QuickJS` execution.
    #[display("Quickjs Error: {_0}")]
    Quickjs(rquickjs::Error),
}

impl CddEngineError {
    /// Maps this error variant to a deterministic JSON-RPC 2.0 error code.
    ///
    /// # Returns
    ///
    /// An `i32` representing the standard or server-defined JSON-RPC 2.0 error code.
    #[must_use]
    pub const fn jsonrpc_error_code(&self) -> i32 {
        match self {
            Self::Json(_) => JSONRPC_PARSE_ERROR,
            Self::ProtocolViolation(_) => JSONRPC_INVALID_REQUEST,
            Self::NotFound(_) => JSONRPC_METHOD_NOT_FOUND,
            Self::Validation(_)
            | Self::InvalidSpecification(_)
            | Self::Utf8(_)
            | Self::TryFromInt(_) => JSONRPC_INVALID_PARAMS,
            Self::Wasm(_)
            | Self::Command(_)
            | Self::ProcessSpawn(_)
            | Self::Wasmtime(_)
            | Self::MemoryAccess(_)
            | Self::Quickjs(_) => JSONRPC_EXECUTION_ERROR,
            Self::Io(_)
            | Self::ConfigLoad(_)
            | Self::Config(_)
            | Self::Mcp(_)
            | Self::Recv(_)
            | Self::Internal(_) => JSONRPC_INTERNAL_ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`CddEngineError`] conversions, display formatting, and JSON-RPC error codes.

    use super::*;

    /// Verifies that all structured `From` conversions and string variants produce the expected
    /// display output and JSON-RPC 2.0 error codes.
    #[test]
    fn test_error_conversions_and_codes() {
        let io_err = CddEngineError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing file",
        ));
        assert!(io_err.to_string().contains("I/O Error: missing file"));
        assert_eq!(io_err.jsonrpc_error_code(), JSONRPC_INTERNAL_ERROR);

        let json_res: Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str("{invalid");
        if let Err(raw_json_err) = json_res {
            let json_err = CddEngineError::from(raw_json_err);
            assert!(json_err.to_string().starts_with("JSON Error:"));
            assert_eq!(json_err.jsonrpc_error_code(), JSONRPC_PARSE_ERROR);
        }

        let cfg_load_err = CddEngineError::from(config::ConfigError::NotFound("key".into()));
        assert!(cfg_load_err.to_string().contains("Configuration Error:"));
        assert_eq!(cfg_load_err.jsonrpc_error_code(), JSONRPC_INTERNAL_ERROR);

        let cfg_err = CddEngineError::Config("bad config".into());
        assert_eq!(cfg_err.to_string(), "Configuration Error: bad config");
        assert_eq!(cfg_err.jsonrpc_error_code(), JSONRPC_INTERNAL_ERROR);

        let wasm_err = CddEngineError::Wasm("bad module".into());
        assert_eq!(wasm_err.to_string(), "WASM Error: bad module");
        assert_eq!(wasm_err.jsonrpc_error_code(), JSONRPC_EXECUTION_ERROR);

        let cmd_err = CddEngineError::Command("exit 1".into());
        assert_eq!(cmd_err.to_string(), "System Command Failed: exit 1");
        assert_eq!(cmd_err.jsonrpc_error_code(), JSONRPC_EXECUTION_ERROR);

        let spawn_err = CddEngineError::ProcessSpawn("no bin".into());
        assert_eq!(spawn_err.to_string(), "Process Spawn Error: no bin");
        assert_eq!(spawn_err.jsonrpc_error_code(), JSONRPC_EXECUTION_ERROR);

        let proto_err = CddEngineError::ProtocolViolation("bad version".into());
        assert_eq!(proto_err.to_string(), "MCP Protocol Violation: bad version");
        assert_eq!(proto_err.jsonrpc_error_code(), JSONRPC_INVALID_REQUEST);

        let mcp_err = CddEngineError::Mcp("orchestrator failure".into());
        assert_eq!(mcp_err.to_string(), "MCP Error: orchestrator failure");
        assert_eq!(mcp_err.jsonrpc_error_code(), JSONRPC_INTERNAL_ERROR);

        let val_err = CddEngineError::Validation("missing param".into());
        assert_eq!(val_err.to_string(), "Validation Error: missing param");
        assert_eq!(val_err.jsonrpc_error_code(), JSONRPC_INVALID_PARAMS);

        let spec_err = CddEngineError::InvalidSpecification("unknown format".into());
        assert_eq!(
            spec_err.to_string(),
            "Invalid Specification: unknown format"
        );
        assert_eq!(spec_err.jsonrpc_error_code(), JSONRPC_INVALID_PARAMS);

        let utf8_res = String::from_utf8(vec![0xff, 0xfe]);
        if let Err(raw_utf8) = utf8_res {
            let utf8_err = CddEngineError::from(raw_utf8);
            assert!(utf8_err.to_string().starts_with("UTF-8 Validation Error:"));
            assert_eq!(utf8_err.jsonrpc_error_code(), JSONRPC_INVALID_PARAMS);
        }

        let int_res = usize::try_from(-1_i32);
        if let Err(raw_int) = int_res {
            let int_err = CddEngineError::from(raw_int);
            assert!(int_err.to_string().starts_with("Integer Conversion Error:"));
            assert_eq!(int_err.jsonrpc_error_code(), JSONRPC_INVALID_PARAMS);
        }

        let not_found_err = CddEngineError::NotFound("cdd-rust".into());
        assert_eq!(not_found_err.to_string(), "Not Found: cdd-rust");
        assert_eq!(not_found_err.jsonrpc_error_code(), JSONRPC_METHOD_NOT_FOUND);

        let internal_err = CddEngineError::Internal("invariant".into());
        assert_eq!(internal_err.to_string(), "Internal Server Error: invariant");
        assert_eq!(internal_err.jsonrpc_error_code(), JSONRPC_INTERNAL_ERROR);

        let wasmtime_err = CddEngineError::from(wasmtime::Error::msg("trap"));
        assert_eq!(wasmtime_err.to_string(), "Wasmtime Error: trap");
        assert_eq!(wasmtime_err.jsonrpc_error_code(), JSONRPC_EXECUTION_ERROR);

        let qjs_err = CddEngineError::from(rquickjs::Error::Unknown);
        assert!(qjs_err.to_string().starts_with("Quickjs Error:"));
        assert_eq!(qjs_err.jsonrpc_error_code(), JSONRPC_EXECUTION_ERROR);
    }

    /// Verifies that a closed `oneshot` channel converts into [`CddEngineError::Recv`] and maps to
    /// [`JSONRPC_INTERNAL_ERROR`].
    #[tokio::test]
    async fn test_recv_error() {
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        drop(sender);
        let recv_res = receiver.await;
        assert!(recv_res.is_err());
        if let Err(raw_err) = recv_res {
            let engine_err = CddEngineError::from(raw_err);
            assert!(engine_err
                .to_string()
                .starts_with("MCP Channel Receive Error:"));
            assert_eq!(engine_err.jsonrpc_error_code(), JSONRPC_INTERNAL_ERROR);
        }
    }
}
