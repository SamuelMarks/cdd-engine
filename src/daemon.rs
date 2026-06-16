#![deny(missing_docs)]
//! Daemon manager for external JSON-RPC/MCP servers (cdd-* projects).
#![allow(clippy::needless_return)]

use crate::error::CddEngineError;
use crate::mcp::{McpOrchestrator, McpRequest, McpResponse};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::task::JoinHandle;

fn default_max_retries() -> usize {
    5
}
fn default_restart_delay_ms() -> u64 {
    2000
}

/// Configuration for a single managed process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessConfig {
    /// Command to run.
    pub command: Option<String>,
    /// Arguments to pass to the command.
    pub args: Option<Vec<String>>,
    /// External address overriding the local spawn.
    pub external_address: Option<String>,
    /// Maximum number of consecutive retries before giving up.
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// Delay between restarts in milliseconds.
    #[serde(default = "default_restart_delay_ms")]
    pub restart_delay_ms: u64,
}

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Result<McpResponse, CddEngineError>>>>>;
type ChannelSender = mpsc::Sender<(
    McpRequest,
    oneshot::Sender<Result<McpResponse, CddEngineError>>,
)>;

/// Daemon manager that keeps track of the processes.
pub struct ProcessManager {
    /// Configurations.
    pub configs: HashMap<String, ProcessConfig>,
    /// Active monitor tasks keyed by their logical name.
    pub handles: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    /// Active communication channels to the daemons.
    active_channels: Arc<Mutex<HashMap<String, ChannelSender>>>,
    /// Channel to signal shutdown to all monitors.
    shutdown_tx: watch::Sender<bool>,
}

