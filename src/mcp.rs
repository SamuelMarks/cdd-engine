#![deny(missing_docs)]

//! Model Context Protocol (MCP) JSON-RPC 2.0 domain types and orchestrator trait.
//!
//! Includes specification format detection ([`SpecFormat`]), compiler options
//! ([`CompilerOptions`], [`GeneratorTarget`]), repository layout partitioning
//! ([`RepoLayoutMode`], [`RepoLayoutConfig`]), and strongly typed MCP primitives
//! ([`RequestId`], [`TargetLanguage`], [`JsonRpcErrorObject`], [`McpTool`], [`CallToolParams`],
//! [`CallToolResult`], [`McpRequest`], [`McpResponse`], [`McpOrchestrator`]).

use crate::error::CddEngineError;
use crate::wasm_executor::GeneratedFile;
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

/// Supported API specification and workflow input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecFormat {
    /// `OpenAPI` `3.0.x`, `3.1.x`, or `3.2.x` specification (`openapi: "3.x.x"`).
    #[serde(rename = "openapi3")]
    OpenApi3,
    /// Swagger `2.0` specification (`swagger: "2.0"`).
    #[serde(rename = "swagger2")]
    Swagger2,
    /// Arazzo `1.0.x` or `1.1.0` workflow specification (`arazzo: "1.x.x"`).
    #[serde(rename = "arazzo1")]
    Arazzo1,
    /// Google API Discovery v1 document (`discoveryVersion: "v1"` or `kind: "discovery#restDescription"`).
    #[serde(rename = "google_discovery1")]
    GoogleDiscovery1,
}

impl SpecFormat {
    /// Inspects a raw JSON or YAML specification string and determines its [`SpecFormat`].
    ///
    /// # Arguments
    ///
    /// * `raw_spec` - The raw specification content in JSON or YAML format.
    ///
    /// # Returns
    ///
    /// The detected [`SpecFormat`] variant.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::InvalidSpecification`] if `raw_spec` is empty, not a JSON/YAML
    /// mapping, or lacks a recognized specification version header.
    pub fn detect(raw_spec: &str) -> Result<Self, CddEngineError> {
        let trimmed = raw_spec.trim();
        if trimmed.is_empty() {
            return Err(CddEngineError::InvalidSpecification(
                "Specification content is empty".to_string(),
            ));
        }

        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return Self::detect_from_json(&json_val);
        }

