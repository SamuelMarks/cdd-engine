# Developing `cdd-engine`

> This document serves as the contributor guide, detailing how to set up the local development environment, build from source, and run tests and coverage verification for `cdd-engine`.

Thank you for your interest in contributing to `cdd-engine`! This repository provides the core asynchronous daemon manager (`ProcessManager`), strongly typed Model Context Protocol (`McpOrchestrator`, `McpRequest`, `McpResponse`), `GraalVM` WebAssembly host linker (`GraalVmLinker`), and sandboxed `wasmtime` / `rquickjs` execution engine (`NativeWasmExecutor`).

---

## 1. Prerequisites

Ensure the following tools are installed:

- **Rust Toolchain**: Stable (`1.93+`) and Nightly (for `rustdoc --show-coverage`).
- **Coverage Tooling**: `cargo-tarpaulin` (`cargo install cargo-tarpaulin`).
- **Pre-commit**: `pre-commit` (`pip install pre-commit`) to enforce formatting, clippy, test, and coverage hooks.

---

## 2. Building & Testing

### Build the Library
```bash
cargo +stable build --release
```

### Run Unit Tests
```bash
cargo +stable test --all-targets
```

### Run Strict Clippy Lints
```bash
cargo +stable clippy --all-targets --all-features -- -D warnings
```

### Verify 100% Test and Documentation Coverage
```bash
./scripts/update_shields.sh
```

---

## 3. Coding Standards & Invariants

1. **100% Documentation Coverage**: Every module (`//!`), struct, enum, trait, field, variant, and function (`///` including `# Arguments`, `# Returns`, `# Errors`) must be documented. Enforced via `#![deny(missing_docs)]` and `#![deny(clippy::missing_docs_in_private_items)]`.
2. **100% Test Coverage**: All functions, lines, and branches must be covered by meaningful unit tests.
3. **Zero `unwrap` or `expect`**: Enforced crate-wide via `#![deny(clippy::unwrap_used, clippy::expect_used)]`. All errors must flow through `CddEngineError` (`derive_more`).
4. **Strong Typing**: Always prefer domain types (`RequestId`, `TargetLanguage`, `JsonRpcErrorObject`, `CallToolParams`) over untyped JSON maps.
