# Model Context Protocol (MCP) SDK Conformance Table

This table tracks the completeness of language SDK integration with the Model Context Protocol (MCP). It is divided into three sections:
1. **Architectural Integration Layers**: Tracks the exposure of MCP across the CLI, SDK, and Server boundaries.
2. **Semantic & Conceptual Features**: Tracks protocol mechanics, transports, and behavioral requirements.
3. **Schema & Object Conformance**: An exhaustive property-by-property map derived directly from the official MCP JSON Schema (2024-11-05).

### Legend & Tracking Guide
*   **To**: Language -> MCP (Generating MCP Server payloads and handling requests from strongly typed code)
*   **From**: MCP -> Language (Generating MCP Client code, parsing responses, and invoking remote methods)
*   **Presence `[To, From]`**: The object/feature is successfully parsed, validated, utilized, or generated.
*   **Absence `[To, From]`**: The object/feature is currently unsupported, dropped, or falls back to generic/`any` types.
*   **Skipped `[To, From]`**: Intentionally ignored because it is irrelevant or unsupported by the Client architecture.
*   **Checkboxes**: Mark `[x]` as conformance is achieved.

## 1. Architectural Integration Layers

This section tracks how the Model Context Protocol is exposed across both the **Generated Artifacts** (the output SDKs/APIs) and the **Generator Tooling** itself (the bidirectional `cdd` compiler/engine).

### 1A. Target/Generated Artifacts
Implementing MCP across the generated output ensures maximum flexibility for the end-user's AI architectures:

*   **CLI Integration (Local Desktop via `stdio`)**: Enables local AI assistants (Claude Desktop, Cursor, Windsurf) to spawn the generated CLI as a subprocess and natively interact with the API locally.
*   **SDK Integration (Programmatic / In-Memory)**: Provides native adapters (e.g., `client.mcp.get_tools()`) so developers can seamlessly attach the generated SDK to frameworks like LangChain, LlamaIndex, or raw LLM clients without network overhead.
*   **Server Integration (Remote AI Gateway via `sse`)**: Generates an AI Gateway endpoint (e.g., `/mcp/sse`), allowing remote, multi-tenant AI agents and web clients to securely consume the API as LLM tools over HTTP.

| Generated Boundary | Presence `[To, From]` | Absence `[To, From]` | Skipped `[To, From]` | Notes / Implementation Strategy |
| :--- | :---: | :---: | :---: | :--- |
| **CLI Integration (Local Desktop)** | | | | |
| CLI `mcp` Subcommand | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Generates a command (e.g., `app mcp`) to start the server |
| `stdio` Transport Bindings | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Wires stdin/stdout to the generated CLI logic |
| **SDK Integration (Programmatic)** | | | | |
| Native MCP Tool Adapter | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | E.g., `client.mcp.get_tools()` mapping SDK methods |
| Native MCP Resource Adapter | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Exposes internal state/docs as MCP resources |
| LLM Execution Router | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Native execution via `client.mcp.execute_tool(name, args)` |
| **Server Integration (Remote / SSE)** | | | | |
| SSE Endpoint Generation | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Wires MCP endpoints (e.g. `/mcp/sse`, `/mcp/message`) |
| HTTP Request/Auth Bridging | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Passes standard API auth into the MCP context |
| Dynamic API-to-Tool Proxy | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Resolves incoming tool calls to backend route handlers |

### 1B. Generator/Tooling Artifacts (Meta-MCP)
Exposing the `cdd` bidirectional code generator itself to MCP allows AI models to natively orchestrate code generation, schema manipulation, and code-to-schema extraction.

*   **Generator CLI via `stdio`**: Allows local IDEs or AI agents to directly instruct the generator to scaffold, diff, or compile code across languages (e.g., Tool: `cdd_generate(lang="python")`).
*   **Generator SDK / Core**: Exposes the AST and schema parsing engine natively to MCP, allowing AI tools to dynamically query API specs, understand types, and invoke generator internals in memory.

| Generator Boundary | Presence `[To, From]` | Absence `[To, From]` | Skipped `[To, From]` | Notes / Implementation Strategy |
| :--- | :---: | :---: | :---: | :--- |
| **Generator CLI (`stdio`)** | | | | |
| Code Scaffold / Generate Tools | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | AI can invoke standard generator CLI commands via MCP |
| Schema Inspection Tools | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | AI can query loaded OpenAPI/AsyncAPI schemas |
| Bidirectional Sync Tools | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | AI can trigger code-to-schema extraction natively |
| **Generator SDK / Core** | | | | |
| AST / Type Query Resources | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | AI can read internal AST structures as MCP resources |
| In-Memory Generation Router | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Native bindings to run the generator core directly via MCP |

## 2. Semantic & Conceptual Features

