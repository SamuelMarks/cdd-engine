use crate::daemon::ProcessConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application configuration loaded from a file and/or environment variables.
///
/// All fields can be overridden via environment variables prefixed with `CDD__`
/// (double underscore as separator), e.g. `CDD__JWT_SECRET=mysecret`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    /// PostgreSQL connection URL (env: `CDD__DATABASE_URL`).
    pub database_url: String,
    /// Address and port the HTTP server binds to (env: `CDD__SERVER_BIND`).
    pub server_bind: String,
    /// Secret used to sign and verify JWT tokens (env: `CDD__JWT_SECRET`).
    ///
    /// Defaults to `"super-secret-key"` — **must** be overridden in production.
    pub jwt_secret: String,
    /// Secret used to verify GitHub webhook HMAC-SHA256 signatures
    /// (env: `CDD__WEBHOOK_SECRET`).
    ///
    /// Defaults to `"my_webhook_secret"` — **must** be overridden in production.
    pub webhook_secret: String,
    /// Optional GitHub personal access token used as a system-level fallback
    /// when no per-user token is available (env: `CDD__GITHUB_TOKEN`).
    pub github_token: Option<String>,
    /// When `true` the server starts without a PostgreSQL connection and uses
    /// an in-memory no-op repository instead (env: `CDD__OFFLINE_MODE`).
    #[serde(default)]
    pub offline_mode: bool,
    /// Child-process configuration keyed by tool name.
    #[serde(default)]
    pub servers: HashMap<String, ProcessConfig>,
}

impl AppConfig {
    /// Load configuration from an optional file path and environment variables.
    ///
    /// Precedence (highest → lowest):
    /// 1. Environment variables (`CDD__*`)
    /// 2. Config file (if `config_path` is `Some`)
    /// 3. Built-in defaults
    pub fn load(config_path: Option<&str>) -> Result<Self, crate::error::CddEngineError> {
        let mut builder = config::Config::builder()
            .set_default("database_url", "postgres://postgres:password@localhost/cdd")?
            .set_default("server_bind", "0.0.0.0:8084")?
            .set_default("jwt_secret", "super-secret-key")?
            .set_default("webhook_secret", "my_webhook_secret")?
            .set_default("offline_mode", false)?;

        if let Some(path) = config_path {
            builder = builder.add_source(config::File::with_name(path).required(false));
        }

        builder
            .add_source(config::Environment::with_prefix("CDD").separator("__"))
            .build()?
            .try_deserialize()
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_config_env_overrides() -> Result<(), crate::error::CddEngineError> {
        let _lock = ENV_MUTEX.lock().expect("mutex");

        // 1. Default config
        std::env::remove_var("CDD__JWT_SECRET");
        std::env::remove_var("CDD__WEBHOOK_SECRET");
        std::env::remove_var("CDD__GITHUB_TOKEN");
        std::env::remove_var("CDD__OFFLINE_MODE");

        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.server_bind, "0.0.0.0:8084");
        assert_eq!(
            cfg.database_url,
            "postgres://postgres:password@localhost/cdd"
        );
        assert_eq!(cfg.jwt_secret, "super-secret-key");
        assert_eq!(cfg.webhook_secret, "my_webhook_secret");
        assert!(cfg.github_token.is_none());
        assert!(!cfg.offline_mode);

        // 2. JWT Secret override
        std::env::set_var("CDD__JWT_SECRET", "test-jwt-secret");
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.jwt_secret, "test-jwt-secret");
        std::env::remove_var("CDD__JWT_SECRET");