        Self::detect_from_yaml(trimmed)
    }

    /// Detects the [`SpecFormat`] from a parsed [`serde_json::Value`].
    ///
    /// # Arguments
    ///
    /// * `value` - The parsed JSON value.
    ///
    /// # Returns
    ///
    /// The detected [`SpecFormat`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::InvalidSpecification`] if `value` is not a JSON object or does
    /// not contain a supported specification header.
    fn detect_from_json(value: &serde_json::Value) -> Result<Self, CddEngineError> {
        let Some(obj) = value.as_object() else {
            return Err(CddEngineError::InvalidSpecification(
                "Specification root must be a JSON object".to_string(),
            ));
        };

        if let Some(openapi_ver) = obj.get("openapi").and_then(serde_json::Value::as_str) {
            if openapi_ver.starts_with("3.") {
                return Ok(Self::OpenApi3);
            }
        }

        if let Some(swagger_ver) = obj.get("swagger").and_then(serde_json::Value::as_str) {
            if swagger_ver == "2.0" {
                return Ok(Self::Swagger2);
            }
        }

        if let Some(arazzo_ver) = obj.get("arazzo").and_then(serde_json::Value::as_str) {
            if arazzo_ver.starts_with("1.") {
                return Ok(Self::Arazzo1);
            }
        }

        if obj
            .get("discoveryVersion")
            .and_then(serde_json::Value::as_str)
            == Some("v1")
            || obj.get("kind").and_then(serde_json::Value::as_str)
                == Some("discovery#restDescription")
        {
            return Ok(Self::GoogleDiscovery1);
        }

        Err(CddEngineError::InvalidSpecification(
            "Unrecognized JSON specification header (expected openapi 3.x, swagger 2.0, arazzo 1.x, or Google Discovery v1)".to_string(),
        ))
    }

    /// Detects the [`SpecFormat`] from a line-oriented YAML document.
    ///
    /// # Arguments
    ///
    /// * `yaml_text` - The trimmed YAML text.
    ///
    /// # Returns
    ///
    /// The detected [`SpecFormat`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::InvalidSpecification`] if no recognized top-level YAML header is
    /// found.
    fn detect_from_yaml(yaml_text: &str) -> Result<Self, CddEngineError> {
        let quotes: &[char] = &['"', '\''];
        for line in yaml_text.lines() {
            let trimmed_line = line.trim();
            if let Some((raw_key, raw_val)) = trimmed_line.split_once(':') {
                let key = raw_key.trim().trim_matches(quotes);
                let without_comment = raw_val
                    .split_once(" #")
                    .map_or(raw_val, |(before, _)| before);
                let val = without_comment.trim().trim_matches(quotes);
                if key == "openapi" && val.starts_with("3.") {
                    return Ok(Self::OpenApi3);
                }
                if key == "swagger" && val == "2.0" {
                    return Ok(Self::Swagger2);
                }
                if key == "arazzo" && val.starts_with("1.") {
                    return Ok(Self::Arazzo1);
                }
                if (key == "discoveryVersion" && val == "v1")
                    || (key == "kind" && val == "discovery#restDescription")
                {
                    return Ok(Self::GoogleDiscovery1);
                }
            }
        }

        Err(CddEngineError::InvalidSpecification(
            "Unrecognized specification format (expected OpenAPI 3.x, Swagger 2.0, Arazzo 1.x, or Google Discovery v1)".to_string(),
        ))
    }

    /// Returns the CLI source subcommand corresponding to this [`SpecFormat`].
    ///
    /// # Returns
    ///
    /// `"from_openapi"` for [`SpecFormat::OpenApi3`] and [`SpecFormat::Swagger2`],
    /// `"from_arazzo"` for [`SpecFormat::Arazzo1`], and `"from_google_discovery"` for
    /// [`SpecFormat::GoogleDiscovery1`].
    #[must_use]
    pub const fn cli_source_subcommand(self) -> &'static str {
        match self {
            Self::OpenApi3 | Self::Swagger2 => "from_openapi",
            Self::Arazzo1 => "from_arazzo",
            Self::GoogleDiscovery1 => "from_google_discovery",
        }
    }

    /// Returns the canonical guest input filename for this [`SpecFormat`].
    ///
    /// # Returns
    ///
    /// `"openapi.json"` for `OpenApi3`/`Swagger2`, `"arazzo.json"` for `Arazzo1`, and
    /// `"discovery.json"` for `GoogleDiscovery1`.
    #[must_use]
    pub const fn default_input_filename(self) -> &'static str {
        match self {
            Self::OpenApi3 | Self::Swagger2 => "openapi.json",
            Self::Arazzo1 => "arazzo.json",
            Self::GoogleDiscovery1 => "discovery.json",
        }
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
    /// Returns all 13 canonical target language ecosystems in the `cdd-*` toolchain.
    ///
    /// # Returns
    ///
    /// A fixed-size array containing the 13 standard [`TargetLanguage`] variants.
    #[must_use]
    pub const fn all_standard() -> [Self; 13] {
        [
            Self::C,
            Self::Cpp,
            Self::CSharp,
            Self::Go,
            Self::Java,
            Self::Kotlin,
            Self::Php,
            Self::PythonAll,
            Self::Ruby,
            Self::Rust,
            Self::Sh,
            Self::Swift,
            Self::Ts,
        ]
    }

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

/// Target artifact kind produced by a `cdd-*` language compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GeneratorTarget {
    /// Generate an installable client SDK library (`to_sdk`).
    #[default]
    #[serde(rename = "to_sdk")]
    ToSdk,
    /// Generate a command-line interface wrapping the client SDK (`to_sdk_cli`).
    #[serde(rename = "to_sdk_cli")]
    ToSdkCli,
    /// Generate a server stub / route handler scaffold (`to_server`).
    #[serde(rename = "to_server")]
    ToServer,
    /// Generate structured documentation JSON (`to_docs_json`).
    #[serde(rename = "to_docs_json")]
    ToDocsJson,
    /// Convert or extract an `OpenAPI` 3.2.0 specification (`to_openapi`).
    #[serde(rename = "to_openapi")]
    ToOpenApi,
}

impl GeneratorTarget {
    /// Returns the canonical CLI subcommand token for this [`GeneratorTarget`].
    ///
    /// # Returns
    ///
    /// The static string slice (`"to_sdk"`, `"to_sdk_cli"`, `"to_server"`, `"to_docs_json"`, or
    /// `"to_openapi"`).
    #[must_use]
    pub const fn as_cli_str(self) -> &'static str {
        match self {
            Self::ToSdk => "to_sdk",
            Self::ToSdkCli => "to_sdk_cli",
            Self::ToServer => "to_server",
            Self::ToDocsJson => "to_docs_json",
            Self::ToOpenApi => "to_openapi",
        }
    }
}