| MCP Feature / Behavior | Presence `[To, From]` | Absence `[To, From]` | Skipped `[To, From]` | Notes / Implementation Strategy |
| :--- | :---: | :---: | :---: | :--- |
| **Transports** | | | | |
| Standard I/O (stdio) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | stdin/stdout message passing |
| Server-Sent Events (sse) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | HTTP POST + SSE streams |
| Custom Transports | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Pluggable transport interface |
| **JSON-RPC 2.0 Mechanics** | | | | |
| Message Parsing & Serialization | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Request ID Mapping/Resolution | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Resolving async responses to requests |
| Error Code Mapping (Standard) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Codes like -32600, -32603 |
| Notification Handling | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Processing fire-and-forget messages |
| **Connection Lifecycle** | | | | |
| initialize Handshake Sequence | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Capability negotiation & version matching |
| initialized Acknowledgment | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Sent by client after successful initialization |
| Graceful Disconnect / Close | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Liveness (ping) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Periodic connection checks |
| Request Cancellation (cancelled)| `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Thread/Task abortion mechanics |
| **Behavioral & Security** | | | | |
| Pagination Cursor Management | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Handling nextCursor fetch loops |
| Progress Tracking (progress) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Emitting/handling progress events |
| Human-in-the-loop (Sampling) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Prompting user before LLM generation |
| Human-in-the-loop (Tools) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Security approvals/denials for tool calls |
| Root Boundary Enforcement | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Preventing traversal outside allowed directories |
| URI Protocol Handling | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | Resolving custom URI schemes |

## 3. Schema & Object Conformance

| Schema Definition / Property | Presence `[To, From]` | Absence `[To, From]` | Skipped `[To, From]` | Notes |
| :--- | :---: | :---: | :---: | :--- |
| **Annotated** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Annotated (`annotations`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Annotated (`annotations`) (`audience`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Annotated (`annotations`) (`priority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **BlobResourceContents** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| BlobResourceContents (`blob`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| BlobResourceContents (`mimeType`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| BlobResourceContents (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CallToolRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolRequest (`params`) (`arguments`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolRequest (`params`) (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CallToolResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolResult (`content`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CallToolResult (`isError`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CancelledNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CancelledNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CancelledNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CancelledNotification (`params`) (`reason`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CancelledNotification (`params`) (`requestId`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ClientCapabilities** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ClientCapabilities (`experimental`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ClientCapabilities (`roots`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ClientCapabilities (`roots`) (`listChanged`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ClientCapabilities (`sampling`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ClientNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ClientRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ClientResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CompleteRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteRequest (`params`) (`argument`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteRequest (`params`) (`argument`) (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteRequest (`params`) (`argument`) (`value`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteRequest (`params`) (`ref`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CompleteResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteResult (`completion`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteResult (`completion`) (`hasMore`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteResult (`completion`) (`total`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CompleteResult (`completion`) (`values`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CreateMessageRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`includeContext`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`maxTokens`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`messages`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`metadata`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`modelPreferences`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`stopSequences`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`systemPrompt`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageRequest (`params`) (`temperature`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **CreateMessageResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageResult (`content`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageResult (`model`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageResult (`role`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| CreateMessageResult (`stopReason`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Cursor** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **EmbeddedResource** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| EmbeddedResource (`annotations`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| EmbeddedResource (`annotations`) (`audience`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| EmbeddedResource (`annotations`) (`priority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| EmbeddedResource (`resource`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| EmbeddedResource (`type`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **EmptyResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **GetPromptRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptRequest (`params`) (`arguments`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptRequest (`params`) (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **GetPromptResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptResult (`description`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| GetPromptResult (`messages`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ImageContent** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ImageContent (`annotations`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ImageContent (`annotations`) (`audience`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ImageContent (`annotations`) (`priority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ImageContent (`data`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ImageContent (`mimeType`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ImageContent (`type`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Implementation** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Implementation (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Implementation (`version`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **InitializeRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeRequest (`params`) (`capabilities`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeRequest (`params`) (`clientInfo`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeRequest (`params`) (`protocolVersion`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **InitializeResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeResult (`capabilities`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeResult (`instructions`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeResult (`protocolVersion`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializeResult (`serverInfo`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **InitializedNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializedNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializedNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| InitializedNotification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **JSONRPCError** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCError (`error`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCError (`error`) (`code`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCError (`error`) (`data`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCError (`error`) (`message`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCError (`id`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCError (`jsonrpc`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **JSONRPCMessage** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **JSONRPCNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCNotification (`jsonrpc`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCNotification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **JSONRPCRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCRequest (`id`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCRequest (`jsonrpc`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCRequest (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCRequest (`params`) (`_meta`) (`progressToken`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **JSONRPCResponse** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCResponse (`id`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCResponse (`jsonrpc`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| JSONRPCResponse (`result`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListPromptsRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListPromptsRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListPromptsRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListPromptsRequest (`params`) (`cursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListPromptsResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListPromptsResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListPromptsResult (`nextCursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListPromptsResult (`prompts`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListResourceTemplatesRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourceTemplatesRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourceTemplatesRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourceTemplatesRequest (`params`) (`cursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListResourceTemplatesResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourceTemplatesResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourceTemplatesResult (`nextCursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourceTemplatesResult (`resourceTemplates`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListResourcesRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourcesRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourcesRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourcesRequest (`params`) (`cursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListResourcesResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourcesResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourcesResult (`nextCursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListResourcesResult (`resources`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListRootsRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListRootsRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListRootsRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListRootsRequest (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListRootsRequest (`params`) (`_meta`) (`progressToken`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListRootsResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListRootsResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListRootsResult (`roots`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListToolsRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListToolsRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListToolsRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListToolsRequest (`params`) (`cursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ListToolsResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListToolsResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListToolsResult (`nextCursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ListToolsResult (`tools`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **LoggingLevel** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **LoggingMessageNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| LoggingMessageNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| LoggingMessageNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| LoggingMessageNotification (`params`) (`data`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| LoggingMessageNotification (`params`) (`level`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| LoggingMessageNotification (`params`) (`logger`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ModelHint** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ModelHint (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ModelPreferences** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ModelPreferences (`costPriority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ModelPreferences (`hints`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ModelPreferences (`intelligencePriority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ModelPreferences (`speedPriority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Notification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Notification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Notification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Notification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PaginatedRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PaginatedRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PaginatedRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PaginatedRequest (`params`) (`cursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PaginatedResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PaginatedResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PaginatedResult (`nextCursor`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PingRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PingRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PingRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PingRequest (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PingRequest (`params`) (`_meta`) (`progressToken`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ProgressNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ProgressNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ProgressNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ProgressNotification (`params`) (`progressToken`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ProgressNotification (`params`) (`progress`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ProgressNotification (`params`) (`total`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ProgressToken** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Prompt** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Prompt (`arguments`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Prompt (`description`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Prompt (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PromptArgument** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptArgument (`description`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptArgument (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptArgument (`required`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PromptListChangedNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptListChangedNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptListChangedNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptListChangedNotification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PromptMessage** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptMessage (`content`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptMessage (`role`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **PromptReference** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptReference (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| PromptReference (`type`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ReadResourceRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ReadResourceRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ReadResourceRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ReadResourceRequest (`params`) (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ReadResourceResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ReadResourceResult (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ReadResourceResult (`contents`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Request** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Request (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Request (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Request (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Request (`params`) (`_meta`) (`progressToken`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **RequestId** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Resource** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`annotations`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`annotations`) (`audience`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`annotations`) (`priority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`description`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`mimeType`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`size`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Resource (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ResourceContents** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceContents (`mimeType`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceContents (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ResourceListChangedNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceListChangedNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceListChangedNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceListChangedNotification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ResourceReference** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceReference (`type`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceReference (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ResourceTemplate** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`annotations`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`annotations`) (`audience`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`annotations`) (`priority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`description`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`mimeType`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceTemplate (`uriTemplate`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ResourceUpdatedNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceUpdatedNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceUpdatedNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ResourceUpdatedNotification (`params`) (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Result** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Result (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Role** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Root** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Root (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Root (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **RootsListChangedNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| RootsListChangedNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| RootsListChangedNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| RootsListChangedNotification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **SamplingMessage** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SamplingMessage (`content`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SamplingMessage (`role`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ServerCapabilities** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`experimental`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`logging`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`prompts`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`prompts`) (`listChanged`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`resources`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`resources`) (`listChanged`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`resources`) (`subscribe`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`tools`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ServerCapabilities (`tools`) (`listChanged`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ServerNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ServerRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ServerResult** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **SetLevelRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SetLevelRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SetLevelRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SetLevelRequest (`params`) (`level`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **SubscribeRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SubscribeRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SubscribeRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| SubscribeRequest (`params`) (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **TextContent** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextContent (`annotations`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextContent (`annotations`) (`audience`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextContent (`annotations`) (`priority`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextContent (`text`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextContent (`type`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **TextResourceContents** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextResourceContents (`mimeType`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextResourceContents (`text`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| TextResourceContents (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **Tool** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Tool (`description`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Tool (`inputSchema`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Tool (`inputSchema`) (`properties`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Tool (`inputSchema`) (`required`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Tool (`inputSchema`) (`type`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| Tool (`name`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **ToolListChangedNotification** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ToolListChangedNotification (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ToolListChangedNotification (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| ToolListChangedNotification (`params`) (`_meta`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| **UnsubscribeRequest** | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| UnsubscribeRequest (`method`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| UnsubscribeRequest (`params`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
| UnsubscribeRequest (`params`) (`uri`) | `[ ]` , `[ ]` | `[ ]` , `[ ]` | `[ ]` , `[ ]` | |
