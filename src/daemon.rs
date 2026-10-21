#![deny(missing_docs)]

//! Daemon manager for external JSON-RPC / Model Context Protocol (MCP) servers (`cdd-*` projects).
//!
//! Provides [`ProcessConfig`] and [`ProcessManager`] to supervise long-running child processes,
//! correlate asynchronous newline-delimited JSON-RPC 2.0 messages over `stdin`/`stdout`, capture
//! and log `stderr`, and expose the [`McpOrchestrator`] trait interface.

use crate::config::DEFAULT_REQUEST_TIMEOUT_MS;
use crate::error::CddEngineError;
use crate::mcp::{
    CallToolParams, ImplementationInfo, InitializeResult, ListToolsResult, McpOrchestrator,
    McpRequest, McpResponse, McpTool, RequestId, ServerCapabilities, ToolInputSchema,
    ToolsCapability, MCP_PROTOCOL_VERSION,
};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};

/// Default stability threshold in milliseconds (`5_000` ms) before resetting the retry counter.
pub const DEFAULT_STABILITY_THRESHOLD_MS: u64 = 5_000;

/// Returns the default stability threshold in milliseconds.
const fn default_stability_threshold_ms() -> u64 {
    DEFAULT_STABILITY_THRESHOLD_MS
}

/// Configuration for a single external `cdd-*` daemon process.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProcessConfig {
    /// Executable command or binary path (if managed locally).
    pub command: Option<String>,
    /// Command-line arguments passed when spawning the binary.
    pub args: Option<Vec<String>>,
    /// External address if the server is already running outside the manager (e.g., `"http://localhost:8001"`).
    pub external_address: Option<String>,
    /// Maximum number of restart attempts before giving up (`0` means never restart).
    #[serde(default)]
    pub max_retries: u32,
    /// Delay in milliseconds between restart attempts.
    #[serde(default)]
    pub restart_delay_ms: u64,
    /// Minimum uptime in milliseconds required to reset the crash retry counter to `0`.
    #[serde(default = "default_stability_threshold_ms")]
    pub stability_threshold_ms: u64,
}

/// Type alias for the reply channel used to deliver a correlated [`McpResponse`] or error.
type ReplySender = oneshot::Sender<Result<McpResponse, CddEngineError>>;

/// Type alias for the command channel tuple sent from [`ProcessManager`] to a worker loop.
type DaemonCommand = (McpRequest, ReplySender);

/// Shared map of in-flight JSON-RPC [`RequestId`]s awaiting responses from child `stdout`.
type PendingRequestsMap = Arc<Mutex<HashMap<RequestId, ReplySender>>>;

/// Shared list of active supervisor task handles.
type SupervisorHandlesList = Arc<Mutex<Vec<(String, JoinHandle<()>)>>>;

/// Manages the lifecycle and MCP communication of external `cdd-*` processes.
pub struct ProcessManager {
    /// Configured servers keyed by daemon name.
    configs: HashMap<String, ProcessConfig>,
    /// Active supervisor task handles (`(daemon_name, join_handle)`).
    handles: SupervisorHandlesList,
    /// Active command channels for sending MCP requests to supervised processes.
    active_channels: Arc<Mutex<HashMap<String, mpsc::Sender<DaemonCommand>>>>,
    /// Shutdown broadcast sender.
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    /// Per-request timeout in milliseconds when awaiting a daemon response.
    request_timeout_ms: u64,
}

/// Holds the spawned child process and its extracted standard I/O pipes.
struct SpawnedDaemonPipes {
    /// Running child process handle.
    child: Child,
    /// Pipe connected to child standard input.
    stdin: ChildStdin,
    /// Pipe connected to child standard output.
    stdout: ChildStdout,
    /// Pipe connected to child standard error.
    stderr: ChildStderr,
}

impl ProcessManager {
    /// Creates a new [`ProcessManager`] with the provided daemon configurations.
    ///
    /// # Arguments
    ///
    /// * `configs` - Map of daemon names to their [`ProcessConfig`] settings.
    ///
    /// # Returns
    ///
    /// An initialized [`ProcessManager`].
    #[must_use]
    pub fn new(configs: HashMap<String, ProcessConfig>) -> Self {
        Self::with_timeout(configs, DEFAULT_REQUEST_TIMEOUT_MS)
    }