/// Strongly typed per-language compiler flags and generation options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct CompilerOptions {
    /// Target generation mode (`to_sdk`, `to_sdk_cli`, `to_server`, `to_docs_json`, `to_openapi`).
    #[serde(default)]
    pub target: GeneratorTarget,
    /// Whether to emit Model Context Protocol (`--mcp`) bindings across the generated output.
    #[serde(default)]
    pub mcp: bool,
    /// Optional target web/client framework (e.g., `"angular"`, `"axum"`, `"fastapi"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    /// Optional target ORM or data-access layer (e.g., `"diesel"`, `"sqlalchemy"`, `"prisma"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orm: Option<String>,
    /// Whether to generate an automatic admin interface (`--auto-admin`).
    #[serde(default)]
    pub auto_admin: bool,
    /// Whether to emit generated unit/conformance test suites (`--tests`).
    #[serde(default)]
    pub tests: bool,
    /// Whether to omit `.github/workflows` CI scaffolding (`--no-github-actions`).
    #[serde(default)]
    pub no_github_actions: bool,
    /// Whether to omit package manifest files (`--no-installable-package`).
    #[serde(default)]
    pub no_installable_package: bool,
    /// Whether to omit top-level imports in generated snippets (`--no-imports`).
    #[serde(default)]
    pub no_imports: bool,
    /// Whether to omit outer class/function wrapping in generated snippets (`--no-wrapping`).
    #[serde(default)]
    pub no_wrapping: bool,
    /// Additional pass-through CLI arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
}

impl CompilerOptions {
    /// Constructs the deterministic CLI argument vector for a `cdd-*` compiler invocation.
    ///
    /// # Arguments
    ///
    /// * `spec_format` - The input [`SpecFormat`] determining the source subcommand.
    /// * `input_path` - Path to the input specification inside the execution environment.
    /// * `output_path` - Path to the output directory or file inside the execution environment.
    ///
    /// # Returns
    ///
    /// Ordered `Vec<String>` of CLI arguments ready to pass to a WASI guest or native daemon.
    #[must_use]
    pub fn to_cli_args(
        &self,
        spec_format: SpecFormat,
        input_path: &str,
        output_path: &str,
    ) -> Vec<String> {
        let mut args = vec![
            spec_format.cli_source_subcommand().to_string(),
            self.target.as_cli_str().to_string(),
            "-i".to_string(),
            input_path.to_string(),
            "-o".to_string(),
            output_path.to_string(),
        ];

        if self.mcp {
            args.push("--mcp".to_string());
        }
        if let Some(fw) = &self.framework {
            args.push("--framework".to_string());
            args.push(fw.clone());
        }
        if let Some(orm_val) = &self.orm {
            args.push("--orm".to_string());
            args.push(orm_val.clone());
        }
        if self.auto_admin {
            args.push("--auto-admin".to_string());
        }
        if self.tests {
            args.push("--tests".to_string());
        }
        if self.no_github_actions {
            args.push("--no-github-actions".to_string());
        }
        if self.no_installable_package {
            args.push("--no-installable-package".to_string());
        }
        if self.no_imports {
            args.push("--no-imports".to_string());
        }
        if self.no_wrapping {
            args.push("--no-wrapping".to_string());
        }
        args.extend_from_slice(&self.extra_args);
        args
    }
}

/// Repository output layout mode for multi-language SDK batch generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RepoLayoutMode {
    /// Single monorepo layout: `<org_name>-sdk/<language_slug>/<file_path>`.
    #[default]
    #[serde(rename = "monorepo")]
    Monorepo,
    /// Separate polyrepo layout per language: `<org_name>-sdk-<language_slug>/<file_path>`.
    #[serde(rename = "polyrepo")]
    Polyrepo,
}

/// Configuration governing how generated files are partitioned into monorepo or polyrepo paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoLayoutConfig {
    /// Organization or project prefix (e.g., `"acme"` -> `"acme-sdk"` or `"acme-sdk-rust"`).
    pub org_name: String,
    /// Layout mode (`Monorepo` or `Polyrepo`).
    #[serde(default)]
    pub mode: RepoLayoutMode,
}