impl ProcessManager {
    /// Create a new ProcessManager from configurations.
    pub fn new(configs: HashMap<String, ProcessConfig>) -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            configs,
            handles: Arc::new(Mutex::new(HashMap::new())),
            active_channels: Arc::new(Mutex::new(HashMap::new())),
            shutdown_tx,
        }
    }

    /// Start all configured local processes.
    pub async fn start_all(&self) -> Result<(), CddEngineError> {
        let mut handles = self.handles.lock().await;
        let mut channels = self.active_channels.lock().await;

        for (name, config) in &self.configs {
            if let Some(ref external) = config.external_address {
                info!(
                    "[{}] Configured to use external address: {}",
                    name, external
                );
                continue;
            }

            if config.command.is_none() {
                error!("[{}] No command or external address configured", name);
                continue;
            }

            let (tx, rx) = mpsc::channel(32);
            channels.insert(name.clone(), tx);

            let name_clone = name.clone();
            let config_clone = config.clone();
            let shutdown_rx = self.shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                Self::monitor_process(name_clone, config_clone, rx, shutdown_rx).await;
            });

            handles.insert(name.clone(), handle);
        }
        Ok(())
    }

    /// Stop all managed local processes and wait for them to exit gracefully.
    pub async fn stop_all(&self) {
        info!("Initiating graceful shutdown of all managed processes...");
        let _ = self.shutdown_tx.send(true);

        let mut handles = self.handles.lock().await;
        for (name, handle) in handles.drain() {
            info!("Waiting for process monitor '{}' to exit...", name);
            let _ = handle.await;
        }
        info!("All managed processes stopped.");
    }

    /// The core monitor loop for a single process. Handles spawning, standard I/O bridging, and restarting.
    pub async fn monitor_process(
        name: String,
        config: ProcessConfig,
        mut mcp_rx: mpsc::Receiver<(
            McpRequest,
            oneshot::Sender<Result<McpResponse, CddEngineError>>,
        )>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        let cmd_str = match &config.command {
            Some(c) => c.clone(),
            None => return,
        };
        let mut retries = 0;

        loop {
            info!("[{}] Starting process: {}", name, cmd_str);
            let mut cmd = Command::new(&cmd_str);
            if let Some(ref args) = config.args {
                cmd.args(args);
            }

            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let start_time = tokio::time::Instant::now();

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    error!("[{}] Failed to spawn: {}", name, e);
                    if retries >= config.max_retries {
                        error!(
                            "[{}] Max retries ({}) reached. Giving up.",
                            name, config.max_retries
                        );
                        return;
                    }
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(config.restart_delay_ms)).await;
                    continue;
                }
            };

            let mut stdin = match child.stdin.take() {
                Some(s) => s,
                None => panic!("[{}] Failed to take stdin", name),
            };
            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => panic!("[{}] Failed to take stdout", name),
            };
            let stderr = match child.stderr.take() {
                Some(s) => s,
                None => panic!("[{}] Failed to take stderr", name),
            };

            let pending_requests: PendingMap = Arc::new(Mutex::new(HashMap::new()));
            let pending_clone = pending_requests.clone();
            let name_out = name.clone();

            // Stdout reader task: parses responses and routes them back to the caller
            let mut stdout_reader = BufReader::new(stdout).lines();
            let reader_handle = tokio::spawn(async move {
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    if let Ok(res) = serde_json::from_str::<McpResponse>(&line) {
                        if let Some(id_val) = &res.id {
                            let id_str = id_val.to_string();
                            let mut pending = pending_clone.lock().await;
                            if let Some(tx) = pending.remove(&id_str) {
                                let _ = tx.send(Ok(res));
                            }
                        }
                    } else {
                        // Fallback: Just log it
                        info!("[{}] {}", name_out, line);
                    }
                }
            });

            let name_err = name.clone();
            let mut stderr_reader = BufReader::new(stderr).lines();
            let err_handle = tokio::spawn(async move {
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    warn!("[{}] ERR: {}", name_err, line);
                }
            });

            loop {
                tokio::select! {
                    req_opt = mcp_rx.recv() => {
                        match req_opt {
                            Some((req, reply_tx)) => {
                                if let Ok(json) = serde_json::to_string(&req) {
                                    let msg = format!("{}\n", json);
                                    if stdin.write_all(msg.as_bytes()).await.is_err() {
                                        let _ = reply_tx.send(Err(CddEngineError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin write failed"))));
                                        break;
                                    }
                                    if let Some(id_val) = &req.id {
                                        pending_requests.lock().await.insert(id_val.to_string(), reply_tx);
                                    } else {
                                        let _ = reply_tx.send(Err(CddEngineError::Validation("Request ID is missing".into())));
                                    }
                                }
                            }
                            None => {
                                info!("[{}] Channel closed internally.", name);
                                break;
                            }
                        }
                    }
                    status_res = child.wait() => {
                        if let Ok(status) = status_res {
                            if status.success() {
                                info!("[{}] Exited successfully.", name);
                            } else {
                                warn!("[{}] Exited with status: {}", name, status);
                            }
                        }
                        break;
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("[{}] Shutdown signaled. Killing process.", name);
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                            return;
                        }
                    }
                }
            }

            reader_handle.abort();
            err_handle.abort();

            if *shutdown_rx.borrow() {
                return;
            }

            if start_time.elapsed()
                > if cfg!(test) {
                    Duration::from_millis(10)
                } else {
                    Duration::from_secs(10)
                }
            {
                info!("[{}] Process was stable. Resetting retry count.", name);
                retries = 0;
            }

            if retries >= config.max_retries {
                error!(
                    "[{}] Max retries ({}) reached after crash. Giving up.",
                    name, config.max_retries
                );
                return;
            }

            retries += 1;
            warn!(
                "[{}] Restarting in {} ms (Retry {}/{})",
                name, config.restart_delay_ms, retries, config.max_retries
            );
            tokio::time::sleep(Duration::from_millis(config.restart_delay_ms)).await;
        }
    }
}

