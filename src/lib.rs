#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![allow(
    clippy::missing_errors_doc,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::significant_drop_tightening,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::option_if_let_else,
    clippy::module_name_repetitions,
    clippy::unused_self,
    clippy::used_underscore_binding
)]

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
