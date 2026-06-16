# cdd-engine

[![CI](https://github.com/SamuelMarks/cdd-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/SamuelMarks/cdd-engine/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Test Coverage](https://img.shields.io/badge/coverage-73%25-success.svg)](https://github.com/SamuelMarks/cdd-engine/actions)
[![Doc Coverage](https://img.shields.io/badge/docs-73%25-success.svg)](https://github.com/SamuelMarks/cdd-engine/actions)

The core execution engine for the `cdd-*` toolchain. 
This crate provides the Tokio-based daemon manager for running native subprocesses and the Wasmtime-based execution environment for securely running `.wasm` generators.

## Overview

`cdd-engine` is decoupled from the `cdd-gateway` REST API. It serves strictly as the execution orchestrator, capturing `stdout`/`stderr`, managing lifecycle backoff policies, and providing WASM sandbox boundaries.

## Usage

Add `cdd-engine` to your `Cargo.toml`:

```toml
[dependencies]
cdd-engine = { git = "https://github.com/SamuelMarks/cdd-engine" }
```

## License

This project is dual-licensed under either of the following, at your option:

- Apache License, Version 2.0 (LICENSE-APACHE or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License (LICENSE-MIT or <https://opensource.org/licenses/MIT>)
