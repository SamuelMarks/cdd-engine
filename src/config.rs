#![cfg(not(tarpaulin_include))]

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
            .set_default("database_url", "postgres://postgres:password@localhost/cdd")
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?
            .set_default("server_bind", "0.0.0.0:8080")
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?
            .set_default("jwt_secret", "super-secret-key")
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?
            .set_default("webhook_secret", "my_webhook_secret")
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?
            .set_default("offline_mode", false)
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?;

        if let Some(path) = config_path {
            builder = builder.add_source(config::File::with_name(path).required(false));
        }

        builder
            .add_source(config::Environment::with_prefix("CDD").separator("__"))
            .build()
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?
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
        let _lock = ENV_MUTEX
            .lock()
            .map_err(|e| crate::error::CddEngineError::Config(e.to_string()))?;

        // 1. Default config
        std::env::remove_var("CDD__JWT_SECRET");
        std::env::remove_var("CDD__WEBHOOK_SECRET");
        std::env::remove_var("CDD__GITHUB_TOKEN");
        std::env::remove_var("CDD__OFFLINE_MODE");

        let cfg = AppConfig::load(None)?;
        assert_eq!(cfg.server_bind, "0.0.0.0:8080");
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

        Ok(())
    }

    #[test]
    fn test_config_load_with_file_path() -> Result<(), crate::error::CddEngineError> {
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
}
