#![deny(missing_docs)]

//! Configuration loading and validation for `cdd-engine`.
//!
//! Supports hierarchical configuration via built-in defaults, optional configuration files
//! (TOML/JSON/YAML), and `CDD__*` environment variable overrides.

use crate::daemon::ProcessConfig;
use crate::error::CddEngineError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Default directory containing compiled `.wasm` generator modules.
pub const DEFAULT_WASM_ASSETS_DIR: &str = "cdd-ctl-wasm-sdk/assets/wasm";

/// Default path to the `wasm-support.json` capability matrix.
pub const DEFAULT_WASM_SUPPORT_MANIFEST: &str = "cdd-ctl-wasm-sdk/assets/wasm-support.json";

/// Default maximum in-memory pipe capacity (10 MiB) for capturing WASI stdout/stderr.
pub const DEFAULT_MAX_STDIO_BUFFER_BYTES: usize = 10 * 1024 * 1024;

/// Default timeout in milliseconds (`30_000` ms) for individual MCP daemon requests.
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30_000;

/// Returns the default WASM assets directory [`PathBuf`].
fn default_wasm_assets_dir() -> PathBuf {
    PathBuf::from(DEFAULT_WASM_ASSETS_DIR)
}

/// Returns the default WASM support manifest [`PathBuf`].
fn default_wasm_support_manifest() -> PathBuf {
    PathBuf::from(DEFAULT_WASM_SUPPORT_MANIFEST)
}

/// Returns the default maximum WASI stdio buffer size in bytes.
const fn default_max_stdio_buffer_bytes() -> usize {
    DEFAULT_MAX_STDIO_BUFFER_BYTES
}

/// Returns the default MCP request timeout in milliseconds.
const fn default_request_timeout_ms() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_MS
}

/// Application and execution engine configuration loaded from a file and/or environment variables.
///
/// All fields can be overridden via environment variables prefixed with `CDD__`
/// (double underscore as separator), e.g. `CDD__SERVER_BIND=127.0.0.1:8084`.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// Address and port the local RPC/MCP server binds to (env: `CDD__SERVER_BIND`).
    pub server_bind: String,
    /// Directory containing compiled `.wasm` artifacts (env: `CDD__WASM_ASSETS_DIR`).
    #[serde(default = "default_wasm_assets_dir")]
    pub wasm_assets_dir: PathBuf,
    /// Path to the `wasm-support.json` manifest (env: `CDD__WASM_SUPPORT_MANIFEST`).
    #[serde(default = "default_wasm_support_manifest")]
    pub wasm_support_manifest: PathBuf,
    /// Maximum buffer size in bytes for capturing WASI stdout/stderr (env: `CDD__MAX_STDIO_BUFFER_BYTES`).
    #[serde(default = "default_max_stdio_buffer_bytes")]
    pub max_stdio_buffer_bytes: usize,
    /// Default request timeout in milliseconds for child daemon calls (env: `CDD__DEFAULT_REQUEST_TIMEOUT_MS`).
    #[serde(default = "default_request_timeout_ms")]
    pub default_request_timeout_ms: u64,
    /// When `true`, the engine operates strictly in offline / sandboxed mode (env: `CDD__OFFLINE_MODE`).
    #[serde(default)]
    pub offline_mode: bool,
    /// Child-process configurations keyed by daemon or tool name.
    #[serde(default)]
    pub servers: HashMap<String, ProcessConfig>,
}

impl AppConfig {
    /// Loads [`AppConfig`] from an optional file path and `CDD__*` environment variables.
    ///
    /// Precedence (highest to lowest):
    /// 1. Environment variables (`CDD__*`)
    /// 2. Configuration file (if `config_path` is `Some`)
    /// 3. Built-in defaults
    ///
    /// # Arguments
    ///
    /// * `config_path` - Optional path to a configuration file.
    ///
    /// # Returns
    ///
    /// The validated and deserialized [`AppConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::ConfigLoad`] or [`CddEngineError::Config`] if building or
    /// deserializing the configuration fails.
    pub fn load(config_path: Option<&str>) -> Result<Self, CddEngineError> {
        let mut builder = config::Config::builder()
            .set_default("server_bind", "127.0.0.1:8084")?
            .set_default("wasm_assets_dir", DEFAULT_WASM_ASSETS_DIR)?
            .set_default("wasm_support_manifest", DEFAULT_WASM_SUPPORT_MANIFEST)?
            .set_default(
                "max_stdio_buffer_bytes",
                i64::try_from(DEFAULT_MAX_STDIO_BUFFER_BYTES)?,
            )?
            .set_default(
                "default_request_timeout_ms",
                i64::try_from(DEFAULT_REQUEST_TIMEOUT_MS)?,
            )?
            .set_default("offline_mode", false)?;

        if let Some(path) = config_path {
            builder = builder.add_source(config::File::with_name(path).required(false));
        }

        builder
            .add_source(config::Environment::with_prefix("CDD").separator("__"))
            .build()?
            .try_deserialize()
            .map_err(|e| CddEngineError::Config(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`AppConfig`] loading, environment overrides, and serde round-trips.

    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    /// Global mutex serializing environment variable manipulation during configuration tests.
    static ENV_MUTEX: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));

