# cdd-engine

[![CI](https://github.com/SamuelMarks/cdd-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/SamuelMarks/cdd-engine/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Test Coverage](https://img.shields.io/badge/coverage-74%25-success.svg)](https://github.com/SamuelMarks/cdd-engine/actions)
[![Doc Coverage](https://img.shields.io/badge/docs-74%25-success.svg)](https://github.com/SamuelMarks/cdd-engine/actions)

The core execution engine for the `cdd-*` toolchain. 
This crate provides the Tokio-based daemon manager for running native subprocesses and the Wasmtime-based execution environment for securely running `.wasm` generators.

## Overview

`cdd-engine` is decoupled from the `cdd-gateway` REST API and the [cdd-web-ui](https://github.com/SamuelMarks/cdd-web-ui). It serves strictly as the execution orchestrator, capturing `stdout`/`stderr`, managing lifecycle backoff policies, and providing WASM sandbox boundaries.

## Ecosystem Architecture

The `cdd` ecosystem is powered by a distributed microservice architecture. `cdd-engine` operates as the Generator in this setup:

| Repository                                                              | Role        | Description                                                                                        |
| ----------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------- |
| [`cdd-web-ui`](https://github.com/SamuelMarks/cdd-web-ui)               | Frontend    | The central control plane dashboard and UI for managing organizations, repositories, and releases. |
| [`cdd-control-plane`](https://github.com/SamuelMarks/cdd-control-plane) | Backend API | Manages Database, Auth, RBAC, organizations, and secrets.                                          |
| [`cdd-engine`](https://github.com/SamuelMarks/cdd-engine)               | Generator   | Core code generation, WASI orchestration, and AST transformations.                                 |
| [`cdd-gateway`](https://github.com/SamuelMarks/cdd-gateway)             | Ingress     | API Gateway, reverse proxy, and routing.                                                           |
| [`cdd-publisher`](https://github.com/SamuelMarks/cdd-publisher)         | Worker      | Background worker for secure SDK releases to package registries.                                   |
| [`cdd-storage`](https://github.com/SamuelMarks/cdd-storage)             | Storage     | High-performance blob storage for JSON schemas and SDK zip artifacts.                              |
| [`cdd-docs-ui`](https://github.com/SamuelMarks/cdd-docs-ui)             | Frontend    | Dynamic API documentation viewer rendered for published endpoints.                                 |

When used locally with `cdd-web-ui`, `cdd-engine` can be run in JSON-RPC mode to handle code generation using either WASM or native toolchains directly from the browser interface.

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