        // 3. Webhook Secret override
        std::env::set_var("CDD__WEBHOOK_SECRET", "test-webhook-secret");
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.webhook_secret, "test-webhook-secret");
        std::env::remove_var("CDD__WEBHOOK_SECRET");

        // 4. GitHub Token override
        std::env::set_var("CDD__GITHUB_TOKEN", "ghp_test123");
        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.github_token.as_deref(), Some("ghp_test123"));
        std::env::remove_var("CDD__GITHUB_TOKEN");

        // 5. Offline Mode override
        std::env::set_var("CDD__OFFLINE_MODE", "true");
        let cfg = AppConfig::load(None)?;
        assert!(cfg.offline_mode);
        std::env::remove_var("CDD__OFFLINE_MODE");

        // 6. Config error (deserialization)
        std::env::set_var("CDD__OFFLINE_MODE", "not_a_boolean");
        let err = AppConfig::load(None);
        assert!(err.is_err());
        std::env::remove_var("CDD__OFFLINE_MODE");
        Ok(())
    }

    #[test]
    fn test_config_load_with_file_path() -> Result<(), crate::error::CddEngineError> {
        let _lock = ENV_MUTEX.lock().expect("mutex");
        std::env::remove_var("CDD__OFFLINE_MODE");

        // Create a temporary file with config
        use std::io::Write;
        let file_path = "test_cdd_config.toml";
        let mut file = std::fs::File::create(file_path)?;
        writeln!(file, "server_bind = \"127.0.0.1:9090\"")?;

        let config = AppConfig::load(Some(file_path))?;
        assert_eq!(config.server_bind, "127.0.0.1:9090");

        std::fs::remove_file(file_path)?;
        Ok(())
    }
    #[test]
    fn test_config_derives() -> Result<(), Box<dyn std::error::Error>> {
        let process_config = ProcessConfig {
            command: Some("cmd".to_string()),
            args: None,
            external_address: None,
            max_retries: 3,
            restart_delay_ms: 100,
        };

        let mut different_pc = process_config.clone();
        different_pc.command = Some("other".to_string());
        assert_ne!(process_config, different_pc);
        assert_eq!(process_config, process_config.clone());
        let mut servers = HashMap::new();
        servers.insert("test".to_string(), process_config.clone());

        let config = AppConfig {
            database_url: "url".to_string(),
            server_bind: "bind".to_string(),
            jwt_secret: "jwt".to_string(),
            webhook_secret: "webhook".to_string(),
            github_token: None,
            offline_mode: false,
            servers,
        };
        let cloned = config.clone();
        assert_eq!(config.database_url, cloned.database_url);

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("AppConfig"));

        let serialized = serde_json::to_string(&config)?;
        let deserialized: AppConfig = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized.database_url, config.database_url);
        Ok(())
    }
}
#[test]
fn test_config_serde_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "database_url": "url",
        "server_bind": "bind",
        "jwt_secret": "jwt",
        "webhook_secret": "webhook"
    }"#;
    let de: AppConfig = serde_json::from_str(json)?;
    assert_eq!(de.offline_mode, false);
    assert!(de.servers.is_empty());
    Ok(())
}

#[test]
fn test_process_config_derives() {
    use crate::daemon::ProcessConfig;
    let config = ProcessConfig {
        command: Some("cmd".to_string()),
        args: None,
        external_address: None,
        max_retries: 3,
        restart_delay_ms: 100,
    };
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("ProcessConfig"));

    let cloned = config.clone();
    assert_eq!(config.command, cloned.command);

    let serialized = serde_json::to_string(&config).expect("json error");
    let deserialized: ProcessConfig = serde_json::from_str(&serialized).expect("json error");
    assert_eq!(deserialized.command, config.command);
}

#[test]
fn test_config_serde_all_fields() {
    let json = r#"{
        "database_url": "url",
        "server_bind": "bind",
        "jwt_secret": "jwt",
        "webhook_secret": "webhook",
        "github_token": "token",
        "offline_mode": true,
        "servers": {
            "my_server": {
                "command": "cmd",
                "args": ["a"],
                "external_address": "addr",
                "max_retries": 1,
                "restart_delay_ms": 1
            }
        }
    }"#;
    let de: AppConfig = serde_json::from_str(json).expect("json");
    assert_eq!(de.github_token.as_deref(), Some("token"));
}

#[test]
fn test_config_serde_invalid_types() {
    let json = r#"123"#;
    let de: Result<AppConfig, _> = serde_json::from_str(json);
    assert!(de.is_err());

    let json2 = r#"[1, 2, 3]"#;
    let de2: Result<AppConfig, _> = serde_json::from_str(json2);
    assert!(de2.is_err());

    let pc_json = r#"123"#;
    use crate::daemon::ProcessConfig;
    let pc_de: Result<ProcessConfig, _> = serde_json::from_str(pc_json);
    assert!(pc_de.is_err());
}