    /// RAII guard that removes a temporary file on drop.
    struct TempFileGuard {
        /// Path to the temporary file.
        path: PathBuf,
    }

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Clears all `CDD__*` environment variables used in tests.
    fn clear_cdd_env_vars() {
        std::env::remove_var("CDD__SERVER_BIND");
        std::env::remove_var("CDD__WASM_ASSETS_DIR");
        std::env::remove_var("CDD__WASM_SUPPORT_MANIFEST");
        std::env::remove_var("CDD__MAX_STDIO_BUFFER_BYTES");
        std::env::remove_var("CDD__DEFAULT_REQUEST_TIMEOUT_MS");
        std::env::remove_var("CDD__OFFLINE_MODE");
    }

    /// Verifies built-in defaults and `CDD__*` environment variable overrides.
    #[test]
    fn test_config_env_overrides() -> Result<(), CddEngineError> {
        let guard = ENV_MUTEX
            .lock()
            .map_err(|e| CddEngineError::Internal(e.to_string()))?;
        clear_cdd_env_vars();

        // 1. Default config
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.server_bind, "127.0.0.1:8084");
        assert_eq!(cfg.wasm_assets_dir, PathBuf::from(DEFAULT_WASM_ASSETS_DIR));
        assert_eq!(
            cfg.wasm_support_manifest,
            PathBuf::from(DEFAULT_WASM_SUPPORT_MANIFEST)
        );
        assert_eq!(cfg.max_stdio_buffer_bytes, DEFAULT_MAX_STDIO_BUFFER_BYTES);
        assert_eq!(cfg.default_request_timeout_ms, DEFAULT_REQUEST_TIMEOUT_MS);
        assert!(!cfg.offline_mode);

