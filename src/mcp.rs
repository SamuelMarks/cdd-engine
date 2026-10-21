#![deny(missing_docs)]

//! Model Context Protocol (MCP) JSON-RPC 2.0 domain types and orchestrator trait.
//!
//! Provides strongly typed protocol primitives including [`RequestId`], [`TargetLanguage`],
//! [`JsonRpcErrorObject`], [`McpTool`], [`CallToolParams`], [`CallToolResult`], [`McpRequest`],
//! [`McpResponse`], and the asynchronous [`McpOrchestrator`] trait.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The canonical JSON-RPC 2.0 protocol version string (`"2.0"`).
pub const JSONRPC_VERSION: &str = "2.0";

/// Default MCP protocol version negotiated during `initialize`.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Strongly typed JSON-RPC 2.0 request identifier.
///
/// Per the JSON-RPC 2.0 and MCP specifications, a request identifier must be either a string
/// or an integer number.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric request identifier.
    Number(i64),
    /// String request identifier.
    String(String),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(num) => write!(f, "{num}"),
            Self::String(text) => write!(f, "{text}"),
        }
    }
}

impl From<i64> for RequestId {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl From<&str> for RequestId {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for RequestId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

/// Supported target language ecosystems in the `cdd-*` toolchain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetLanguage {
    /// C language target (`cdd-c`).
    C,
    /// C++ language target (`cdd-cpp`).
    Cpp,
    /// C# language target (`cdd-csharp`).
    CSharp,
    /// Go language target (`cdd-go`).
    Go,
    /// Java language target (`cdd-java`).
    Java,
    /// Kotlin language target (`cdd-kotlin`).
    Kotlin,
    /// PHP language target (`cdd-php`).
    Php,
    /// Python language target (`cdd-python`).
    Python,
    /// Python all-in-one language target (`cdd-python-all`).
    PythonAll,
    /// Ruby language target (`cdd-ruby`).
    Ruby,
    /// Rust language target (`cdd-rust`).
    Rust,
    /// POSIX Shell language target (`cdd-sh`).
    Sh,
    /// Swift language target (`cdd-swift`).
    Swift,
    /// TypeScript language target (`cdd-ts`).
    Ts,
    /// Custom or third-party language target.
    Custom(String),
}

impl TargetLanguage {
    /// Parses a language identifier (with or without a `"cdd-"` prefix) into a [`TargetLanguage`].
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw target language string (e.g., `"rust"` or `"cdd-rust"`).
    ///
    /// # Returns
    ///
    /// The parsed [`TargetLanguage`] variant.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        let stripped = raw.strip_prefix("cdd-").unwrap_or(raw);
        match stripped {
            "c" => Self::C,
            "cpp" => Self::Cpp,
            "csharp" => Self::CSharp,
            "go" => Self::Go,
            "java" => Self::Java,
            "kotlin" => Self::Kotlin,
            "php" => Self::Php,
            "python" => Self::Python,
            "python-all" => Self::PythonAll,
            "ruby" => Self::Ruby,
            "rust" => Self::Rust,
            "sh" => Self::Sh,
            "swift" => Self::Swift,
            "ts" => Self::Ts,
            other => Self::Custom(other.to_string()),
        }
    }

    /// Returns the short language slug without the `"cdd-"` prefix.
    ///
    /// # Returns
    ///
    /// The canonical short language identifier (e.g., `"rust"`, `"python-all"`).
    #[must_use]
    pub const fn as_slug(&self) -> &str {
        match self {
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "csharp",
            Self::Go => "go",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Php => "php",
            Self::Python => "python",
            Self::PythonAll => "python-all",
            Self::Ruby => "ruby",
            Self::Rust => "rust",
            Self::Sh => "sh",
            Self::Swift => "swift",
            Self::Ts => "ts",
            Self::Custom(custom) => custom.as_str(),
        }
    }

    /// Returns the canonical `cdd-*` daemon and tool name.
    ///
    /// # Returns
    ///
    /// A `String` formatted as `"cdd-<slug>"`.
    #[must_use]
    pub fn daemon_name(&self) -> String {
        format!("cdd-{}", self.as_slug())
    }
}

impl fmt::Display for TargetLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.daemon_name())
    }
}

impl Serialize for TargetLanguage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_slug())
    }
}

impl<'de> Deserialize<'de> for TargetLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::parse(&raw))
    }
}

/// Structured JSON-RPC 2.0 error object returned inside an [`McpResponse`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsonRpcErrorObject {
    /// Standard or server-defined JSON-RPC 2.0 error code.
    pub code: i32,
    /// Human-readable summary of the error.
    pub message: String,
    /// Optional structured data providing additional error context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// JSON Schema definition for an MCP tool's input parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolInputSchema {
    /// Schema type, typically `"object"`.
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Map of parameter names to their JSON schema definitions.
    pub properties: serde_json::Value,
    /// List of required parameter names.
    pub required: Vec<String>,
}

