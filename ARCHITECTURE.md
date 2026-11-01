# cdd-engine Architecture

> This document details the internal technical architecture of the Daemon Manager and WebAssembly execution engine of the CDD ecosystem.

`cdd-engine` serves as the core execution orchestrator for the multi-language `cdd-*` toolchain. It provides a highly concurrent, reliable foundation for managing the execution of 13+ distinct language SDKs and components, either as native subprocesses or sandboxed WebAssembly (WASM) modules.

As part of the broader microservice architecture (interacting with `cdd-control-plane`, `cdd-gateway`, and `cdd-web-ui`), `cdd-engine` focuses strictly on execution, logging, and lifecycle management.

## High-Level Diagram

```ascii
                      +-----------------------------+
                      |   cdd-web-ui (Local Mode)   |
                      |   or cdd-control-plane      |
                      +-------------+---------------+
                                    | (JSON-RPC)
                                    v
                      +-------------+---------------+
                      |        cdd-engine           |
                      | (Daemon & Wasm Orchestrator)|
                      +----+-------------------+----+
                           |                   |
        +------------------+                   +-----------------------+
        | (Lifecycle Events)                                           | (wasmtime calls)
        v                                                              v
+-------+---------+                                          +---------+---------+
| Daemon Manager  |                                          | WASM Executor     |
| (tokio tasks)   |                                          | (wasmtime / WASI) |
+-------+---------+                                          +---------+---------+
        |                                                              |
        | (Spawns & Tracks)                                            | (Evaluates)
        v                                                              v
+------------------------------------+               +-----------------------------------+
|      cdd-* JSON-RPC Servers        |               |      cdd-* .wasm modules          |
| (Native Python, Rust, Go binaries) |               | (Sandboxed generation payloads)   |
+------------------------------------+               +-----------------------------------+
```

## Core Subsystems

### 1. The Daemon Manager (`src/daemon.rs`)

Because the ecosystem consists of diverse technology stacks (Python, Java, Go, Rust, Zig, C++, etc.), `cdd-engine` acts as an agnostic process supervisor. Built fully on Tokio's async runtime, it serves as an embedded daemon supervisor.

- **Concurrency:** Spawns distinct tasks for each monitored process, allowing non-blocking I/O handling.
- **I/O Standardizing:** Captures `stdout` and `stderr` from the RPC servers, tagging and logging lines securely via the unified `log` crate.
- **Resilience:** Implements auto-restart backoffs, tracking uptime to distinguish between persistent crashes and sporadic failures.
- **Graceful Shutdown:** Subscribes all processes to cleanly cascade termination signals across the entire language-server fleet when the engine stops.

### 2. The WASM Execution Engine (`src/wasm_executor.rs`)

To support purely offline and sandboxed generation (e.g., when requested directly by the `cdd-web-ui`), `cdd-engine` uses `wasmtime` to evaluate `.wasm` builds of the supported `cdd-*` ecosystems within a robust, multi-tenant sandbox.

- **WASI Integration:** Provides virtualized filesystem access and standard streams mapping.
- **GraalVM Linker (`src/graalvm_linker.rs`):** Specifically handles complex linking requirements for Java/JVM-based generators compiled via GraalVM to WASM.

### 3. Model Context Protocol, Multi-Spec Detection & Compiler Options (`src/mcp.rs`)

Provides strongly typed integration with the Model Context Protocol (`RequestId`, `TargetLanguage`, `McpRequest`, `McpResponse`, `CallToolParams`, `GenerateSdkArguments`, `CallToolResult`) along with deterministic specification format routing and batch repository layout partitioning:

- **Multi-Specification Format Detection (`SpecFormat`)**: Inspects JSON or YAML input documents to identify **OpenAPI 3.0/3.1/3.2** (`SpecFormat::OpenApi3`), **Swagger 2.0** (`SpecFormat::Swagger2`), **Arazzo 1.0/1.1.0** (`SpecFormat::Arazzo1`), and **Google API Discovery v1** (`SpecFormat::GoogleDiscovery1`).
- **Automatic Google Discovery Pre-Conversion**: When a `SpecFormat::GoogleDiscovery1` document is submitted for a target other than `cdd-cpp` (or for `GeneratorTarget::ToDocsJson`), `cdd-engine` automatically invokes `cdd-cpp` (`from_google_discovery to_openapi`) first—either in-memory via `cdd-cpp.wasm` or over MCP via the `cdd-cpp` daemon—and forwards the resulting OpenAPI 3.2.0 specification to the target compiler.
- **Strongly Typed Compiler Options (`CompilerOptions` & `GeneratorTarget`)**: Encapsulates `target` (`to_sdk`, `to_sdk_cli`, `to_server`, `to_docs_json`, `to_openapi`), `--mcp`, `--framework`, `--orm`, `--auto-admin`, `--tests`, `--no-github-actions`, `--no-installable-package`, `--no-imports`, and `--no-wrapping`.
- **Monorepo vs. Polyrepo Output Partitioning (`RepoLayoutConfig` & `RepoLayoutMode`)**: Partitions multi-language batch generation outputs into either `<org>-sdk/<lang>/<file>` (`Monorepo`) or `<org>-sdk-<lang>/<file>` (`Polyrepo`).

## Configuration

Configurations are handled via the `config` crate (`src/config.rs`), allowing flexible deployment whether run locally by a developer using `cdd-web-ui` or hosted centrally as part of the CDD backend infrastructure.
