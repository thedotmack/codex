use super::*;

// === Server Notifications ===
// Thread/Turn lifecycle notifications and item progress events
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadStartedNotification {
    pub thread: Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadStatusChangedNotification {
    pub thread_id: String,
    pub status: ThreadStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadArchivedNotification {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadUnarchivedNotification {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadClosedNotification {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// Notification emitted when watched local skill files change.
///
/// Treat this as an invalidation signal and re-run `skills/list` with the
/// client's current parameters when refreshed skill metadata is needed.
pub struct SkillsChangedNotification {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadNameUpdatedNotification {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub thread_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalUpdatedNotification {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub goal: ThreadGoal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalClearedNotification {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnStartedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookStartedNotification {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub run: HookRunSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Usage {
    pub input_tokens: i32,
    pub cached_input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnCompletedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookCompletedNotification {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub run: HookRunSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// Notification that the turn-level unified diff has changed.
/// Contains the latest aggregated diff across all file changes in the turn.
pub struct TurnDiffUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub diff: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnPlanUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub explanation: Option<String>,
    pub plan: Vec<TurnPlanStep>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnPlanStep {
    pub step: String,
    pub status: TurnPlanStepStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum TurnPlanStepStatus {
    Pending,
    InProgress,
    Completed,
}

impl From<CorePlanItemArg> for TurnPlanStep {
    fn from(value: CorePlanItemArg) -> Self {
        Self {
            step: value.step,
            status: value.status.into(),
        }
    }
}

impl From<CorePlanStepStatus> for TurnPlanStepStatus {
    fn from(value: CorePlanStepStatus) -> Self {
        match value {
            CorePlanStepStatus::Pending => Self::Pending,
            CorePlanStepStatus::InProgress => Self::InProgress,
            CorePlanStepStatus::Completed => Self::Completed,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ItemStartedNotification {
    pub item: ThreadItem,
    pub thread_id: String,
    pub turn_id: String,
    /// Unix timestamp (in milliseconds) when this item lifecycle started.
    #[ts(type = "number")]
    pub started_at_ms: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// [UNSTABLE] Temporary notification payload for approval auto-review. This
/// shape is expected to change soon.
pub struct ItemGuardianApprovalReviewStartedNotification {
    pub thread_id: String,
    pub turn_id: String,
    /// Stable identifier for this review.
    pub review_id: String,
    /// Identifier for the reviewed item or tool call when one exists.
    ///
    /// In most cases, one review maps to one target item. The exceptions are
    /// - execve reviews, where a single command may contain multiple execve
    ///   calls to review (only possible when using the shell_zsh_fork feature)
    /// - network policy reviews, where there is no target item
    ///
    /// A network call is triggered by a CommandExecution item, so having a
    /// target_item_id set to the CommandExecution item would be misleading
    /// because the review is about the network call, not the command execution.
    /// Therefore, target_item_id is set to None for network policy reviews.
    pub target_item_id: Option<String>,
    pub review: GuardianApprovalReview,
    pub action: GuardianApprovalReviewAction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// [UNSTABLE] Temporary notification payload for approval auto-review. This
/// shape is expected to change soon.
pub struct ItemGuardianApprovalReviewCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    /// Stable identifier for this review.
    pub review_id: String,
    /// Identifier for the reviewed item or tool call when one exists.
    ///
    /// In most cases, one review maps to one target item. The exceptions are
    /// - execve reviews, where a single command may contain multiple execve
    ///   calls to review (only possible when using the shell_zsh_fork feature)
    /// - network policy reviews, where there is no target item
    ///
    /// A network call is triggered by a CommandExecution item, so having a
    /// target_item_id set to the CommandExecution item would be misleading
    /// because the review is about the network call, not the command execution.
    /// Therefore, target_item_id is set to None for network policy reviews.
    pub target_item_id: Option<String>,
    pub decision_source: AutoReviewDecisionSource,
    pub review: GuardianApprovalReview,
    pub action: GuardianApprovalReviewAction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ItemCompletedNotification {
    pub item: ThreadItem,
    pub thread_id: String,
    pub turn_id: String,
    /// Unix timestamp (in milliseconds) when this item lifecycle completed.
    #[ts(type = "number")]
    pub completed_at_ms: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct RawResponseItemCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item: ResponseItem,
}

// Item-specific progress notifications
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AgentMessageDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// EXPERIMENTAL - proposed plan streaming deltas for plan items. Clients should
/// not assume concatenated deltas match the completed plan item content.
pub struct PlanDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReasoningSummaryTextDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
    #[ts(type = "number")]
    pub summary_index: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReasoningSummaryPartAddedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    #[ts(type = "number")]
    pub summary_index: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReasoningTextDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
    #[ts(type = "number")]
    pub content_index: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TerminalInteractionNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub process_id: String,
    pub stdin: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct CommandExecutionOutputDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

/// Base64-encoded output chunk emitted for a streaming `command/exec` request.
///
/// These notifications are connection-scoped. If the originating connection
/// closes, the server terminates the process.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct CommandExecOutputDeltaNotification {
    /// Client-supplied, connection-scoped `processId` from the original
    /// `command/exec` request.
    pub process_id: String,
    /// Output stream for this chunk.
    pub stream: CommandExecOutputStream,
    /// Base64-encoded output bytes.
    pub delta_base64: String,
    /// `true` on the final streamed chunk for a stream when `outputBytesCap`
    /// truncated later output on that stream.
    pub cap_reached: bool,
}

/// Deprecated legacy notification for `apply_patch` textual output.
///
/// The server no longer emits this notification.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct FileChangeOutputDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct FileChangePatchUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub changes: Vec<FileUpdateChange>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ServerRequestResolvedNotification {
    pub thread_id: String,
    pub request_id: RequestId,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpToolCallProgressNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerOauthLoginCompletedNotification {
    pub name: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum McpServerStartupState {
    Starting,
    Ready,
    Failed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerStatusUpdatedNotification {
    pub name: String,
    pub status: McpServerStartupState,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WindowsWorldWritableWarningNotification {
    pub sample_paths: Vec<String>,
    pub extra_count: usize,
    pub failed_scan: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum WindowsSandboxSetupMode {
    Elevated,
    Unelevated,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum WindowsSandboxReadiness {
    Ready,
    NotConfigured,
    UpdateRequired,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WindowsSandboxSetupStartParams {
    pub mode: WindowsSandboxSetupMode,
    #[ts(optional = nullable)]
    pub cwd: Option<AbsolutePathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WindowsSandboxSetupStartResponse {
    pub started: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WindowsSandboxReadinessResponse {
    pub status: WindowsSandboxReadiness,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WindowsSandboxSetupCompletedNotification {
    pub mode: WindowsSandboxSetupMode,
    pub success: bool,
    pub error: Option<String>,
}

/// Deprecated: Use `ContextCompaction` item type instead.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ContextCompactedNotification {
    pub thread_id: String,
    pub turn_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AccountRateLimitsUpdatedNotification {
    pub rate_limits: RateLimitSnapshot,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct RateLimitSnapshot {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
    pub credits: Option<CreditsSnapshot>,
    pub plan_type: Option<PlanType>,
    pub rate_limit_reached_type: Option<RateLimitReachedType>,
}

impl From<CoreRateLimitSnapshot> for RateLimitSnapshot {
    fn from(value: CoreRateLimitSnapshot) -> Self {
        Self {
            limit_id: value.limit_id,
            limit_name: value.limit_name,
            primary: value.primary.map(RateLimitWindow::from),
            secondary: value.secondary.map(RateLimitWindow::from),
            credits: value.credits.map(CreditsSnapshot::from),
            plan_type: value.plan_type,
            rate_limit_reached_type: value
                .rate_limit_reached_type
                .map(RateLimitReachedType::from),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/", rename_all = "snake_case")]
pub enum RateLimitReachedType {
    RateLimitReached,
    WorkspaceOwnerCreditsDepleted,
    WorkspaceMemberCreditsDepleted,
    WorkspaceOwnerUsageLimitReached,
    WorkspaceMemberUsageLimitReached,
}

impl From<CoreRateLimitReachedType> for RateLimitReachedType {
    fn from(value: CoreRateLimitReachedType) -> Self {
        match value {
            CoreRateLimitReachedType::RateLimitReached => Self::RateLimitReached,
            CoreRateLimitReachedType::WorkspaceOwnerCreditsDepleted => {
                Self::WorkspaceOwnerCreditsDepleted
            }
            CoreRateLimitReachedType::WorkspaceMemberCreditsDepleted => {
                Self::WorkspaceMemberCreditsDepleted
            }
            CoreRateLimitReachedType::WorkspaceOwnerUsageLimitReached => {
                Self::WorkspaceOwnerUsageLimitReached
            }
            CoreRateLimitReachedType::WorkspaceMemberUsageLimitReached => {
                Self::WorkspaceMemberUsageLimitReached
            }
        }
    }
}

impl From<RateLimitReachedType> for CoreRateLimitReachedType {
    fn from(value: RateLimitReachedType) -> Self {
        match value {
            RateLimitReachedType::RateLimitReached => Self::RateLimitReached,
            RateLimitReachedType::WorkspaceOwnerCreditsDepleted => {
                Self::WorkspaceOwnerCreditsDepleted
            }
            RateLimitReachedType::WorkspaceMemberCreditsDepleted => {
                Self::WorkspaceMemberCreditsDepleted
            }
            RateLimitReachedType::WorkspaceOwnerUsageLimitReached => {
                Self::WorkspaceOwnerUsageLimitReached
            }
            RateLimitReachedType::WorkspaceMemberUsageLimitReached => {
                Self::WorkspaceMemberUsageLimitReached
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct RateLimitWindow {
    pub used_percent: i32,
    #[ts(type = "number | null")]
    pub window_duration_mins: Option<i64>,
    #[ts(type = "number | null")]
    pub resets_at: Option<i64>,
}

impl From<CoreRateLimitWindow> for RateLimitWindow {
    fn from(value: CoreRateLimitWindow) -> Self {
        Self {
            used_percent: value.used_percent.round() as i32,
            window_duration_mins: value.window_minutes,
            resets_at: value.resets_at,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct CreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

impl From<CoreCreditsSnapshot> for CreditsSnapshot {
    fn from(value: CoreCreditsSnapshot) -> Self {
        Self {
            has_credits: value.has_credits,
            unlimited: value.unlimited,
            balance: value.balance,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AccountLoginCompletedNotification {
    // Use plain String for identifiers to avoid TS/JSON Schema quirks around uuid-specific types.
    // Convert to/from UUIDs at the application layer as needed.
    pub login_id: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelReroutedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub from_model: String,
    pub to_model: String,
    pub reason: ModelRerouteReason,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelVerificationNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub verifications: Vec<ModelVerification>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DeprecationNoticeNotification {
    /// Concise summary of what is deprecated.
    pub summary: String,
    /// Optional extra guidance, such as migration steps or rationale.
    pub details: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct WarningNotification {
    /// Optional thread target when the warning applies to a specific thread.
    pub thread_id: Option<String>,
    /// Concise warning message for the user.
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct GuardianWarningNotification {
    /// Thread target for the guardian warning.
    pub thread_id: String,
    /// Concise guardian warning message for the user.
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TextPosition {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (in Unicode scalar values).
    pub column: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigWarningNotification {
    /// Concise summary of the warning.
    pub summary: String,
    /// Optional extra guidance or error details.
    pub details: Option<String>,
    /// Optional path to the config file that triggered the warning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub path: Option<String>,
    /// Optional range for the error location inside the config file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub range: Option<TextRange>,
}
