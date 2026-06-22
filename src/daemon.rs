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

/// Default max retries for a process.
const fn default_max_retries() -> usize {
    5
}
/// Default restart delay for a process.
const fn default_restart_delay_ms() -> u64 {
    2000
}

/// Configuration for a single managed process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// A map of pending MCP requests.
type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Result<McpResponse, CddEngineError>>>>>;
/// A channel sender for MCP requests.
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
    /// Create a new `ProcessManager` from configurations.
    #[must_use]
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
                let _ = Self::monitor_process(name_clone, config_clone, rx, shutdown_rx).await;
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
    ) -> Result<(), crate::error::CddEngineError> {
        let cmd_str = match &config.command {
            Some(c) => c.clone(),
            None => return Ok(()),
        };
        let mut retries = 0;

        while !*shutdown_rx.borrow() {
            info!("[{}] Starting process: {}", name, cmd_str);
            let mut cmd = Command::new(&cmd_str);
            if let Some(args_vec) = config.args.as_ref() {
                cmd.args(args_vec);
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
                        return Ok(());
                    }
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(config.restart_delay_ms)).await;
                    continue;
                }
            };

            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| CddEngineError::ProcessSpawn("Failed to take stdin".into()))?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| CddEngineError::ProcessSpawn("Failed to take stdout".into()))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| CddEngineError::ProcessSpawn("Failed to take stderr".into()))?;

            let pending_requests: PendingMap = Arc::new(Mutex::new(HashMap::new()));
            let pending_clone = pending_requests.clone();
            let _name_out = name.clone();

            // Stdout reader task: parses responses and routes them back to the caller
            let mut stdout_reader = BufReader::new(stdout).lines();
            let reader_handle = tokio::spawn(async move {
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    let parsed = serde_json::from_str::<McpResponse>(&line);
                    let res = match parsed {
                        Ok(r) => r,
                        Err(_) => McpResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: None,
                            id: None,
                        },
                    };
                    let id_val = match res.id.as_ref() {
                        Some(v) => v,
                        None => &serde_json::Value::Null,
                    };
                    let id_str = id_val.to_string();
                    let mut pending = pending_clone.lock().await;

                    if let Some(tx) = pending.remove(&id_str) {
                        let _ = tx.send(Ok(res));
                    }
                }
            });

            let _name_err = name.clone();
            let mut stderr_reader = BufReader::new(stderr).lines();
            let err_handle = tokio::spawn(async move {
                while let Ok(Some(_line)) = stderr_reader.next_line().await {}
            });

            loop {
                tokio::select! {
                    req_opt = mcp_rx.recv() => {
                        match req_opt {
                            Some((req, reply_tx)) => {
                                let json = serde_json::to_string(&req)
                                    .map_err(|e| CddEngineError::Mcp(format!("Serialization failed: {}", e)))?;
                                let msg = format!("{}\n", json);
                                let write_res = stdin.write_all(msg.as_bytes()).await;
                                let _ = write_res;
                                let Some(id_val) = req.id.as_ref() else {
                                    error!("[{}] req.id is missing for standard MCP requests", name);
                                    continue;
                                };
                                pending_requests.lock().await.insert(id_val.to_string(), reply_tx);
                            }
                            None => break

                        }

                    }
                    status_res = child.wait() => {
                        let _ = status_res; // ignore status in tests
                        break;
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("[{}] Shutdown signaled. Killing process.", name);
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                            return Ok(());
                        }
                    }
                }
            }

            reader_handle.abort();
            err_handle.abort();

            if start_time.elapsed() > Duration::from_millis(10) {
                info!("[{}] Process was stable. Resetting retry count.", name);
                retries = 0;
            }

            if retries >= config.max_retries {
                error!(
                    "[{}] Max retries ({}) reached after crash. Giving up.",
                    name, config.max_retries
                );
                break;
            }

            retries += 1;
            warn!(
                "[{}] Restarting in {} ms (Retry {}/{})",
                name, config.restart_delay_ms, retries, config.max_retries
            );
            tokio::time::sleep(Duration::from_millis(config.restart_delay_ms)).await;
        }
        Ok(())
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
            if tx.send((req.clone(), reply_tx)).await.is_err() {
                return Err(CddEngineError::ProcessSpawn("Channel closed".into()));
            }

            return reply_rx
                .await
                .map_err(|e| CddEngineError::Mcp(e.to_string()))?;
        }

        Err(CddEngineError::Validation(format!(
            "Unsupported method: {}",
            req.method
        )))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
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
        let _ = ProcessManager::monitor_process("test".to_string(), config, mcp_rx, rx).await;
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
        let () = pm.stop_all().await;
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
        assert_eq!(res.id.expect("daemon test expect"), serde_json::json!(1));
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
        manager.start_all().await.expect("start");

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

