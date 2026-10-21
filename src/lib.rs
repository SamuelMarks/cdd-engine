#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::missing_panics_doc)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//! `cdd-engine` core execution library.
//!
//! This crate provides the asynchronous daemon manager for external JSON-RPC / Model Context
//! Protocol (MCP) language servers (`cdd-*`), strongly typed MCP protocol structures, a
//! `GraalVM` WebAssembly host linker, and an embedded `wasmtime` / `rquickjs` sandboxed
//! execution engine.

/// Configuration loading and validation for the engine and managed processes.
pub mod config;
/// Asynchronous daemon lifecycle supervisor and stdio JSON-RPC / MCP bridge.
pub mod daemon;
/// Unified error definitions and JSON-RPC error code mappings for `cdd-engine`.
pub mod error;
/// `GraalVM` WebAssembly host import bindings and virtual heap state.
pub mod graalvm_linker;
/// Model Context Protocol (MCP) JSON-RPC 2.0 domain types and orchestrator trait.
pub mod mcp;
/// WebAssembly (WASI and `QuickJS`) execution orchestration and file extraction.
pub mod wasm_executor;

pub use config::AppConfig;
pub use daemon::{ProcessConfig, ProcessManager};
pub use error::CddEngineError;
pub use mcp::{McpOrchestrator, McpRequest, McpResponse, RequestId, TargetLanguage};
pub use wasm_executor::{GeneratedFile, NativeWasmExecutor, WasmExecutor};
