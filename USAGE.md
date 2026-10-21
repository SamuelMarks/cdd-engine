# Usage Guide for `cdd-engine`

> This document details how to embed and configure `cdd-engine` as a Rust crate for supervising `cdd-*` MCP daemons and executing sandboxed WebAssembly / `QuickJS` generators.

---

## 1. Embedding `cdd-engine` in `Cargo.toml`

```toml
[dependencies]
cdd-engine = { git = "https://github.com/SamuelMarks/cdd-engine" }
```

---

## 2. Configuration (`AppConfig`)

`AppConfig` loads settings from built-in defaults, an optional TOML/JSON/YAML file, and `CDD__*` environment variables:

```rust
use cdd_engine::AppConfig;

let config = AppConfig::load(Some("cdd.toml"))?;
println!("Binding to: {}", config.server_bind);
```

### Supported Environment Variables
- `CDD__SERVER_BIND`: Local RPC/MCP bind address (default: `127.0.0.1:8084`).
- `CDD__WASM_ASSETS_DIR`: Directory containing `<target>.wasm` binaries (default: `cdd-ctl-wasm-sdk/assets/wasm`).
- `CDD__WASM_SUPPORT_MANIFEST`: Path to `wasm-support.json` (default: `cdd-ctl-wasm-sdk/assets/wasm-support.json`).
- `CDD__MAX_STDIO_BUFFER_BYTES`: Maximum in-memory WASI stdout/stderr buffer in bytes (default: `10485760`).
- `CDD__DEFAULT_REQUEST_TIMEOUT_MS`: Default request timeout in milliseconds (default: `30000`).
- `CDD__OFFLINE_MODE`: Strict offline execution flag (`true` or `false`).

---

## 3. Supervising MCP Daemons (`ProcessManager`)

```rust
use cdd_engine::{McpOrchestrator, McpRequest, ProcessConfig, ProcessManager, RequestId};
use std::collections::HashMap;

let mut servers = HashMap::new();
servers.insert(
    "cdd-rust".to_string(),
    ProcessConfig {
        command: Some("cdd-rust".to_string()),
        args: Some(vec!["--mcp".to_string()]),
        external_address: None,
        max_retries: 3,
        restart_delay_ms: 1000,
        stability_threshold_ms: 5000,
    },
);

let manager = ProcessManager::new(servers);
manager.start_all().await;

let response = manager
    .handle_request(McpRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: Some(RequestId::Number(1)),
    })
    .await?;

manager.stop_all().await;
```

---

## 4. Executing WebAssembly Generators (`NativeWasmExecutor`)

```rust
use cdd_engine::{NativeWasmExecutor, WasmExecutor};

let executor = NativeWasmExecutor::new()?;
let generated_files = executor.execute("rust", r#"{"openapi":"3.1.0"}"#, &[])?;
for file in generated_files {
    println!("Generated {}: {} bytes", file.path, file.content.len());
}
```