    /// Creates a new [`ProcessManager`] with a custom per-request timeout in milliseconds.
    ///
    /// # Arguments
    ///
    /// * `configs` - Map of daemon names to their [`ProcessConfig`] settings.
    /// * `request_timeout_ms` - Timeout in milliseconds for awaiting a daemon reply.
    ///
    /// # Returns
    ///
    /// An initialized [`ProcessManager`].
    #[must_use]
    pub fn with_timeout(configs: HashMap<String, ProcessConfig>, request_timeout_ms: u64) -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        Self {
            configs,
            handles: Arc::new(Mutex::new(Vec::new())),
            active_channels: Arc::new(Mutex::new(HashMap::new())),
            shutdown_tx,
            request_timeout_ms,
        }
    }

    /// Returns a read-only reference to the configured daemon map.
    ///
    /// # Returns
    ///
    /// A reference to the underlying `HashMap<String, ProcessConfig>`.
    #[must_use]
    pub const fn configs(&self) -> &HashMap<String, ProcessConfig> {
        &self.configs
    }

    /// Returns the number of currently active supervisor tasks.
    ///
    /// # Returns
    ///
    /// Count of active background supervisor tasks.
    pub async fn active_daemon_count(&self) -> usize {
        self.handles.lock().await.len()
    }

    /// Starts all locally managed processes and registers their MCP communication channels.
    pub async fn start_all(&self) {
        let mut handles = self.handles.lock().await;
        let mut channels = self.active_channels.lock().await;

        for (name, config) in &self.configs {
            if let Some(ext) = &config.external_address {
                info!("Process '{name}' is configured to use external address: {ext}. Skipping local spawn.");
                continue;
            }

            if config.command.is_none() {
                error!(
                    "Process '{name}' has neither 'command' nor 'external_address' configured! Skipping."
                );
                continue;
            }

            let name_clone = name.clone();
            let config_clone = config.clone();
            let shutdown_rx = self.shutdown_tx.subscribe();
            let (cmd_tx, cmd_rx) = mpsc::channel(32);

            channels.insert(name.clone(), cmd_tx);

            let handle = tokio::spawn(async move {
                Self::monitor_process(name_clone, config_clone, shutdown_rx, cmd_rx).await;
            });

            handles.push((name.clone(), handle));
        }
        drop(channels);
        drop(handles);
    }

    /// Spawns a child process with piped `stdin`, `stdout`, and `stderr`.
    fn spawn_child_pipes(
        cmd_str: &str,
        config: &ProcessConfig,
    ) -> Result<SpawnedDaemonPipes, CddEngineError> {
        let mut cmd = Command::new(cmd_str);
        if let Some(args) = &config.args {
            cmd.args(args);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| CddEngineError::ProcessSpawn(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CddEngineError::ProcessSpawn("Child stdin missing".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CddEngineError::ProcessSpawn("Child stdout missing".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CddEngineError::ProcessSpawn("Child stderr missing".to_string()))?;

        Ok(SpawnedDaemonPipes {
            child,
            stdin,
            stdout,
            stderr,
        })
    }

    /// Spawns a task that reads newline-delimited JSON-RPC responses from child `stdout` and
    /// resolves matching entries in `pending_requests`.
    fn spawn_stdout_task(
        daemon_name: String,
        stdout: ChildStdout,
        pending_requests: PendingRequestsMap,
    ) -> JoinHandle<()> {
        let mut stdout_reader = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                match serde_json::from_str::<McpResponse>(&line) {
                    Ok(resp) => {
                        if let Some(req_id) = &resp.id {
                            let sender_opt = pending_requests.lock().await.remove(req_id);
                            if let Some(reply_tx) = sender_opt {
                                let _ = reply_tx.send(Ok(resp));
                            }
                        }
                    }
                    Err(err) => {
                        warn!(
                            "[{daemon_name} stdout] Non-JSON or malformed MCP line ({err}): {line}"
                        );
                    }
                }
            }
        })
    }

    /// Spawns a task that reads and logs lines from child `stderr`.
    fn spawn_stderr_task(daemon_name: String, stderr: ChildStderr) -> JoinHandle<()> {
        let mut stderr_reader = BufReader::new(stderr).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                debug!("[{daemon_name} stderr] {line}");
            }
        })
    }

    /// Fails all currently pending requests when a child process exits or crashes before replying.
    async fn fail_pending_requests(pending_requests: &PendingRequestsMap, reason: &str) {
        let mut guard = pending_requests.lock().await;
        for (_, reply_tx) in guard.drain() {
            let _ = reply_tx.send(Err(CddEngineError::ProcessSpawn(reason.to_string())));
        }
    }

    /// Serializes and writes an [`McpRequest`] to the child's `stdin`, registering `reply_tx` in
    /// `pending_requests` if the request has an `id`.
    async fn forward_request_to_stdin(
        stdin: &mut ChildStdin,
        req: &McpRequest,
        reply_tx: ReplySender,
        pending_requests: &PendingRequestsMap,
    ) -> Result<(), CddEngineError> {
        let mut serialized = serde_json::to_vec(req)?;
        serialized.push(10_u8);

        let Some(req_id) = req.id.clone() else {
            let err =
                CddEngineError::ProtocolViolation("MCP request requires a non-null id".to_string());
            let _ = reply_tx.send(Err(err));
            return Ok(());
        };

        pending_requests
            .lock()
            .await
            .insert(req_id.clone(), reply_tx);
        if let Err(io_err) = stdin.write_all(&serialized).await {
            let removed_tx = pending_requests.lock().await.remove(&req_id);
            if let Some(tx) = removed_tx {
                let _ = tx.send(Err(CddEngineError::Io(io_err)));
            }
            return Err(CddEngineError::ProcessSpawn(
                "Failed writing request to child stdin".to_string(),
            ));
        }
        Ok(())
    }

    /// Supervises a single child process across spawns, crash restarts, and graceful shutdown.
    #[allow(clippy::cognitive_complexity)]
    async fn monitor_process(
        name: String,
        config: ProcessConfig,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
        mut cmd_rx: mpsc::Receiver<DaemonCommand>,
    ) {
        let Some(cmd_str) = config.command.clone() else {
            return;
        };
        let mut retries = 0_u32;

        loop {
            info!("Starting process '{name}' (attempt {})", retries + 1);
            let start_time = Instant::now();

            match Self::spawn_child_pipes(&cmd_str, &config) {
                Ok(mut pipes) => {
                    let pending_requests: PendingRequestsMap = Arc::new(Mutex::new(HashMap::new()));
                    let out_task = Self::spawn_stdout_task(
                        name.clone(),
                        pipes.stdout,
                        pending_requests.clone(),
                    );
                    let err_task = Self::spawn_stderr_task(name.clone(), pipes.stderr);

                    let should_shutdown = loop {
                        tokio::select! {
                            status = pipes.child.wait() => {
                                warn!("Process '{name}' exited with status: {status:?}");
                                break false;
                            }
                            cmd_opt = cmd_rx.recv() => {
                                if let Some((req, reply_tx)) = cmd_opt {
                                    let _ = Self::forward_request_to_stdin(
                                        &mut pipes.stdin,
                                        &req,
                                        reply_tx,
                                        &pending_requests,
                                    )
                                    .await;
                                } else {
                                    break true;
                                }
                            }
                            _ = shutdown_rx.recv() => {
                                info!("Shutdown signal received for '{name}'. Terminating child process...");
                                let _ = pipes.child.kill().await;
                                break true;
                            }
                        }
                    };

                    out_task.abort();
                    err_task.abort();
                    Self::fail_pending_requests(
                        &pending_requests,
                        "Daemon process exited before responding",
                    )
                    .await;

                    if should_shutdown {
                        break;
                    }
                    if start_time.elapsed() > Duration::from_millis(config.stability_threshold_ms) {
                        retries = 0;
                    }
                }
                Err(err) => {
                    error!("Failed to spawn process '{name}': {err}");
                }
            }

            if retries >= config.max_retries {
                error!(
                    "Process '{name}' reached max retries ({}). Giving up.",
                    config.max_retries
                );
                break;
            }

            retries += 1;
            let delay = Duration::from_millis(config.restart_delay_ms);
            info!("Waiting {delay:?} before restarting '{name}'...");

            tokio::select! {
                () = sleep(delay) => {}
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received while waiting to restart '{name}'. Aborting.");
                    break;
                }
            }
        }
    }

    /// Gracefully stops all supervised processes and clears active channels and handles.
    pub async fn stop_all(&self) {
        info!("Stopping all managed processes...");
        let _ = self.shutdown_tx.send(());

        let drained: Vec<_> = self.handles.lock().await.drain(..).collect();
        for (name, handle) in drained {
            debug!("Waiting for process monitor '{name}' to exit...");
            let _ = handle.await;
        }
        self.active_channels.lock().await.clear();
        info!("All managed processes stopped.");
    }

    /// Builds the `tools/list` response listing the `cdd_generate_sdk` MCP tool.
    fn handle_tools_list(id: Option<RequestId>) -> Result<McpResponse, CddEngineError> {
        let tool = McpTool {
            name: "cdd_generate_sdk".to_string(),
            description:
                "Generates an SDK or server stub using a specific cdd-* language toolchain."
                    .to_string(),
            input_schema: ToolInputSchema {
                schema_type: "object".to_string(),
                properties: serde_json::json!({
                    "target_language": {
                        "type": "string",
                        "description": "The target language (e.g., python, rust, go, ts, csharp)."
                    },
                    "input": {
                        "type": "string",
                        "description": "The input OpenAPI specification path or content."
                    },
                    "output": {
                        "type": "string",
                        "description": "The output directory path."
                    }
                }),
                required: vec!["target_language".to_string(), "input".to_string()],
            },
        };
        McpResponse::success(
            id,
            &ListToolsResult {
                tools: vec![tool],
                next_cursor: None,
            },
        )
    }

    /// Builds the `initialize` response advertising the engine's MCP protocol version and capabilities.
    fn handle_initialize(id: Option<RequestId>) -> Result<McpResponse, CddEngineError> {
        let init_result = InitializeResult {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {
                    list_changed: false,
                },
            },
            server_info: ImplementationInfo {
                name: "cdd-engine".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        McpResponse::success(id, &init_result)
    }

    /// Validates and routes a `tools/call` MCP request to the target `cdd-*` daemon.
    async fn handle_tools_call(&self, request: McpRequest) -> Result<McpResponse, CddEngineError> {
        let params_val = request
            .params
            .as_ref()
            .ok_or_else(|| CddEngineError::Validation("Missing params".to_string()))?;
        let call_params: CallToolParams = serde_json::from_value(params_val.clone())
            .map_err(|e| CddEngineError::Validation(format!("Invalid tool call params: {e}")))?;

        let daemon_name = call_params.arguments.target_language.daemon_name();
        let sender = {
            let channels = self.active_channels.lock().await;
            channels.get(&daemon_name).cloned().ok_or_else(|| {
                CddEngineError::NotFound(format!("Daemon {daemon_name} not found or not running"))
            })?
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        sender
            .send((request, reply_tx))
            .await
            .map_err(|_| CddEngineError::Mcp("Failed to send to daemon worker".to_string()))?;

        let timeout_duration = Duration::from_millis(self.request_timeout_ms);
        tokio::time::timeout(timeout_duration, reply_rx)
            .await
            .map_err(|_| {
                CddEngineError::Mcp(format!(
                    "Request to daemon {daemon_name} timed out after {} ms",
                    self.request_timeout_ms
                ))
            })??
    }
}

impl McpOrchestrator for ProcessManager {
    async fn handle_request(&self, request: McpRequest) -> Result<McpResponse, CddEngineError> {
        match request.method.as_str() {
            "initialize" => Self::handle_initialize(request.id),
            "notifications/initialized" | "ping" => {
                McpResponse::success(request.id, &serde_json::json!({}))
            }
            "tools/list" => Self::handle_tools_list(request.id),
            "tools/call" => self.handle_tools_call(request).await,
            other => Err(CddEngineError::ProtocolViolation(format!(
                "Unsupported MCP method: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`ProcessManager`], [`ProcessConfig`], and [`McpOrchestrator`] routing.

    use super::*;
    use crate::mcp::JSONRPC_VERSION;

    /// Helper to build a default [`ProcessConfig`] for tests.
    fn make_process_config(
        command: Option<&str>,
        args: Option<Vec<&str>>,
        max_retries: u32,
        restart_delay_ms: u64,
        stability_threshold_ms: u64,
    ) -> ProcessConfig {
        ProcessConfig {
            command: command.map(str::to_string),
            args: args.map(|list| list.into_iter().map(str::to_string).collect()),
            external_address: None,
            max_retries,
            restart_delay_ms,
            stability_threshold_ms,
        }
    }

    /// Verifies `initialize`, `ping`, `notifications/initialized`, `tools/list`, and unsupported
    /// method handling on [`ProcessManager`].
    #[tokio::test]
    async fn test_mcp_orchestrator_lifecycle_and_tools_list() -> Result<(), CddEngineError> {
        let pm = ProcessManager::new(HashMap::new());
        assert!(pm.configs().is_empty());
        assert_eq!(pm.active_daemon_count().await, 0);

        // 1. initialize
        let init_query = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "initialize".to_string(),
            params: None,
            id: Some(RequestId::Number(1)),
        };
        let init_reply = pm.handle_request(init_query).await?;
        assert_eq!(init_reply.id, Some(RequestId::Number(1)));
        let init_body: InitializeResult = serde_json::from_value(
            init_reply
                .result
                .ok_or_else(|| CddEngineError::Internal("missing result".into()))?,
        )?;
        assert_eq!(init_body.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(init_body.server_info.name, "cdd-engine");

        // 2. ping & notifications/initialized
        let ping_query = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "ping".to_string(),
            params: None,
            id: Some(RequestId::from("ping-1")),
        };
        let ping_reply = pm.handle_request(ping_query).await?;
        assert_eq!(ping_reply.id, Some(RequestId::from("ping-1")));
        assert_eq!(ping_reply.result, Some(serde_json::json!({})));

        // 3. tools/list
        let list_query = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(RequestId::Number(2)),
        };
        let list_reply = pm.handle_request(list_query).await?;
        let tools_body: ListToolsResult = serde_json::from_value(
            list_reply
                .result
                .ok_or_else(|| CddEngineError::Internal("missing result".into()))?,
        )?;
        assert_eq!(tools_body.tools.len(), 1);
        assert_eq!(tools_body.tools[0].name, "cdd_generate_sdk");

        // 4. Unsupported method
        let unknown_query = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "resources/list".to_string(),
            params: None,
            id: Some(RequestId::Number(3)),
        };
        let unknown_reply = pm.handle_request(unknown_query).await;
        assert!(matches!(
            unknown_reply,
            Err(CddEngineError::ProtocolViolation(_))
        ));

        Ok(())
    }

    /// Verifies validation errors and missing daemon errors for `tools/call`.
    #[tokio::test]
    async fn test_mcp_orchestrator_tools_call_validation_and_not_found() {
        let pm = ProcessManager::new(HashMap::new());

        // Missing params
        let req_no_params = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: None,
            id: Some(RequestId::Number(1)),
        };
        assert!(matches!(
            pm.handle_request(req_no_params).await,
            Err(CddEngineError::Validation(_))
        ));

        // Invalid params structure
        let req_bad_params = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"arguments": {}})),
            id: Some(RequestId::Number(2)),
        };
        assert!(matches!(
            pm.handle_request(req_bad_params).await,
            Err(CddEngineError::Validation(_))
        ));

        // Valid params, daemon not registered
        let req_not_found = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "cdd_generate_sdk",
                "arguments": {
                    "target_language": "python",
                    "input": "spec.json"
                }
            })),
            id: Some(RequestId::Number(3)),
        };
        assert!(matches!(
            pm.handle_request(req_not_found).await,
            Err(CddEngineError::NotFound(_))
        ));
    }

    /// Verifies `start_all` skipping behavior for `external_address` and missing `command`, plus
    /// direct `monitor_process` invocation when `command` is `None`.
    #[tokio::test]
    async fn test_start_all_external_and_missing_command() {
        let mut configs = HashMap::new();
        configs.insert(
            "cdd-ext".to_string(),
            ProcessConfig {
                command: None,
                args: None,
                external_address: Some("http://localhost:8001".to_string()),
                max_retries: 0,
                restart_delay_ms: 10,
                stability_threshold_ms: 100,
            },
        );
        configs.insert(
            "cdd-invalid".to_string(),
            make_process_config(None, None, 0, 10, 100),
        );

        let pm = ProcessManager::new(configs);
        pm.start_all().await;
        assert_eq!(pm.active_daemon_count().await, 0);
        pm.stop_all().await;

        let (_tx, rx) = tokio::sync::broadcast::channel(1);
        let (_cmd_tx, cmd_rx) = mpsc::channel(1);
        ProcessManager::monitor_process(
            "none".to_string(),
            make_process_config(None, None, 0, 10, 100),
            rx,
            cmd_rx,
        )
        .await;
    }

    /// Verifies full end-to-end JSON-RPC request/response correlation, malformed stdout lines,
    /// unmatched response IDs, stderr logging, and missing request IDs with a live shell child.
    #[tokio::test]
    async fn test_daemon_live_stdio_communication() -> Result<(), CddEngineError> {
        let mut configs = HashMap::new();
        configs.insert(
            "cdd-python".to_string(),
            make_process_config(
                Some("sh"),
                Some(vec![
                    "-c",
                    r#"echo 'err_msg' >&2; echo 'not_json'; echo '{"jsonrpc":"2.0","result":{},"id":999}'; while read line; do echo '{"jsonrpc":"2.0","result":{"status":"ok"},"id":1}'; done; sleep 10"#,
                ]),
                1,
                20,
                10,
            ),
        );

        let pm = ProcessManager::new(configs);
        pm.start_all().await;
        assert_eq!(pm.active_daemon_count().await, 1);

        // 1. Correlated request with id = 1
        let req = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "cdd_generate_sdk",
                "arguments": {
                    "target_language": "cdd-python",
                    "input": "openapi.json"
                }
            })),
            id: Some(RequestId::Number(1)),
        };
        let res = pm.handle_request(req).await?;
        assert_eq!(res.id, Some(RequestId::Number(1)));
        assert_eq!(res.result, Some(serde_json::json!({"status": "ok"})));

        // 2. Request without id (rejected as ProtocolViolation)
        let req_no_id = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "cdd_generate_sdk",
                "arguments": {
                    "target_language": "python",
                    "input": "openapi.json"
                }
            })),
            id: None,
        };
        let no_id_res = pm.handle_request(req_no_id).await;
        assert!(matches!(
            no_id_res,
            Err(CddEngineError::ProtocolViolation(_))
        ));

        pm.stop_all().await;
        assert_eq!(pm.active_daemon_count().await, 0);
        Ok(())
    }

    /// Verifies retry exhaustion, stability threshold reset, and shutdown during retry delay.
    #[tokio::test]
    async fn test_daemon_retry_and_shutdown_flows() {
        let mut configs = HashMap::new();
        // 1. Process that runs longer than stability_threshold_ms (5ms) so retries reset to 0, then stops via shutdown
        configs.insert(
            "cdd-stable-reset".to_string(),
            make_process_config(Some("sh"), Some(vec!["-c", "sleep 0.02"]), 0, 500, 5),
        );
        // 2. Process that exits faster than stability_threshold_ms (5000ms) and exhausts max_retries = 0
        configs.insert(
            "cdd-fast-exit".to_string(),
            make_process_config(Some("sh"), Some(vec!["-c", "exit 0"]), 0, 10, 5000),
        );
        // 3. Process that fails to spawn and retries once
        configs.insert(
            "cdd-non-existent".to_string(),
            make_process_config(Some("/non_existent_binary_cdd_999"), None, 1, 10, 5000),
        );

        let pm = ProcessManager::new(configs);
        pm.start_all().await;
        sleep(Duration::from_millis(80)).await;
        pm.stop_all().await;

        // Shutdown during restart sleep delay
        let mut delay_configs = HashMap::new();
        delay_configs.insert(
            "cdd-delay-shutdown".to_string(),
            make_process_config(Some("/non_existent_binary_cdd_999"), None, 3, 5000, 5000),
        );
        let pm_delay = ProcessManager::new(delay_configs);
        pm_delay.start_all().await;
        sleep(Duration::from_millis(20)).await;
        pm_delay.stop_all().await;
    }

    /// Verifies that when a child process exits before replying, or when the command channel closes,
    /// deterministic errors are returned.
    #[tokio::test]
    async fn test_daemon_exits_before_response_and_channel_closed() {
        let mut configs = HashMap::new();
        configs.insert(
            "cdd-crash".to_string(),
            make_process_config(
                Some("sh"),
                Some(vec!["-c", "read line; exit 1"]),
                0,
                10,
                5000,
            ),
        );

        let pm = ProcessManager::new(configs);
        pm.start_all().await;

        let req = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "arguments": { "target_language": "crash" }
            })),
            id: Some(RequestId::Number(101)),
        };
        let res = pm.handle_request(req.clone()).await;
        assert!(res.is_err());

        // Wait for monitor_process to finish after max_retries = 0, then send again to hit closed channel
        sleep(Duration::from_millis(40)).await;
        let res_closed = pm.handle_request(req).await;
        assert!(matches!(res_closed, Err(CddEngineError::Mcp(_))));

        pm.stop_all().await;
    }

    /// Verifies per-request timeout when a child daemon hangs and clean exit when
    /// `active_channels` is cleared.
    #[tokio::test]
    async fn test_daemon_cmd_channel_dropped() {
        let mut configs = HashMap::new();
        configs.insert(
            "cdd-sleep".to_string(),
            make_process_config(Some("sh"), Some(vec!["-c", "sleep 5"]), 0, 10, 5000),
        );
        let pm = ProcessManager::with_timeout(configs, 15);
        pm.start_all().await;

        let req = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "arguments": { "target_language": "sleep" }
            })),
            id: Some(RequestId::Number(404)),
        };
        let timeout_res = pm.handle_request(req).await;
        assert!(matches!(timeout_res, Err(CddEngineError::Mcp(_))));

        pm.active_channels.lock().await.clear();
        sleep(Duration::from_millis(20)).await;
        pm.stop_all().await;
    }

    /// Verifies `forward_request_to_stdin` error handling when writing to a closed child `stdin` pipe.
    #[tokio::test]
    async fn test_forward_request_broken_stdin_pipe() -> Result<(), CddEngineError> {
        let cfg = make_process_config(
            Some("sh"),
            Some(vec!["-c", "exec 0<&-; sleep 0.2"]),
            0,
            10,
            5000,
        );
        let mut pipes = ProcessManager::spawn_child_pipes("sh", &cfg)?;
        drop(pipes.stdout);
        drop(pipes.stderr);

        // Give child shell a moment to close its stdin descriptor
        sleep(Duration::from_millis(40)).await;
        let pending: PendingRequestsMap = Arc::new(Mutex::new(HashMap::new()));

        // Write a large payload so the OS pipe buffer overflows and triggers EPIPE immediately
        let large_input = "x".repeat(512 * 1024);
        let req = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({ "data": large_input })),
            id: Some(RequestId::Number(77)),
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        let write_res =
            ProcessManager::forward_request_to_stdin(&mut pipes.stdin, &req, reply_tx, &pending)
                .await;
        assert!(write_res.is_err());
        let reply = reply_rx.await?;
        assert!(matches!(reply, Err(CddEngineError::Io(_))));

        let _ = pipes.child.kill().await;
        Ok(())
    }
}