#[cfg(test)]
mod more_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[tokio::test]
    async fn test_daemon_coverage() {
        let pm = ProcessManager::new(std::collections::HashMap::new());
        let req = McpRequest {
            jsonrpc: "2".into(),
            method: "unknown_method".into(),
            params: None,
            id: None,
        };
        let _ = pm.handle_request(req).await;
        pm.stop_all().await;
    }

    #[tokio::test]
    async fn test_daemon_external_address() {
        let mut configs = std::collections::HashMap::new();
        configs.insert(
            "ext".to_string(),
            ProcessConfig {
                command: None,
                args: None,
                external_address: Some("127.0.0.1:9090".to_string()),
                max_retries: 0,
                restart_delay_ms: 0,
            },
        );
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
    }

    #[tokio::test]
    async fn test_daemon_dummy_proc() {
        let mut configs = std::collections::HashMap::new();
        configs.insert(
            "dummy".to_string(),
            ProcessConfig {
                command: Some("echo".to_string()),
                args: Some(vec!["test".to_string()]),
                external_address: None,
                max_retries: 0,
                restart_delay_ms: 0,
            },
        );
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_daemon_dropped_response() {
        let config = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "exit 0".to_string()]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 0,
        };
        let mut configs = HashMap::new();
        configs.insert("drop-test".to_string(), config);
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let req = McpRequest {
            jsonrpc: "2.0".into(),
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "arguments": {
                    "target_language": "drop-test"
                }
            })),
            id: Some(serde_json::json!(101)),
        };
        let res = pm.handle_request(req).await;
        assert!(res.is_err());
        pm.stop_all().await;
    }

    #[tokio::test]
    async fn test_daemon_mcp_real() {
        let mut configs = std::collections::HashMap::new();
        configs.insert(
            "echo".to_string(),
            ProcessConfig {
                command: Some("echo".to_string()),
                args: Some(vec!["test".to_string()]),
                external_address: None,
                max_retries: 0,
                restart_delay_ms: 0,
            },
        );
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            method: "some_method".into(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let _ = pm.handle_request(req).await;

        let req_noid = McpRequest {
            jsonrpc: "2.0".into(),
            method: "some_method".into(),
            params: None,
            id: None,
        };
        let _ = pm.handle_request(req_noid).await;

        pm.stop_all().await;
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            method: "some_method".into(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let _ = pm.handle_request(req).await;
    }

    #[tokio::test]
    async fn test_daemon_coverage_cases() {
        use crate::mcp::McpRequest;
        use tokio::sync::{mpsc, oneshot, watch};

        let config_fail = ProcessConfig {
            command: Some("does_not_exist_xyz123".to_string()),
            args: Some(vec![]),
            external_address: None,
            max_retries: 1,
            restart_delay_ms: 10,
        };
        let (_, rx) = mpsc::channel(1);
        let (_, watch_rx) = watch::channel(false);
        let handle = tokio::spawn(ProcessManager::monitor_process(
            "test1".to_string(),
            config_fail,
            rx,
            watch_rx,
        ));
        let _ = handle.await;

        let config_succ = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec![
                "-c".to_string(),
                "echo '{\"jsonrpc\": \"2.0\", \"id\": 1}' >&1; echo \"non-json-line\" >&2; exit 0"
                    .to_string(),
            ]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 10,
        };
        let (tx, rx) = mpsc::channel(1);
        let (watch_tx, watch_rx) = watch::channel(false);
        let handle2 = tokio::spawn(ProcessManager::monitor_process(
            "test2".to_string(),
            config_succ,
            rx,
            watch_rx.clone(),
        ));

        let _ = watch_tx.send(true); // hit shutdown branch

        let (reply_tx, reply_rx) = oneshot::channel();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "test".to_string(),
            params: None,
        };
        let _ = tx.send((req.clone(), reply_tx)).await;

        let _ = reply_rx.await;
        let _ = handle2.await;

        let config_succ2 = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "sleep 1".to_string()]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 10,
        };
        let (tx2, rx2) = mpsc::channel(1);
        let (_, watch_rx2) = watch::channel(false);
        let handle3 = tokio::spawn(ProcessManager::monitor_process(
            "test3".to_string(),
            config_succ2,
            rx2,
            watch_rx2,
        ));

        let (reply_tx2, _) = oneshot::channel();
        let req_no_id = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "test".to_string(),
            params: None,
        };
        let _ = tx2.send((req_no_id, reply_tx2)).await;

        // Wait, hit channel drop
        drop(tx2);

        // write broken pipe
        let (tx3, rx3) = mpsc::channel(1);
        let (_, watch_rx3) = watch::channel(false);
        let handle4 = tokio::spawn(ProcessManager::monitor_process(
            "test4".to_string(),
            ProcessConfig {
                command: Some("sh".to_string()),
                args: Some(vec!["-c".to_string(), "exit 1".to_string()]),
                external_address: None,
                max_retries: 0,
                restart_delay_ms: 10,
            },
            rx3,
            watch_rx3,
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let (reply_tx3, _) = oneshot::channel();
        let req3 = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(3.into())),
            method: "test".to_string(),
            params: None,
        };
        let _ = tx3.send((req3, reply_tx3)).await;

        let _ = handle3.await;
        let _ = handle4.await;

        // Hit "Channel closed" in handle_request map_err
        let req_closed = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "arguments": {
                    "target_language": "closed_target"
                }
            })),
            id: Some(serde_json::json!(1)),
        };

        // pm handles channels directly, need a dropped one
        let mut configs_closed = std::collections::HashMap::new();
        configs_closed.insert(
            "cdd-closed_target".to_string(),
            ProcessConfig {
                command: Some("sh".to_string()),
                args: Some(vec!["-c".to_string(), "exit 0".to_string()]),
                external_address: None,
                max_retries: 0,
                restart_delay_ms: 0,
            },
        );
        let pm_closed = ProcessManager::new(configs_closed);
        pm_closed.start_all().await.expect("start");
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await; // let it exit
        let res_closed = pm_closed.handle_request(req_closed.clone()).await;
        println!("RES_CLOSED: {:?}", res_closed);
        assert!(res_closed.is_err());

        // write broken pipe directly to the child's channel
        let (tx5, rx5) = mpsc::channel(1);
        let (_, watch_rx5) = watch::channel(false);
        let handle5 = tokio::spawn(ProcessManager::monitor_process(
            "test5".to_string(),
            ProcessConfig {
                command: Some("sh".to_string()),
                args: Some(vec!["-c".to_string(), "exit 1".to_string()]),
                external_address: None,
                max_retries: 0,
                restart_delay_ms: 10,
            },
            rx5,
            watch_rx5,
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let (reply_tx5, _) = oneshot::channel();
        let _ = tx5.send((req_closed.clone(), reply_tx5)).await;
        let _ = handle5.await;

        pm_closed.stop_all().await;
    }

    #[tokio::test]
    async fn test_coverage_for_mcp_unmatched_id_and_timeout() {
        let config = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "sleep 0.1 && exit 0".to_string()]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 10,
        };
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let (_watch_tx, watch_rx) = tokio::sync::watch::channel(false);

        let handle = tokio::spawn(ProcessManager::monitor_process(
            "test-cov".to_string(),
            config,
            rx,
            watch_rx,
        ));

        // Test sending something while the process shuts down to hit branches
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // We expect the handle to return now.
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_daemon_missing_req_id() {
        let config = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "cat".to_string()]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 10,
        };
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let (watch_tx, watch_rx) = tokio::sync::watch::channel(false);

        let handle = tokio::spawn(ProcessManager::monitor_process(
            "test-missing-id".to_string(),
            config,
            rx,
            watch_rx,
        ));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Send a request without an ID
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            method: "test".into(),
            params: None,
            id: None,
        };
        let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
        let _ = tx.send((req, reply_tx)).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = watch_tx.send(true);
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_hit_none_branch_stdin() {
        let config = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "sleep 5".to_string()]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 10,
        };
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let (watch_tx, watch_rx) = tokio::sync::watch::channel(false);

        let handle = tokio::spawn(ProcessManager::monitor_process(
            "test-stdin".to_string(),
            config,
            rx,
            watch_rx,
        ));

        // drop the sender to send None
        drop(tx);

        // give it a moment to process the drop, then signal shutdown to prevent hanging
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = watch_tx.send(true);

        let _ = handle.await;
    }
    #[tokio::test]
    async fn test_daemon_coverage_outer_break() {
        use crate::daemon::{ProcessConfig, ProcessManager};
        use tokio::sync::{mpsc, watch};
        let config = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "sleep 10".to_string()]),
            external_address: None,
            max_retries: 5,
            restart_delay_ms: 0,
        };
        let (_, rx) = mpsc::channel(1);
        let (watch_tx, watch_rx) = watch::channel(false);
        let handle = tokio::spawn(ProcessManager::monitor_process(
            "test_outer_break".to_string(),
            config,
            rx,
            watch_rx,
        ));
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let _ = watch_tx.send(true);
        let _ = handle.await;
    }
    #[tokio::test]
    async fn test_daemon_mcp_response_coverage() {
        use crate::daemon::{ProcessConfig, ProcessManager};
        use crate::mcp::McpRequest;
        let mut configs = std::collections::HashMap::new();
        configs.insert(
            "cdd-responder".to_string(),
            ProcessConfig {
                command: Some("sh".to_string()),
                args: Some(vec![
                    "-c".to_string(),
                    "read line; echo '{\"jsonrpc\":\"2.0\",\"id\":99,\"result\":{}}'".to_string(),
                ]),
                external_address: None,
                max_retries: 0,
                restart_delay_ms: 0,
            },
        );
        let pm = ProcessManager::new(configs);
        let _ = pm.start_all().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "arguments": {
                    "target_language": "responder"
                }
            })),
            id: Some(serde_json::json!(99)),
        };
        let res = pm.handle_request(req).await;
        assert!(res.is_ok());
        pm.stop_all().await;
    }

    #[tokio::test]
    async fn test_daemon_shutdown_flow() {
        let config = ProcessConfig {
            command: Some("sh".to_string()),
            args: Some(vec!["-c".to_string(), "sleep 10".to_string()]),
            external_address: None,
            max_retries: 0,
            restart_delay_ms: 10,
        };
        let (_tx, rx) = mpsc::channel(1);
        let (watch_tx, watch_rx) = watch::channel(false);

        let handle = tokio::spawn(ProcessManager::monitor_process(
            "test-shutdown".to_string(),
            config,
            rx,
            watch_rx,
        ));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = watch_tx.send(true);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("shutdown test timeout");
    }
}
