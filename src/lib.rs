#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! cdd-engine core library

/// Configuration module
pub mod config;
/// Daemon module
pub mod daemon;
/// Error module
pub mod error;
/// GraalVM WASM Linker mock state
pub mod graalvm_linker;
/// WASM execution orchestration
pub mod wasm_executor;

pub use config::AppConfig;
pub use daemon::{ProcessConfig, ProcessManager};
pub use error::CddEngineError;
/// MCP module
pub mod mcp;
