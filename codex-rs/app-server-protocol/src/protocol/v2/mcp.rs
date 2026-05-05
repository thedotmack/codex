use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ListMcpServerStatusParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to a server-defined value.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Controls how much MCP inventory data to fetch for each server.
    /// Defaults to `Full` when omitted.
    #[ts(optional = nullable)]
    pub detail: Option<McpServerStatusDetail>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum McpServerStatusDetail {
    Full,
    ToolsAndAuthOnly,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerStatus {
    pub name: String,
    pub tools: std::collections::HashMap<String, McpTool>,
    pub resources: Vec<McpResource>,
    pub resource_templates: Vec<McpResourceTemplate>,
    pub auth_status: McpAuthStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ListMcpServerStatusResponse {
    pub data: Vec<McpServerStatus>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// If None, there are no more items to return.
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpResourceReadParams {
    #[ts(optional = nullable)]
    pub thread_id: Option<String>,
    pub server: String,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpResourceReadResponse {
    pub contents: Vec<McpResourceContent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerToolCallParams {
    pub thread_id: String,
    pub server: String,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub arguments: Option<JsonValue>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerToolCallResponse {
    pub content: Vec<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub structured_content: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub is_error: Option<bool>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<JsonValue>,
}

impl From<CoreMcpCallToolResult> for McpServerToolCallResponse {
    fn from(result: CoreMcpCallToolResult) -> Self {
        Self {
            content: result.content,
            structured_content: result.structured_content,
            is_error: result.is_error,
            meta: result.meta,
        }
    }
}

impl From<CoreMcpCallToolResult> for McpToolCallResult {
    fn from(result: CoreMcpCallToolResult) -> Self {
        Self {
            content: result.content,
            structured_content: result.structured_content,
            meta: result.meta,
        }
    }
}

impl From<CoreMcpToolCallError> for McpToolCallError {
    fn from(error: CoreMcpToolCallError) -> Self {
        Self {
            message: error.message,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerRefreshParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerRefreshResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerOauthLoginParams {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub scopes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub timeout_secs: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerOauthLoginResponse {
    pub authorization_url: String,
}