/// Description of an executable tool exposed over MCP (`tools/list`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    /// Unique identifier for the tool.
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema describing the expected tool arguments.
    pub input_schema: ToolInputSchema,
}

/// Result payload returned by the `tools/list` MCP method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// List of available MCP tools.
    pub tools: Vec<McpTool>,
    /// Optional pagination cursor for fetching additional tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Strongly typed arguments for the `cdd_generate_sdk` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerateSdkArguments {
    /// Target ecosystem language for SDK generation.
    pub target_language: TargetLanguage,
    /// Input `OpenAPI` specification path or content.
    #[serde(default)]
    pub input: Option<String>,
    /// Optional output directory path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// Parameters sent in a `tools/call` MCP request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallToolParams {
    /// Optional name of the tool being invoked (defaults to `"cdd_generate_sdk"` if omitted).
    #[serde(default)]
    pub name: Option<String>,
    /// Arguments passed to the tool.
    pub arguments: GenerateSdkArguments,
}

/// Content item returned inside a [`CallToolResult`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolContent {
    /// Textual tool output.
    Text {
        /// Text content returned by the tool.
        text: String,
    },
}

/// Result payload returned by the `tools/call` MCP method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// List of content blocks produced by the tool call.
    pub content: Vec<ToolContent>,
    /// Whether the tool call resulted in a domain execution error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Server implementation metadata returned during MCP `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImplementationInfo {
    /// Name of the MCP server implementation.
    pub name: String,
    /// Version of the MCP server implementation.
    pub version: String,
}

/// Tool capabilities advertised by the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether the server emits notifications when the tool list changes.
    pub list_changed: bool,
}

/// Capabilities advertised by the MCP server during `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerCapabilities {
    /// Tool execution capabilities.
    pub tools: ToolsCapability,
}

/// Result payload returned by the `initialize` MCP handshake method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Negotiated MCP protocol version.
    pub protocol_version: String,
    /// Advertised server capabilities.
    pub capabilities: ServerCapabilities,
    /// Server implementation metadata.
    pub server_info: ImplementationInfo,
}

/// Represents an MCP (Model Context Protocol) JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpRequest {
    /// JSON-RPC version string, typically `"2.0"`.
    pub jsonrpc: String,
    /// The method name to be invoked (e.g., `"initialize"`, `"ping"`, `"tools/list"`, `"tools/call"`).
    pub method: String,
    /// Optional parameters for the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Optional strongly typed request identifier (`None` for notifications).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
}

/// Represents an MCP (Model Context Protocol) JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResponse {
    /// JSON-RPC version string, typically `"2.0"`.
    pub jsonrpc: String,
    /// Optional result returned by the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Optional structured JSON-RPC error object if the method invocation failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
    /// Request identifier that matches the initiating [`McpRequest`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
}

impl McpResponse {
    /// Constructs a successful [`McpResponse`] from a serializable result payload.
    ///
    /// # Arguments
    ///
    /// * `id` - The request identifier from the originating [`McpRequest`].
    /// * `payload` - Any serializable result structure.
    ///
    /// # Returns
    ///
    /// A populated [`McpResponse`] with `error: None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CddEngineError::Json`] if `payload` cannot be serialized to JSON.
    pub fn success<T: Serialize>(
        id: Option<RequestId>,
        payload: &T,
    ) -> Result<Self, crate::error::CddEngineError> {
        let value = serde_json::to_value(payload)?;
        Ok(Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(value),
            error: None,
            id,
        })
    }

    /// Constructs an error [`McpResponse`] from a [`crate::error::CddEngineError`].
    ///
    /// # Arguments
    ///
    /// * `id` - The request identifier from the originating [`McpRequest`].
    /// * `err` - The error that occurred during request handling.
    ///
    /// # Returns
    ///
    /// A populated [`McpResponse`] with `result: None` and a structured [`JsonRpcErrorObject`].
    #[must_use]
    pub fn from_error(id: Option<RequestId>, err: &crate::error::CddEngineError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcErrorObject {
                code: err.jsonrpc_error_code(),
                message: err.to_string(),
                data: None,
            }),
            id,
        }
    }
}

/// A trait defining the behavior of an asynchronous MCP orchestrator.
pub trait McpOrchestrator: Send + Sync {
    /// Handles an incoming MCP request and returns an MCP response.
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming [`McpRequest`] to process.
    ///
    /// # Returns
    ///
    /// The resulting [`McpResponse`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CddEngineError`] if validation, routing, or daemon communication fails.
    fn handle_request(
        &self,
        request: McpRequest,
    ) -> impl std::future::Future<Output = Result<McpResponse, crate::error::CddEngineError>> + Send;
}

#[cfg(test)]
mod tests {
    //! Unit tests for MCP protocol domain models, [`RequestId`], and [`TargetLanguage`].

    use super::*;
    use crate::error::{CddEngineError, JSONRPC_INVALID_PARAMS};