impl RepoLayoutConfig {
    /// Creates a validated [`RepoLayoutConfig`].
    ///
    /// # Arguments
    ///
    /// * `org_name` - Non-empty organization or repository prefix.
    /// * `mode` - Desired [`RepoLayoutMode`] (`Monorepo` or `Polyrepo`).
    ///
    /// # Returns
    ///
    /// A validated [`RepoLayoutConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Validation`] if `org_name` is empty or whitespace-only.
    pub fn new(org_name: &str, mode: RepoLayoutMode) -> Result<Self, CddEngineError> {
        let trimmed = org_name.trim();
        if trimmed.is_empty() {
            return Err(CddEngineError::Validation(
                "Organization name for repository layout cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            org_name: trimmed.to_string(),
            mode,
        })
    }

    /// Computes the directory prefix for a specific [`TargetLanguage`].
    ///
    /// # Arguments
    ///
    /// * `language` - The target language being partitioned.
    ///
    /// # Returns
    ///
    /// `"<org_name>-sdk/<slug>"` for [`RepoLayoutMode::Monorepo`], or
    /// `"<org_name>-sdk-<slug>"` for [`RepoLayoutMode::Polyrepo`].
    #[must_use]
    pub fn repo_prefix_for_language(&self, language: &TargetLanguage) -> String {
        let slug = language.as_slug();
        match self.mode {
            RepoLayoutMode::Monorepo => format!("{}-sdk/{slug}", self.org_name),
            RepoLayoutMode::Polyrepo => format!("{}-sdk-{slug}", self.org_name),
        }
    }

    /// Partitions a slice of [`GeneratedFile`] entries under the configured repository layout.
    ///
    /// # Arguments
    ///
    /// * `language` - The [`TargetLanguage`] that produced `files`.
    /// * `files` - The generated files with paths relative to the language output root.
    ///
    /// # Returns
    ///
    /// A new `Vec<GeneratedFile>` with every file path prefixed by
    /// [`RepoLayoutConfig::repo_prefix_for_language`].
    #[must_use]
    pub fn partition_files(
        &self,
        language: &TargetLanguage,
        files: &[GeneratedFile],
    ) -> Vec<GeneratedFile> {
        let prefix = self.repo_prefix_for_language(language);
        files
            .iter()
            .map(|file| {
                let clean_rel = file.path.trim_start_matches('/');
                GeneratedFile {
                    path: format!("{prefix}/{clean_rel}"),
                    content: file.content.clone(),
                }
            })
            .collect()
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
    /// Input specification path or raw specification content (`OpenAPI`, Swagger, Arazzo, or Google Discovery).
    #[serde(default)]
    pub input: Option<String>,
    /// Optional output directory path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Strongly typed compiler options (`target`, `mcp`, `framework`, `orm`, etc.).
    #[serde(default, flatten)]
    pub options: CompilerOptions,
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
    /// Returns [`CddEngineError::Json`] if `payload` cannot be serialized to JSON.
    pub fn success<T: Serialize>(
        id: Option<RequestId>,
        payload: &T,
    ) -> Result<Self, CddEngineError> {
        let value = serde_json::to_value(payload)?;
        Ok(Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(value),
            error: None,
            id,
        })
    }

    /// Constructs an error [`McpResponse`] from a [`CddEngineError`].
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
    pub fn from_error(id: Option<RequestId>, err: &CddEngineError) -> Self {
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
    /// Returns [`CddEngineError`] if validation, routing, or daemon communication fails.
    fn handle_request(
        &self,
        request: McpRequest,
    ) -> impl std::future::Future<Output = Result<McpResponse, CddEngineError>> + Send;
}

#[cfg(test)]
mod tests {
    //! Unit tests for MCP protocol domain models, [`SpecFormat`], [`CompilerOptions`],
    //! [`RepoLayoutConfig`], [`RequestId`], and [`TargetLanguage`].

    use super::*;
    use crate::error::JSONRPC_INVALID_PARAMS;

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

    /// Verifies [`SpecFormat::detect`], [`SpecFormat::cli_source_subcommand`], and
    /// [`SpecFormat::default_input_filename`] across JSON, YAML, and invalid inputs.
    #[test]
    fn test_spec_format_detection_and_helpers() -> Result<(), CddEngineError> {
        // 1. JSON detection for all 4 formats
        let oa3_json = r#"{"openapi": "3.2.0", "info": {"title": "API", "version": "1.0"}}"#;
        assert_eq!(SpecFormat::detect(oa3_json)?, SpecFormat::OpenApi3);
        assert_eq!(SpecFormat::OpenApi3.cli_source_subcommand(), "from_openapi");
        assert_eq!(
            SpecFormat::OpenApi3.default_input_filename(),
            "openapi.json"
        );

        let sw2_json = r#"{"swagger": "2.0", "info": {"title": "Legacy", "version": "1.0"}}"#;
        assert_eq!(SpecFormat::detect(sw2_json)?, SpecFormat::Swagger2);
        assert_eq!(SpecFormat::Swagger2.cli_source_subcommand(), "from_openapi");
        assert_eq!(
            SpecFormat::Swagger2.default_input_filename(),
            "openapi.json"
        );

        let arazzo_json = r#"{"arazzo": "1.1.0", "info": {"title": "Flow", "version": "1.0"}}"#;
        assert_eq!(SpecFormat::detect(arazzo_json)?, SpecFormat::Arazzo1);
        assert_eq!(SpecFormat::Arazzo1.cli_source_subcommand(), "from_arazzo");
        assert_eq!(SpecFormat::Arazzo1.default_input_filename(), "arazzo.json");

        let disc_v1_json = r#"{"discoveryVersion": "v1", "name": "compute"}"#;
        assert_eq!(
            SpecFormat::detect(disc_v1_json)?,
            SpecFormat::GoogleDiscovery1
        );
        let disc_kind_json = r#"{"kind": "discovery#restDescription", "name": "storage"}"#;
        assert_eq!(
            SpecFormat::detect(disc_kind_json)?,
            SpecFormat::GoogleDiscovery1
        );
        assert_eq!(
            SpecFormat::GoogleDiscovery1.cli_source_subcommand(),
            "from_google_discovery"
        );
        assert_eq!(
            SpecFormat::GoogleDiscovery1.default_input_filename(),
            "discovery.json"
        );

        // 2. YAML detection for all 4 formats
        assert_eq!(
            SpecFormat::detect("openapi: 3.1.0\ninfo:\n  title: Test\n")?,
            SpecFormat::OpenApi3
        );
        assert_eq!(
            SpecFormat::detect("swagger: '2.0'\ninfo:\n  title: Test\n")?,
            SpecFormat::Swagger2
        );
        assert_eq!(
            SpecFormat::detect("arazzo: '1.1.0' # workflow\n")?,
            SpecFormat::Arazzo1
        );
        assert_eq!(
            SpecFormat::detect("discoveryVersion: v1\nname: calendar\n")?,
            SpecFormat::GoogleDiscovery1
        );
        assert_eq!(
            SpecFormat::detect("kind: discovery#restDescription\n")?,
            SpecFormat::GoogleDiscovery1
        );

        for fmt in [
            SpecFormat::OpenApi3,
            SpecFormat::Swagger2,
            SpecFormat::Arazzo1,
            SpecFormat::GoogleDiscovery1,
        ] {
            let s = serde_json::to_string(&fmt)?;
            let de: SpecFormat = serde_json::from_str(&s)?;
            assert_eq!(de, fmt);
        }

        Ok(())
    }

    /// Verifies [`SpecFormat::detect`] rejection of empty, malformed, and unrecognized inputs.
    #[test]
    fn test_spec_format_detect_invalid_inputs() {
        let invalid_cases = [
            "   ",
            "[1, 2, 3]",
            r#"{"openapi": "2.0"}"#,
            r#"{"swagger": "1.2"}"#,
            r#"{"arazzo": "2.0.0"}"#,
            r#"{"discoveryVersion": "v2", "kind": "other"}"#,
            "openapi: 2.0\nsome_other_key: value\njust_a_line_without_colon",
            "swagger: 3.0\narazzo: 0.9\ndiscoveryVersion: v2\nkind: other",
        ];

        for invalid in invalid_cases {
            assert!(matches!(
                SpecFormat::detect(invalid),
                Err(CddEngineError::InvalidSpecification(_))
            ));
        }
    }

    /// Verifies [`TargetLanguage`] parsing, slug generation, daemon naming, `all_standard`, and
    /// serde round-trips across all 13 canonical ecosystems plus `python` and custom targets.
    #[test]
    fn test_target_language_all_variants() -> Result<(), CddEngineError> {
        let standard = TargetLanguage::all_standard();
        assert_eq!(standard.len(), 13);

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

    /// Verifies [`GeneratorTarget`] and [`CompilerOptions::to_cli_args`] across all flag combinations.
    #[test]
    fn test_generator_target_and_compiler_options() -> Result<(), CddEngineError> {
        let targets = [
            (GeneratorTarget::ToSdk, "to_sdk"),
            (GeneratorTarget::ToSdkCli, "to_sdk_cli"),
            (GeneratorTarget::ToServer, "to_server"),
            (GeneratorTarget::ToDocsJson, "to_docs_json"),
            (GeneratorTarget::ToOpenApi, "to_openapi"),
        ];
        for (target, expected_str) in targets {
            assert_eq!(target.as_cli_str(), expected_str);
            let json = serde_json::to_string(&target)?;
            let decoded: GeneratorTarget = serde_json::from_str(&json)?;
            assert_eq!(decoded, target);
        }

        let default_opts = CompilerOptions::default();
        assert_eq!(default_opts.target, GeneratorTarget::ToSdk);
        let base_args = default_opts.to_cli_args(
            SpecFormat::OpenApi3,
            "/workspace/openapi.json",
            "/workspace/out",
        );
        assert_eq!(
            base_args,
            vec![
                "from_openapi",
                "to_sdk",
                "-i",
                "/workspace/openapi.json",
                "-o",
                "/workspace/out"
            ]
        );

        let full_opts = CompilerOptions {
            target: GeneratorTarget::ToServer,
            mcp: true,
            framework: Some("angular".to_string()),
            orm: Some("prisma".to_string()),
            auto_admin: true,
            tests: true,
            no_github_actions: true,
            no_installable_package: true,
            no_imports: true,
            no_wrapping: true,
            extra_args: vec!["--custom".to_string()],
        };
        let full_args = full_opts.to_cli_args(
            SpecFormat::Arazzo1,
            "/workspace/arazzo.json",
            "/workspace/out",
        );
        assert_eq!(
            full_args,
            vec![
                "from_arazzo",
                "to_server",
                "-i",
                "/workspace/arazzo.json",
                "-o",
                "/workspace/out",
                "--mcp",
                "--framework",
                "angular",
                "--orm",
                "prisma",
                "--auto-admin",
                "--tests",
                "--no-github-actions",
                "--no-installable-package",
                "--no-imports",
                "--no-wrapping",
                "--custom",
            ]
        );

        let serialized = serde_json::to_string(&full_opts)?;
        let deserialized: CompilerOptions = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, full_opts);

        Ok(())
    }

    /// Verifies [`RepoLayoutMode`] and [`RepoLayoutConfig`] for both monorepo and polyrepo layouts.
    #[test]
    fn test_repo_layout_partitioning() -> Result<(), CddEngineError> {
        assert_eq!(RepoLayoutMode::default(), RepoLayoutMode::Monorepo);
        assert!(matches!(
            RepoLayoutConfig::new("   ", RepoLayoutMode::Monorepo),
            Err(CddEngineError::Validation(_))
        ));

        let files = vec![
            GeneratedFile {
                path: "Cargo.toml".to_string(),
                content: "[package]".to_string(),
            },
            GeneratedFile {
                path: "/src/lib.rs".to_string(),
                content: "pub mod api;".to_string(),
            },
        ];

        let mono = RepoLayoutConfig::new("acme", RepoLayoutMode::Monorepo)?;
        assert_eq!(
            mono.repo_prefix_for_language(&TargetLanguage::Rust),
            "acme-sdk/rust"
        );
        let mono_files = mono.partition_files(&TargetLanguage::Rust, &files);
        assert_eq!(mono_files.len(), 2);
        assert_eq!(mono_files[0].path, "acme-sdk/rust/Cargo.toml");
        assert_eq!(mono_files[1].path, "acme-sdk/rust/src/lib.rs");

        let poly = RepoLayoutConfig::new("acme", RepoLayoutMode::Polyrepo)?;
        assert_eq!(
            poly.repo_prefix_for_language(&TargetLanguage::PythonAll),
            "acme-sdk-python-all"
        );
        let poly_files = poly.partition_files(&TargetLanguage::PythonAll, &files);
        assert_eq!(poly_files[0].path, "acme-sdk-python-all/Cargo.toml");
        assert_eq!(poly_files[1].path, "acme-sdk-python-all/src/lib.rs");

        let mono_json = serde_json::to_string(&mono)?;
        let mono_de: RepoLayoutConfig = serde_json::from_str(&mono_json)?;
        assert_eq!(mono_de, mono);

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
