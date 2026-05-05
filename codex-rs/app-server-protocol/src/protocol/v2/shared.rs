use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum NonSteerableTurnKind {
    Review,
    Compact,
}

/// This translation layer make sure that we expose codex error code in camel case.
///
/// When an upstream HTTP status is available (for example, from the Responses API or a provider),
/// it is forwarded in `httpStatusCode` on the relevant `codexErrorInfo` variant.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum CodexErrorInfo {
    ContextWindowExceeded,
    UsageLimitExceeded,
    ServerOverloaded,
    CyberPolicy,
    HttpConnectionFailed {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Failed to connect to the response SSE stream.
    ResponseStreamConnectionFailed {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    InternalServerError,
    Unauthorized,
    BadRequest,
    ThreadRollbackFailed,
    SandboxError,
    /// The response SSE stream disconnected in the middle of a turn before completion.
    ResponseStreamDisconnected {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Reached the retry limit for responses.
    ResponseTooManyFailedAttempts {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Returned when `turn/start` or `turn/steer` is submitted while the current active turn
    /// cannot accept same-turn steering, for example `/review` or manual `/compact`.
    ActiveTurnNotSteerable {
        #[serde(rename = "turnKind")]
        #[ts(rename = "turnKind")]
        turn_kind: NonSteerableTurnKind,
    },
    Other,
}

impl From<CoreCodexErrorInfo> for CodexErrorInfo {
    fn from(value: CoreCodexErrorInfo) -> Self {
        match value {
            CoreCodexErrorInfo::ContextWindowExceeded => CodexErrorInfo::ContextWindowExceeded,
            CoreCodexErrorInfo::UsageLimitExceeded => CodexErrorInfo::UsageLimitExceeded,
            CoreCodexErrorInfo::ServerOverloaded => CodexErrorInfo::ServerOverloaded,
            CoreCodexErrorInfo::CyberPolicy => CodexErrorInfo::CyberPolicy,
            CoreCodexErrorInfo::HttpConnectionFailed { http_status_code } => {
                CodexErrorInfo::HttpConnectionFailed { http_status_code }
            }
            CoreCodexErrorInfo::ResponseStreamConnectionFailed { http_status_code } => {
                CodexErrorInfo::ResponseStreamConnectionFailed { http_status_code }
            }
            CoreCodexErrorInfo::InternalServerError => CodexErrorInfo::InternalServerError,
            CoreCodexErrorInfo::Unauthorized => CodexErrorInfo::Unauthorized,
            CoreCodexErrorInfo::BadRequest => CodexErrorInfo::BadRequest,
            CoreCodexErrorInfo::ThreadRollbackFailed => CodexErrorInfo::ThreadRollbackFailed,
            CoreCodexErrorInfo::SandboxError => CodexErrorInfo::SandboxError,
            CoreCodexErrorInfo::ResponseStreamDisconnected { http_status_code } => {
                CodexErrorInfo::ResponseStreamDisconnected { http_status_code }
            }
            CoreCodexErrorInfo::ResponseTooManyFailedAttempts { http_status_code } => {
                CodexErrorInfo::ResponseTooManyFailedAttempts { http_status_code }
            }
            CoreCodexErrorInfo::ActiveTurnNotSteerable { turn_kind } => {
                CodexErrorInfo::ActiveTurnNotSteerable {
                    turn_kind: turn_kind.into(),
                }
            }
            CoreCodexErrorInfo::Other => CodexErrorInfo::Other,
        }
    }
}

impl From<CoreNonSteerableTurnKind> for NonSteerableTurnKind {
    fn from(value: CoreNonSteerableTurnKind) -> Self {
        match value {
            CoreNonSteerableTurnKind::Review => Self::Review,
            CoreNonSteerableTurnKind::Compact => Self::Compact,
        }
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum AskForApproval {
    #[serde(rename = "untrusted")]
    #[ts(rename = "untrusted")]
    UnlessTrusted,
    OnFailure,
    OnRequest,
    #[experimental("askForApproval.granular")]
    Granular {
        sandbox_approval: bool,
        rules: bool,
        #[serde(default)]
        skill_approval: bool,
        #[serde(default)]
        request_permissions: bool,
        mcp_elicitations: bool,
    },
    Never,
}

impl AskForApproval {
    pub fn to_core(self) -> CoreAskForApproval {
        match self {
            AskForApproval::UnlessTrusted => CoreAskForApproval::UnlessTrusted,
            AskForApproval::OnFailure => CoreAskForApproval::OnFailure,
            AskForApproval::OnRequest => CoreAskForApproval::OnRequest,
            AskForApproval::Granular {
                sandbox_approval,
                rules,
                skill_approval,
                request_permissions,
                mcp_elicitations,
            } => CoreAskForApproval::Granular(CoreGranularApprovalConfig {
                sandbox_approval,
                rules,
                skill_approval,
                request_permissions,
                mcp_elicitations,
            }),
            AskForApproval::Never => CoreAskForApproval::Never,
        }
    }
}

impl From<CoreAskForApproval> for AskForApproval {
    fn from(value: CoreAskForApproval) -> Self {
        match value {
            CoreAskForApproval::UnlessTrusted => AskForApproval::UnlessTrusted,
            CoreAskForApproval::OnFailure => AskForApproval::OnFailure,
            CoreAskForApproval::OnRequest => AskForApproval::OnRequest,
            CoreAskForApproval::Granular(granular_config) => AskForApproval::Granular {
                sandbox_approval: granular_config.sandbox_approval,
                rules: granular_config.rules,
                skill_approval: granular_config.skill_approval,
                request_permissions: granular_config.request_permissions,
                mcp_elicitations: granular_config.mcp_elicitations,
            },
            CoreAskForApproval::Never => AskForApproval::Never,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, TS)]
#[ts(
    type = r#""user" | "auto_review" | "guardian_subagent""#,
    export_to = "v2/"
)]
/// Configures who approval requests are routed to for review. Examples
/// include sandbox escapes, blocked network access, MCP approval prompts, and
/// ARC escalations. Defaults to `user`. `auto_review` uses a carefully
/// prompted subagent to gather relevant context and apply a risk-based
/// decision framework before approving or denying the request.
pub enum ApprovalsReviewer {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "guardian_subagent", alias = "auto_review")]
    AutoReview,
}

impl JsonSchema for ApprovalsReviewer {
    fn schema_name() -> String {
        "ApprovalsReviewer".to_string()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_enum_schema_with_description(
            &["user", "auto_review", "guardian_subagent"],
            "Configures who approval requests are routed to for review. Examples include sandbox escapes, blocked network access, MCP approval prompts, and ARC escalations. Defaults to `user`. `auto_review` uses a carefully prompted subagent to gather relevant context and apply a risk-based decision framework before approving or denying the request. The legacy value `guardian_subagent` is accepted for compatibility.",
        )
    }
}

fn string_enum_schema_with_description(values: &[&str], description: &str) -> Schema {
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        metadata: Some(Box::new(Metadata {
            description: Some(description.to_string()),
            ..Default::default()
        })),
        ..Default::default()
    };
    schema.enum_values = Some(
        values
            .iter()
            .map(|value| JsonValue::String((*value).to_string()))
            .collect(),
    );
    Schema::Object(schema)
}

impl ApprovalsReviewer {
    pub fn to_core(self) -> CoreApprovalsReviewer {
        match self {
            ApprovalsReviewer::User => CoreApprovalsReviewer::User,
            ApprovalsReviewer::AutoReview => CoreApprovalsReviewer::AutoReview,
        }
    }
}

impl From<CoreApprovalsReviewer> for ApprovalsReviewer {
    fn from(value: CoreApprovalsReviewer) -> Self {
        match value {
            CoreApprovalsReviewer::User => ApprovalsReviewer::User,
            CoreApprovalsReviewer::AutoReview => ApprovalsReviewer::AutoReview,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl SandboxMode {
    pub fn to_core(self) -> CoreSandboxMode {
        match self {
            SandboxMode::ReadOnly => CoreSandboxMode::ReadOnly,
            SandboxMode::WorkspaceWrite => CoreSandboxMode::WorkspaceWrite,
            SandboxMode::DangerFullAccess => CoreSandboxMode::DangerFullAccess,
        }
    }
}

impl From<CoreSandboxMode> for SandboxMode {
    fn from(value: CoreSandboxMode) -> Self {
        match value {
            CoreSandboxMode::ReadOnly => SandboxMode::ReadOnly,
            CoreSandboxMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
            CoreSandboxMode::DangerFullAccess => SandboxMode::DangerFullAccess,
        }
    }
}

v2_enum_from_core!(
    pub enum ReviewDelivery from codex_protocol::protocol::ReviewDelivery {
        Inline, Detached
    }
);

v2_enum_from_core!(
    pub enum McpAuthStatus from codex_protocol::protocol::McpAuthStatus {
        Unsupported,
        NotLoggedIn,
        BearerToken,
        OAuth
    }
);

v2_enum_from_core!(
    pub enum ModelRerouteReason from CoreModelRerouteReason {
        HighRiskCyberActivity
    }
);

v2_enum_from_core!(
    pub enum ModelVerification from CoreModelVerification {
        TrustedAccessForCyber
    }
);

v2_enum_from_core!(
    pub enum HookEventName from CoreHookEventName {
        PreToolUse, PermissionRequest, PostToolUse, SessionStart, UserPromptSubmit, Stop
    }
);

v2_enum_from_core!(
    pub enum HookHandlerType from CoreHookHandlerType {
        Command, Prompt, Agent
    }
);

v2_enum_from_core!(
    pub enum HookExecutionMode from CoreHookExecutionMode {
        Sync, Async
    }
);

v2_enum_from_core!(
    pub enum HookScope from CoreHookScope {
        Thread, Turn
    }
);

v2_enum_from_core!(
    pub enum HookSource from CoreHookSource {
        System,
        User,
        Project,
        Mdm,
        SessionFlags,
        Plugin,
        CloudRequirements,
        LegacyManagedConfigFile,
        LegacyManagedConfigMdm,
        Unknown,
    }
);

v2_enum_from_core!(
    pub enum HookTrustStatus from CoreHookTrustStatus {
        Managed, Untrusted, Trusted, Modified
    }
);

fn default_hook_source() -> HookSource {
    HookSource::Unknown
}

v2_enum_from_core!(
    pub enum HookRunStatus from CoreHookRunStatus {
        Running, Completed, Failed, Blocked, Stopped
    }
);

v2_enum_from_core!(
    pub enum HookOutputEntryKind from CoreHookOutputEntryKind {
        Warning, Stop, Feedback, Context, Error
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadStartSource {
    Startup,
    Clear,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookOutputEntry {
    pub kind: HookOutputEntryKind,
    pub text: String,
}

impl From<CoreHookOutputEntry> for HookOutputEntry {
    fn from(value: CoreHookOutputEntry) -> Self {
        Self {
            kind: value.kind.into(),
            text: value.text,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookRunSummary {
    pub id: String,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub source_path: AbsolutePathBuf,
    #[serde(default = "default_hook_source")]
    pub source: HookSource,
    pub display_order: i64,
    pub status: HookRunStatus,
    pub status_message: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub entries: Vec<HookOutputEntry>,
}

impl From<CoreHookRunSummary> for HookRunSummary {
    fn from(value: CoreHookRunSummary) -> Self {
        Self {
            id: value.id,
            event_name: value.event_name.into(),
            handler_type: value.handler_type.into(),
            execution_mode: value.execution_mode.into(),
            scope: value.scope.into(),
            source_path: value.source_path,
            source: value.source.into(),
            display_order: value.display_order,
            status: value.status.into(),
            status_message: value.status_message,
            started_at: value.started_at,
            completed_at: value.completed_at,
            duration_ms: value.duration_ms,
            entries: value.entries.into_iter().map(Into::into).collect(),
        }
    }
}