    /// Verifies [`RequestId`] conversions, formatting, and JSON serialization.
    #[test]
    fn test_request_id_variants() -> Result<(), CddEngineError> {
        let num_id = RequestId::from(42_i64);
        assert_eq!(num_id.to_string(), "42");
        assert_eq!(serde_json::to_string(&num_id)?, "42");

        let str_id = RequestId::from("req-abc");
        assert_eq!(str_id.to_string(), "req-abc");
        assert_eq!(serde_json::to_string(&str_id)?, r#""req-abc""#);

        let owned_id = RequestId::from("req-owned".to_string());
        assert_eq!(owned_id, RequestId::String("req-owned".to_string()));

        Ok(())
    }

    /// Verifies [`TargetLanguage`] parsing, slug generation, daemon naming, and serde round-trips
    /// across all 14 standard ecosystems plus custom targets.
    #[test]
    fn test_target_language_all_variants() -> Result<(), CddEngineError> {
        let cases = [
            ("c", "cdd-c", TargetLanguage::C),
            ("cdd-cpp", "cdd-cpp", TargetLanguage::Cpp),
            ("csharp", "cdd-csharp", TargetLanguage::CSharp),
            ("go", "cdd-go", TargetLanguage::Go),
            ("java", "cdd-java", TargetLanguage::Java),
            ("kotlin", "cdd-kotlin", TargetLanguage::Kotlin),
            ("php", "cdd-php", TargetLanguage::Php),
            ("python", "cdd-python", TargetLanguage::Python),
            ("python-all", "cdd-python-all", TargetLanguage::PythonAll),
            ("ruby", "cdd-ruby", TargetLanguage::Ruby),
            ("rust", "cdd-rust", TargetLanguage::Rust),
            ("sh", "cdd-sh", TargetLanguage::Sh),
            ("swift", "cdd-swift", TargetLanguage::Swift),
            ("ts", "cdd-ts", TargetLanguage::Ts),
            ("zig", "cdd-zig", TargetLanguage::Custom("zig".to_string())),
        ];

        for (input, expected_daemon, expected_variant) in cases {
            let parsed = TargetLanguage::parse(input);
            assert_eq!(parsed, expected_variant);
            assert_eq!(parsed.daemon_name(), expected_daemon);
            assert_eq!(parsed.to_string(), expected_daemon);

            let serialized = serde_json::to_string(&parsed)?;
            let deserialized: TargetLanguage = serde_json::from_str(&serialized)?;
            assert_eq!(deserialized, expected_variant);
        }

        Ok(())
    }

    /// Verifies serialization and deserialization of [`McpRequest`].
    #[test]
    fn test_mcp_request_serde() -> Result<(), CddEngineError> {
        let req = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(RequestId::Number(1)),
        };
        let json = serde_json::to_string(&req)?;
        let decoded: McpRequest = serde_json::from_str(&json)?;
        assert_eq!(req, decoded);
        Ok(())
    }

    /// Verifies [`McpResponse::success`], [`McpResponse::from_error`], and serde round-trips.
    #[test]
    fn test_mcp_response_helpers_and_serde() -> Result<(), CddEngineError> {
        let tool_res = CallToolResult {
            content: vec![ToolContent::Text {
                text: "generated".to_string(),
            }],
            is_error: Some(false),
        };
        let res = McpResponse::success(Some(RequestId::Number(7)), &tool_res)?;
        assert_eq!(res.id, Some(RequestId::Number(7)));
        assert!(res.error.is_none());
        assert!(res.result.is_some());

        let json = serde_json::to_string(&res)?;
        let decoded: McpResponse = serde_json::from_str(&json)?;
        assert_eq!(res, decoded);

        let err = CddEngineError::Validation("bad field".into());
        let err_res = McpResponse::from_error(Some(RequestId::from("id-1")), &err);
        assert!(err_res.result.is_none());
        assert_eq!(
            err_res.error,
            Some(JsonRpcErrorObject {
                code: JSONRPC_INVALID_PARAMS,
                message: "Validation Error: bad field".to_string(),
                data: None,
            })
        );

        Ok(())
    }

    /// Verifies `Clone`, `Debug`, and `PartialEq` derives across MCP types.
    #[test]
    fn test_mcp_derives() {
        let req = McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "ping".to_string(),
            params: None,
            id: Some(RequestId::Number(1)),
        };
        let cloned = req.clone();
        assert_eq!(req, cloned);

        let mut altered_request = req.clone();
        altered_request.method = "initialize".to_string();
        assert_ne!(req, altered_request);
        assert!(format!("{req:?}").contains("McpRequest"));

        let err_obj = JsonRpcErrorObject {
            code: -32600,
            message: "Invalid".to_string(),
            data: None,
        };
        let res = McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(err_obj.clone()),
            id: Some(RequestId::String("abc".to_string())),
        };
        let cloned_res = res.clone();
        assert_eq!(res, cloned_res);

        let mut modified_response = res.clone();
        modified_response.jsonrpc = "1.0".to_string();
        assert_ne!(res, modified_response);
        assert!(format!("{res:?}").contains("McpResponse"));
        assert!(format!("{err_obj:?}").contains("JsonRpcErrorObject"));
    }
}
