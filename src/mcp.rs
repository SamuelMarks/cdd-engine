use serde::{Deserialize, Serialize};

/// Represents an MCP (Model Context Protocol) JSON-RPC request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpRequest {
    /// JSON-RPC version string, typically "2.0".
    pub jsonrpc: String,
    /// The method name to be invoked.
    pub method: String,
    /// Optional parameters for the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Optional request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// Represents an MCP (Model Context Protocol) JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResponse {
    /// JSON-RPC version string, typically "2.0".
    pub jsonrpc: String,
    /// Optional result returned by the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Optional error object if the method invocation failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    /// Request identifier that matches the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// A trait defining the behavior of an MCP orchestrator.
#[async_trait::async_trait]
pub trait McpOrchestrator: Send + Sync {
    /// Handles an incoming MCP request.
    async fn handle_request(
        &self,
        request: McpRequest,
    ) -> Result<McpResponse, crate::error::CddEngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request() {
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let s = serde_json::to_string(&req).expect("test error");
        let de: McpRequest = serde_json::from_str(&s).expect("test error");
        assert_eq!(req, de);
    }

    #[test]
    fn test_mcp_response() {
        let res = McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!(true)),
            error: None,
            id: Some(serde_json::json!(1)),
        };
        let s = serde_json::to_string(&res).expect("test error");
        let de: McpResponse = serde_json::from_str(&s).expect("test error");
        assert_eq!(res, de);
    }
}