        // 2. Server bind override
        std::env::set_var("CDD__SERVER_BIND", "0.0.0.0:9000");
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.server_bind, "0.0.0.0:9000");
        std::env::remove_var("CDD__SERVER_BIND");

        // 3. WASM assets dir & manifest override
        std::env::set_var("CDD__WASM_ASSETS_DIR", "/tmp/custom-wasm");
        std::env::set_var("CDD__WASM_SUPPORT_MANIFEST", "/tmp/support.json");
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.wasm_assets_dir, PathBuf::from("/tmp/custom-wasm"));
        assert_eq!(
            cfg.wasm_support_manifest,
            PathBuf::from("/tmp/support.json")
        );
        std::env::remove_var("CDD__WASM_ASSETS_DIR");
        std::env::remove_var("CDD__WASM_SUPPORT_MANIFEST");

        // 4. Buffer and timeout override
        std::env::set_var("CDD__MAX_STDIO_BUFFER_BYTES", "2048");
        std::env::set_var("CDD__DEFAULT_REQUEST_TIMEOUT_MS", "5000");
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.max_stdio_buffer_bytes, 2048);
        assert_eq!(cfg.default_request_timeout_ms, 5000);
        std::env::remove_var("CDD__MAX_STDIO_BUFFER_BYTES");
        std::env::remove_var("CDD__DEFAULT_REQUEST_TIMEOUT_MS");

        // 5. Offline mode override
        std::env::set_var("CDD__OFFLINE_MODE", "true");
        let cfg = AppConfig::load(None)?;
        assert!(cfg.offline_mode);
        std::env::remove_var("CDD__OFFLINE_MODE");

        // 6. Invalid type deserialization error
        std::env::set_var("CDD__OFFLINE_MODE", "not_a_boolean");
        let err = AppConfig::load(None);
        assert!(err.is_err());
        clear_cdd_env_vars();
        drop(guard);
        Ok(())
    }

    /// Verifies loading configuration from an isolated temporary TOML file without polluting the repo root.
    #[test]
    fn test_config_load_with_file_path() -> Result<(), CddEngineError> {
        let guard = ENV_MUTEX
            .lock()
            .map_err(|e| CddEngineError::Internal(e.to_string()))?;
        clear_cdd_env_vars();

        let temp_path = std::env::temp_dir().join(format!(
            "cdd_engine_test_config_{}.toml",
            std::process::id()
        ));
        let _cleanup = TempFileGuard {
            path: temp_path.clone(),
        };

        let mut file = std::fs::File::create(&temp_path)?;
        writeln!(file, r#"server_bind = "127.0.0.1:9090""#)?;
        writeln!(file, "offline_mode = true")?;

        let path_str = temp_path.to_string_lossy();
        let config = AppConfig::load(Some(&path_str))?;
        assert_eq!(config.server_bind, "127.0.0.1:9090");
        assert!(config.offline_mode);

        drop(guard);
        Ok(())
    }

    /// Verifies `Clone`, `Debug`, `PartialEq`, `Eq`, and JSON serde round-trips for [`AppConfig`]
    /// and [`ProcessConfig`].
    #[test]
    fn test_config_derives_and_equality() -> Result<(), CddEngineError> {
        let process_config = ProcessConfig {
            command: Some("cdd-rust".to_string()),
            args: None,
            external_address: None,
            max_retries: 3,
            restart_delay_ms: 100,
            stability_threshold_ms: 5000,
        };

        let mut different_pc = process_config.clone();
        different_pc.command = Some("cdd-go".to_string());
        assert_ne!(process_config, different_pc);
        assert_eq!(process_config, process_config.clone());
        assert!(format!("{process_config:?}").contains("ProcessConfig"));

        let pc_json = serde_json::to_string(&process_config)?;
        let pc_de: ProcessConfig = serde_json::from_str(&pc_json)?;
        assert_eq!(pc_de, process_config);

        let mut servers = HashMap::new();
        servers.insert("cdd-rust".to_string(), process_config);

        let config = AppConfig {
            server_bind: "127.0.0.1:8084".to_string(),
            wasm_assets_dir: PathBuf::from(DEFAULT_WASM_ASSETS_DIR),
            wasm_support_manifest: PathBuf::from(DEFAULT_WASM_SUPPORT_MANIFEST),
            max_stdio_buffer_bytes: DEFAULT_MAX_STDIO_BUFFER_BYTES,
            default_request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            offline_mode: false,
            servers,
        };
        let cloned = config.clone();
        assert_eq!(config, cloned);
        assert!(format!("{config:?}").contains("AppConfig"));

        let serialized = serde_json::to_string(&config)?;
        let deserialized: AppConfig = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, config);
        Ok(())
    }

    /// Verifies serde defaults when optional fields are omitted from JSON.
    #[test]
    fn test_config_serde_defaults() -> Result<(), CddEngineError> {
        let json = r#"{
            "server_bind": "127.0.0.1:8084",
            "servers": {
                "cdd-rust": {
                    "command": "cdd-rust",
                    "args": null,
                    "external_address": null
                }
            }
        }"#;
        let de: AppConfig = serde_json::from_str(json)?;
        assert!(!de.offline_mode);
        assert_eq!(de.servers.len(), 1);
        assert_eq!(
            de.servers.get("cdd-rust").map(|p| p.stability_threshold_ms),
            Some(crate::daemon::DEFAULT_STABILITY_THRESHOLD_MS)
        );
        assert_eq!(de.wasm_assets_dir, PathBuf::from(DEFAULT_WASM_ASSETS_DIR));
        assert_eq!(
            de.wasm_support_manifest,
            PathBuf::from(DEFAULT_WASM_SUPPORT_MANIFEST)
        );
        assert_eq!(de.max_stdio_buffer_bytes, DEFAULT_MAX_STDIO_BUFFER_BYTES);
        assert_eq!(de.default_request_timeout_ms, DEFAULT_REQUEST_TIMEOUT_MS);
        Ok(())
    }

    /// Verifies serde deserialization when all fields are explicitly provided.
    #[test]
    fn test_config_serde_all_fields() -> Result<(), CddEngineError> {
        let json = r#"{
            "server_bind": "127.0.0.1:8084",
            "wasm_assets_dir": "/opt/wasm",
            "wasm_support_manifest": "/opt/wasm-support.json",
            "max_stdio_buffer_bytes": 65536,
            "default_request_timeout_ms": 15000,
            "offline_mode": true,
            "servers": {
                "cdd-rust": {
                    "command": "cdd-rust",
                    "args": ["--mcp"],
                    "external_address": null,
                    "max_retries": 2,
                    "restart_delay_ms": 50,
                    "stability_threshold_ms": 1000
                }
            }
        }"#;
        let de: AppConfig = serde_json::from_str(json)?;
        assert!(de.offline_mode);
        assert_eq!(de.max_stdio_buffer_bytes, 65536);
        assert_eq!(de.default_request_timeout_ms, 15000);
        assert!(de.servers.contains_key("cdd-rust"));
        Ok(())
    }

    /// Verifies that invalid JSON types fail deserialization cleanly.
    #[test]
    fn test_config_serde_invalid_types() {
        let de: Result<AppConfig, _> = serde_json::from_str("123");
        assert!(de.is_err());

        let de2: Result<AppConfig, _> = serde_json::from_str("[1, 2, 3]");
        assert!(de2.is_err());

        let pc_de: Result<ProcessConfig, _> = serde_json::from_str("123");
        assert!(pc_de.is_err());
    }
}
