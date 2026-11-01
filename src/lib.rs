#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::missing_panics_doc)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(rustdoc::broken_intra_doc_links)]

//! `cdd-engine` core execution library.
//!
//! This crate provides the asynchronous daemon manager for external JSON-RPC / Model Context
//! Protocol (MCP) language servers (`cdd-*`), strongly typed MCP protocol structures, a
//! `GraalVM` WebAssembly host linker, and an embedded `wasmtime` / `rquickjs` sandboxed
//! execution engine.

pub mod config;
pub mod daemon;
pub mod error;
pub mod graalvm_linker;
pub mod mcp;
pub mod wasm_executor;

pub use config::AppConfig;
pub use daemon::{ProcessConfig, ProcessManager};
pub use error::CddEngineError;
pub use mcp::{
    CompilerOptions, GenerateSdkArguments, GeneratorTarget, McpOrchestrator, McpRequest,
    McpResponse, RepoLayoutConfig, RepoLayoutMode, RequestId, SpecFormat, TargetLanguage,
};
pub use wasm_executor::{GeneratedFile, NativeWasmExecutor, WasmExecutor};