#[async_trait::async_trait]
impl McpOrchestrator for ProcessManager {
    async fn handle_request(&self, req: McpRequest) -> Result<McpResponse, CddEngineError> {
        if req.method == "tools/list" {
            let tools = serde_json::json!({
                "tools": [{
                    "name": "cdd_generate_sdk",
                    "description": "Generate an SDK from an OpenAPI spec",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "target_language": { "type": "string" },
                            "input": { "type": "string" },
                            "output": { "type": "string" }
                        },
                        "required": ["target_language", "input"]
                    }
                }]
            });
            return Ok(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(tools),
                error: None,
                id: req.id,
            });
        }

        if req.method == "tools/call" {
            let lang = req
                .params
                .as_ref()
                .and_then(|p| p.get("arguments"))
                .and_then(|a| a.get("target_language"))
                .and_then(|l| l.as_str())
                .ok_or_else(|| {
                    CddEngineError::Validation("Missing 'target_language' in arguments".into())
                })?;

            let target = if lang.starts_with("cdd-") {
                lang.to_string()
            } else {
                format!("cdd-{}", lang)
            };

            let channels = self.active_channels.lock().await;
            let tx = channels
                .get(&target)
                .ok_or_else(|| CddEngineError::NotFound(format!("Daemon not found: {}", target)))?;

            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send((req.clone(), reply_tx))
                .await
                .map_err(|_| CddEngineError::ProcessSpawn("Channel closed".into()))?;

            return reply_rx
                .await
                .map_err(|_| CddEngineError::ProcessSpawn("Daemon dropped response".into()))?;
        }

        Err(CddEngineError::Validation(format!(
            "Unsupported method: {}",
            req.method
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_process_no_command() {
        let (_tx, rx) = watch::channel(false);
        let config = ProcessConfig {
            command: None,
            args: None,
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 0,
        };
        let (_, mcp_rx) = mpsc::channel(1);
        ProcessManager::monitor_process("test".to_string(), config, mcp_rx, rx).await;
    }

    #[tokio::test]
    async fn test_process_manager_missing_command_and_external() {
        let mut configs = HashMap::new();
        configs.insert(
            "bad".to_string(),
            ProcessConfig {
                command: None,
                args: None,
                external_address: None,
                max_retries: default_max_retries(),
                restart_delay_ms: default_restart_delay_ms(),
            },
        );
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        let _ = pm.stop_all().await;
    }

    #[tokio::test]
    async fn test_mcp_orchestrator_tools_list() {
        let pm = ProcessManager::new(HashMap::new());
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let res = pm.handle_request(req).await.expect("tools/list failed");
        assert_eq!(res.id.unwrap(), serde_json::json!(1));
        assert!(res.result.is_some());
    }

    #[tokio::test]
    async fn test_mcp_orchestrator_tools_call_missing_args() {
        let pm = ProcessManager::new(HashMap::new());
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({})),
            id: Some(serde_json::json!(2)),
        };
        let res = pm.handle_request(req).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_mcp_orchestrator_tools_call_not_found() {
        let pm = ProcessManager::new(HashMap::new());
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "arguments": {
                    "target_language": "rust"
                }
            })),
            id: Some(serde_json::json!(3)),
        };
        let res = pm.handle_request(req).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_process_manager_local_stop_all() -> Result<(), CddEngineError> {
        let mut configs = HashMap::new();
        configs.insert(
            "test-echo".to_string(),
            ProcessConfig {
                command: Some("echo".to_string()),
                args: Some(vec!["hello".to_string()]),
                external_address: None,
                max_retries: 1,
                restart_delay_ms: 100,
            },
        );
        let manager = ProcessManager::new(configs);
        manager.start_all().await?;

        {
            let handles = manager.handles.lock().await;
            assert_eq!(handles.len(), 1);
            assert!(handles.contains_key("test-echo"));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        manager.stop_all().await;

        Ok(())
    }
}

    #[tokio::test]
    async fn test_daemon_coverage() {
        let pm = ProcessManager::new(std::collections::HashMap::new());
        let req = McpRequest { jsonrpc: "2".into(), method: "unknown_method".into(), params: None, id: None };
        let _ = pm.handle_request(req).await;
        pm.stop_all().await;
    }

    #[tokio::test]
    async fn test_daemon_external_address() {
        let mut configs = std::collections::HashMap::new();
        configs.insert("ext".to_string(), ProcessConfig {
            command: None,
            args: None,
            external_address: Some("127.0.0.1:9090".to_string()),
            max_retries: 0, restart_delay_ms: 0
        });
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
    }

    #[tokio::test]
    async fn test_daemon_dummy_proc() {
        let mut configs = std::collections::HashMap::new();
        configs.insert("dummy".to_string(), ProcessConfig {
            command: Some("echo".to_string()),
            args: Some(vec!["test".to_string()]),
            external_address: None,
            max_retries: 0, restart_delay_ms: 0
        });
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_daemon_mcp_real() {
        let mut configs = std::collections::HashMap::new();
        configs.insert("echo".to_string(), ProcessConfig {
            command: Some("echo".to_string()),
            args: Some(vec!["test".to_string()]),
            external_address: None,
            max_retries: 0, restart_delay_ms: 0
        });
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let req = McpRequest { jsonrpc: "2.0".into(), method: "some_method".into(), params: None, id: Some(serde_json::json!(1)) };
        let _ = pm.handle_request(req).await;
        
        let req_noid = McpRequest { jsonrpc: "2.0".into(), method: "some_method".into(), params: None, id: None };
        let _ = pm.handle_request(req_noid).await;
        
        pm.stop_all().await;
        let req = McpRequest { jsonrpc: "2.0".into(), method: "some_method".into(), params: None, id: Some(serde_json::json!(1)) };
        let _ = pm.handle_request(req).await;
}
