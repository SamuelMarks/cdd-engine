#![deny(missing_docs)]

//! Model Context Protocol (MCP) definitions and orchestrator trait.

use crate::error::CddEngineError;
use serde::{Deserialize, Serialize};

/// An MCP Request object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpRequest {
    /// JSON-RPC version, typically "2.0".
    pub jsonrpc: String,
    /// Request method name.
    pub method: String,
    /// Request parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Request ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// An MCP Response object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResponse {
    /// JSON-RPC version, typically "2.0".
    pub jsonrpc: String,
    /// Response result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Response error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    /// Response ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// Core trait for orchestrating MCP tool calls across daemons.
#[async_trait::async_trait]
pub trait McpOrchestrator: Send + Sync {
    /// Handle an incoming MCP request, route it to the appropriate daemon, and return the response.
    async fn handle_request(&self, req: McpRequest) -> Result<McpResponse, CddEngineError>;
}
