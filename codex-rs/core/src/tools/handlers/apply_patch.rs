use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::apply_patch;
use crate::apply_patch::InternalApplyPatchInvocation;
use crate::apply_patch::convert_apply_patch_to_protocol;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::ApplyPatchToolOutput;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::events::ToolEmitter;
use crate::tools::events::ToolEventCtx;
use crate::tools::handlers::apply_granted_turn_permissions;
use crate::tools::handlers::parse_arguments;
use crate::tools::hook_names::HookToolName;
use crate::tools::orchestrator::ToolOrchestrator;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
use crate::tools::registry::ToolArgumentDiffConsumer;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::runtimes::apply_patch::ApplyPatchRequest;
use crate::tools::runtimes::apply_patch::ApplyPatchRuntime;
use crate::tools::sandboxing::ToolCtx;
use codex_apply_patch::ApplyPatchAction;
use codex_apply_patch::ApplyPatchFileChange;
use codex_apply_patch::Hunk;
use codex_apply_patch::StreamingPatchParser;
use codex_exec_server::ExecutorFileSystem;
use codex_features::Feature;
use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::models::FileSystemPermissions;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::FileChange;
use codex_protocol::protocol::PatchApplyUpdatedEvent;
use codex_sandboxing::policy_transforms::effective_file_system_sandbox_policy;
use codex_sandboxing::policy_transforms::merge_permission_profiles;
use codex_sandboxing::policy_transforms::normalize_additional_permissions;
use codex_tools::ApplyPatchToolArgs;
use codex_tools::ToolName;
use codex_utils_absolute_path::AbsolutePathBuf;

const APPLY_PATCH_ARGUMENT_DIFF_BUFFER_INTERVAL: Duration = Duration::from_millis(500);
const APPLY_PATCH_BEGIN_MARKER: &str = "*** Begin Patch\n";
const APPLY_PATCH_ENVIRONMENT_ID_MARKER: &str = "*** Environment ID: ";

pub struct ApplyPatchHandler;

#[derive(Default)]
struct ApplyPatchArgumentDiffConsumer {
    parser: StreamingPatchParser,
    last_sent_at: Option<Instant>,
    pending: Option<PatchApplyUpdatedEvent>,
    metadata_stripper: ApplyPatchMetadataStripper,
}

#[derive(Debug, PartialEq, Eq)]
struct ResolvedApplyPatchInput {
    patch: String,
    environment_id: Option<String>,
}

struct ApplyPatchMetadataStripper {
    state: ApplyPatchMetadataStripperState,
}

enum ApplyPatchMetadataStripperState {
    // Freeform apply_patch may start with an optional `*** Environment ID: ...`
    // line. Streaming diffs can split that prefix across chunks, so we buffer
    // only until we can decide whether to strip metadata or pass the bytes
    // through to the patch parser unchanged.
    //
    // `Prefix` owns that short-lived buffer. Once we either strip the metadata
    // line or determine there is no metadata to strip, we switch to
    // `Passthrough` and stop doing any special-case parsing for later chunks.
    Prefix(String),
    Passthrough,
}

impl Default for ApplyPatchMetadataStripper {
    fn default() -> Self {
        Self {
            state: ApplyPatchMetadataStripperState::Prefix(String::new()),
        }
    }
}

impl ToolArgumentDiffConsumer for ApplyPatchArgumentDiffConsumer {
    fn consume_diff(
        &mut self,
        turn: &TurnContext,
        call_id: String,
        diff: &str,
    ) -> Option<EventMsg> {
        if !turn.features.enabled(Feature::ApplyPatchStreamingEvents) {
            return None;
        }

        self.push_delta(call_id, diff)
            .map(EventMsg::PatchApplyUpdated)
    }

    fn finish(&mut self) -> Result<Option<EventMsg>, FunctionCallError> {
        self.finish_update_on_complete()
            .map(|event| event.map(EventMsg::PatchApplyUpdated))
    }
}

impl ApplyPatchArgumentDiffConsumer {
    fn push_delta(&mut self, call_id: String, delta: &str) -> Option<PatchApplyUpdatedEvent> {
        let delta = self.metadata_stripper.push(delta)?;
        let hunks = self.parser.push_delta(&delta).ok()?;
        if hunks.is_empty() {
            return None;
        }
        let changes = convert_apply_patch_hunks_to_protocol(&hunks);
        let event = PatchApplyUpdatedEvent { call_id, changes };
        let now = Instant::now();
        match self.last_sent_at {
            Some(last_sent_at)
                if now.duration_since(last_sent_at) < APPLY_PATCH_ARGUMENT_DIFF_BUFFER_INTERVAL =>
            {
                self.pending = Some(event);
                None
            }
            Some(_) | None => {
                self.pending = None;
                self.last_sent_at = Some(now);
                Some(event)
            }
        }
    }

    fn finish_update_on_complete(
        &mut self,
    ) -> Result<Option<PatchApplyUpdatedEvent>, FunctionCallError> {
        if let Some(delta) = self.metadata_stripper.finish()? {
            self.parser.push_delta(&delta).map_err(|err| {
                FunctionCallError::RespondToModel(format!("failed to parse apply_patch: {err}"))
            })?;
        }
        self.parser.finish().map_err(|err| {
            FunctionCallError::RespondToModel(format!("failed to parse apply_patch: {err}"))
        })?;

        let event = self.pending.take();
        if event.is_some() {
            self.last_sent_at = Some(Instant::now());
        }
        Ok(event)
    }
}

impl ApplyPatchMetadataStripper {
    fn push(&mut self, delta: &str) -> Option<String> {
        let ApplyPatchMetadataStripperState::Prefix(buffer) = &mut self.state else {
            return Some(delta.to_string());
        };

        buffer.push_str(delta);
        if !APPLY_PATCH_BEGIN_MARKER.starts_with(buffer.as_str())
            && !buffer.starts_with(APPLY_PATCH_BEGIN_MARKER)
        {
            let buffered = std::mem::take(buffer);
            self.state = ApplyPatchMetadataStripperState::Passthrough;
            return Some(buffered);
        }

        let Some(rest) = buffer.as_str().strip_prefix(APPLY_PATCH_BEGIN_MARKER) else {
            return None;
        };
        if rest.is_empty() || APPLY_PATCH_ENVIRONMENT_ID_MARKER.starts_with(rest) {
            return None;
        }

        if let Some(after_marker) = rest.strip_prefix(APPLY_PATCH_ENVIRONMENT_ID_MARKER) {
            let Some((_environment_id, patch_rest)) = after_marker.split_once('\n') else {
                return None;
            };
            let patch = format!("{APPLY_PATCH_BEGIN_MARKER}{patch_rest}");
            self.state = ApplyPatchMetadataStripperState::Passthrough;
            return Some(patch);
        }
        let buffered = std::mem::take(buffer);
        self.state = ApplyPatchMetadataStripperState::Passthrough;
        Some(buffered)
    }

    fn finish(&mut self) -> Result<Option<String>, FunctionCallError> {
        match std::mem::replace(
            &mut self.state,
            ApplyPatchMetadataStripperState::Passthrough,
        ) {
            ApplyPatchMetadataStripperState::Prefix(buffer) => Ok(Some(buffer)),
            ApplyPatchMetadataStripperState::Passthrough => Ok(None),
        }
    }
}

fn parse_freeform_apply_patch_input(
    input: String,
) -> Result<ResolvedApplyPatchInput, FunctionCallError> {
    let Some(rest) = input.strip_prefix(APPLY_PATCH_BEGIN_MARKER) else {
        return Ok(ResolvedApplyPatchInput {
            patch: input,
            environment_id: None,
        });
    };
    let Some(after_marker) = rest.strip_prefix(APPLY_PATCH_ENVIRONMENT_ID_MARKER) else {
        return Ok(ResolvedApplyPatchInput {
            patch: input,
            environment_id: None,
        });
    };
    let Some((environment_id, patch_rest)) = after_marker.split_once('\n') else {
        return Err(FunctionCallError::RespondToModel(
            "apply_patch environment metadata must end with a newline".to_string(),
        ));
    };
    if environment_id.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "environment_id cannot be empty".to_string(),
        ));
    }
    Ok(ResolvedApplyPatchInput {
        patch: format!("{APPLY_PATCH_BEGIN_MARKER}{patch_rest}"),
        environment_id: Some(environment_id.to_string()),
    })
}

fn merge_apply_patch_environment_ids(
    argument_environment_id: Option<String>,
    metadata_environment_id: Option<String>,
) -> Result<Option<String>, FunctionCallError> {
    match (argument_environment_id, metadata_environment_id) {
        (Some(argument_environment_id), Some(metadata_environment_id))
            if argument_environment_id != metadata_environment_id =>
        {
            Err(FunctionCallError::RespondToModel(
                "apply_patch environment_id argument conflicts with patch environment metadata"
                    .to_string(),
            ))
        }
        (Some(argument_environment_id), _) => Ok(Some(argument_environment_id)),
        (None, metadata_environment_id) => Ok(metadata_environment_id),
    }
}

fn parse_function_apply_patch_input(
    arguments: &str,
) -> Result<ResolvedApplyPatchInput, FunctionCallError> {
    let args: ApplyPatchToolArgs = parse_arguments(arguments)?;
    let parsed_input = parse_freeform_apply_patch_input(args.input)?;
    let environment_id =
        merge_apply_patch_environment_ids(args.environment_id, parsed_input.environment_id)?;
    Ok(ResolvedApplyPatchInput {
        patch: parsed_input.patch,
        environment_id,
    })
}

/// Normalizes both apply_patch payload shapes into one resolved request.
///
/// Backward compatibility:
/// - legacy apply_patch calls omit environment selection entirely
/// - those continue to target the primary environment
/// - routing only changes when an environment id is explicitly provided, either
///   as a function argument or as freeform patch metadata
fn resolve_apply_patch_input(
    payload: ToolPayload,
) -> Result<ResolvedApplyPatchInput, FunctionCallError> {
    match payload {
        ToolPayload::Function { arguments } => parse_function_apply_patch_input(&arguments),
        ToolPayload::Custom { input } => parse_freeform_apply_patch_input(input),
        _ => Err(FunctionCallError::RespondToModel(
            "apply_patch handler received unsupported payload".to_string(),
        )),
    }
}

fn convert_apply_patch_hunks_to_protocol(hunks: &[Hunk]) -> HashMap<PathBuf, FileChange> {
    hunks
        .iter()
        .map(|hunk| {
            let path = hunk_source_path(hunk).to_path_buf();
            let change = match hunk {
                Hunk::AddFile { contents, .. } => FileChange::Add {
                    content: contents.clone(),
                },
                Hunk::DeleteFile { .. } => FileChange::Delete {
                    content: String::new(),
                },
                Hunk::UpdateFile {
                    chunks, move_path, ..
                } => FileChange::Update {
                    unified_diff: format_update_chunks_for_progress(chunks),
                    move_path: move_path.clone(),
                },
            };
            (path, change)
        })
        .collect()
}

fn hunk_source_path(hunk: &Hunk) -> &Path {
    match hunk {
        Hunk::AddFile { path, .. } | Hunk::DeleteFile { path } | Hunk::UpdateFile { path, .. } => {
            path
        }
    }
}

fn format_update_chunks_for_progress(chunks: &[codex_apply_patch::UpdateFileChunk]) -> String {
    let mut unified_diff = String::new();
    for chunk in chunks {
        match &chunk.change_context {
            Some(context) => {
                unified_diff.push_str("@@ ");
                unified_diff.push_str(context);
                unified_diff.push('\n');
            }
            None => {
                unified_diff.push_str("@@");
                unified_diff.push('\n');
            }
        }
        for line in &chunk.old_lines {
            unified_diff.push('-');
            unified_diff.push_str(line);
            unified_diff.push('\n');
        }
        for line in &chunk.new_lines {
            unified_diff.push('+');
            unified_diff.push_str(line);
            unified_diff.push('\n');
        }
        if chunk.is_end_of_file {
            unified_diff.push_str("*** End of File");
            unified_diff.push('\n');
        }
    }
    unified_diff
}

fn file_paths_for_action(action: &ApplyPatchAction) -> Vec<AbsolutePathBuf> {
    let mut keys = Vec::new();
    let cwd = &action.cwd;

    for (path, change) in action.changes() {
        if let Some(key) = to_abs_path(cwd, path) {
            keys.push(key);
        }

        if let ApplyPatchFileChange::Update { move_path, .. } = change
            && let Some(dest) = move_path
            && let Some(key) = to_abs_path(cwd, dest)
        {
            keys.push(key);
        }
    }

    keys
}

fn to_abs_path(cwd: &AbsolutePathBuf, path: &Path) -> Option<AbsolutePathBuf> {
    Some(AbsolutePathBuf::resolve_path_against_base(path, cwd))
}

fn write_permissions_for_paths(
    file_paths: &[AbsolutePathBuf],
    file_system_sandbox_policy: &codex_protocol::permissions::FileSystemSandboxPolicy,
    cwd: &AbsolutePathBuf,
) -> Option<AdditionalPermissionProfile> {
    let write_paths = file_paths
        .iter()
        .map(|path| {
            path.parent()
                .unwrap_or_else(|| path.clone())
                .into_path_buf()
        })
        .filter(|path| {
            !file_system_sandbox_policy.can_write_path_with_cwd(path.as_path(), cwd.as_path())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(AbsolutePathBuf::from_absolute_path)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    let permissions = (!write_paths.is_empty()).then_some(AdditionalPermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            Some(vec![]),
            Some(write_paths),
        )),
        ..Default::default()
    })?;

    normalize_additional_permissions(permissions).ok()
}

/// Extracts the raw patch text used as the command-shaped hook input for apply_patch.
///
/// The apply_patch tool can arrive as the older JSON/function shape or as a
/// freeform custom tool call. Both represent the same file edit operation, so
/// hooks see the raw patch body in `tool_input.command` either way.
fn apply_patch_payload_command(payload: &ToolPayload) -> Option<String> {
    match payload {
        ToolPayload::Function { arguments } => parse_arguments::<ApplyPatchToolArgs>(arguments)
            .ok()
            .map(|args| args.input),
        ToolPayload::Custom { input } => Some(input.clone()),
        _ => None,
    }
}

async fn effective_patch_permissions(
    session: &Session,
    turn: &TurnContext,
    cwd: &AbsolutePathBuf,
    action: &ApplyPatchAction,
) -> (
    Vec<AbsolutePathBuf>,
    crate::tools::handlers::EffectiveAdditionalPermissions,
    codex_protocol::permissions::FileSystemSandboxPolicy,
) {
    let file_paths = file_paths_for_action(action);
    let granted_permissions = merge_permission_profiles(
        session.granted_session_permissions().await.as_ref(),
        session.granted_turn_permissions().await.as_ref(),
    );
    let base_file_system_sandbox_policy = turn.file_system_sandbox_policy();
    let file_system_sandbox_policy = effective_file_system_sandbox_policy(
        &base_file_system_sandbox_policy,
        granted_permissions.as_ref(),
    );
    let effective_additional_permissions = apply_granted_turn_permissions(
        session,
        cwd.as_path(),
        crate::sandboxing::SandboxPermissions::UseDefault,
        write_permissions_for_paths(&file_paths, &file_system_sandbox_policy, cwd),
    )
    .await;

    (
        file_paths,
        effective_additional_permissions,
        file_system_sandbox_policy,
    )
}

impl ToolHandler for ApplyPatchHandler {
    type Output = ApplyPatchToolOutput;

    fn tool_name(&self) -> ToolName {
        ToolName::plain("apply_patch")
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(
            payload,
            ToolPayload::Function { .. } | ToolPayload::Custom { .. }
        )
    }

    async fn is_mutating(&self, _invocation: &ToolInvocation) -> bool {
        true
    }

    fn create_diff_consumer(&self) -> Option<Box<dyn ToolArgumentDiffConsumer>> {
        Some(Box::<ApplyPatchArgumentDiffConsumer>::default())
    }

    fn pre_tool_use_payload(&self, invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
        apply_patch_payload_command(&invocation.payload).map(|command| PreToolUsePayload {
            tool_name: HookToolName::apply_patch(),
            tool_input: serde_json::json!({ "command": command }),
        })
    }

    fn post_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
        result: &Self::Output,
    ) -> Option<PostToolUsePayload> {
        let tool_response =
            result.post_tool_use_response(&invocation.call_id, &invocation.payload)?;
        Some(PostToolUsePayload {
            tool_name: HookToolName::apply_patch(),
            tool_use_id: invocation.call_id.clone(),
            tool_input: serde_json::json!({
                "command": apply_patch_payload_command(&invocation.payload)?,
            }),
            tool_response,
        })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            tracker,
            call_id,
            tool_name,
            payload,
            ..
        } = invocation;

        let ResolvedApplyPatchInput {
            patch: patch_input,
            environment_id,
        } = resolve_apply_patch_input(payload)?;
        // Re-parse and verify the patch so we can compute changes and approval.
        // Avoid building temporary ExecParams/command vectors; derive directly from inputs.
        let Some(target_environment) = turn
            .environments
            .get_or_primary(environment_id.as_deref())
            .cloned()
        else {
            return Err(FunctionCallError::RespondToModel(
                "apply_patch is unavailable in this session".to_string(),
            ));
        };
        let cwd = target_environment.cwd.clone();
        let command = vec!["apply_patch".to_string(), patch_input.clone()];
        let environment = Arc::clone(&target_environment.environment);
        let fs = environment.get_filesystem();
        let sandbox = environment.is_remote().then(|| {
            turn.file_system_sandbox_context_for_cwd(&cwd, /*additional_permissions*/ None)
        });
        match codex_apply_patch::maybe_parse_apply_patch_verified(
            &command,
            &cwd,
            fs.as_ref(),
            sandbox.as_ref(),
        )
        .await
        {
            codex_apply_patch::MaybeApplyPatchVerified::Body(changes) => {
                let (file_paths, effective_additional_permissions, file_system_sandbox_policy) =
                    effective_patch_permissions(
                        session.as_ref(),
                        turn.as_ref(),
                        &changes.cwd,
                        &changes,
                    )
                    .await;
                match apply_patch::apply_patch(turn.as_ref(), &file_system_sandbox_policy, changes)
                    .await
                {
                    InternalApplyPatchInvocation::Output(item) => {
                        let content = item?;
                        Ok(ApplyPatchToolOutput::from_text(content))
                    }
                    InternalApplyPatchInvocation::DelegateToRuntime(apply) => {
                        let changes = convert_apply_patch_to_protocol(&apply.action);
                        let emitter =
                            ToolEmitter::apply_patch(changes.clone(), apply.auto_approved);
                        let event_ctx = ToolEventCtx::new(
                            session.as_ref(),
                            turn.as_ref(),
                            &call_id,
                            Some(&tracker),
                        );
                        emitter.begin(event_ctx).await;

                        let req = ApplyPatchRequest {
                            action: apply.action,
                            environment: Arc::clone(&environment),
                            file_paths,
                            changes,
                            exec_approval_requirement: apply.exec_approval_requirement,
                            additional_permissions: effective_additional_permissions
                                .additional_permissions,
                            permissions_preapproved: effective_additional_permissions
                                .permissions_preapproved,
                        };

                        let mut orchestrator = ToolOrchestrator::new();
                        let mut runtime = ApplyPatchRuntime::new();
                        let tool_ctx = ToolCtx {
                            session: session.clone(),
                            turn: turn.clone(),
                            call_id: call_id.clone(),
                            tool_name: tool_name.display(),
                        };
                        let out = orchestrator
                            .run(
                                &mut runtime,
                                &req,
                                &tool_ctx,
                                turn.as_ref(),
                                turn.approval_policy.value(),
                            )
                            .await
                            .map(|result| result.output);
                        let event_ctx = ToolEventCtx::new(
                            session.as_ref(),
                            turn.as_ref(),
                            &call_id,
                            Some(&tracker),
                        );
                        let content = emitter.finish(event_ctx, out).await?;
                        Ok(ApplyPatchToolOutput::from_text(content))
                    }
                }
            }
            codex_apply_patch::MaybeApplyPatchVerified::CorrectnessError(parse_error) => {
                Err(FunctionCallError::RespondToModel(format!(
                    "apply_patch verification failed: {parse_error}"
                )))
            }
            codex_apply_patch::MaybeApplyPatchVerified::ShellParseError(error) => {
                tracing::trace!("Failed to parse apply_patch input, {error:?}");
                Err(FunctionCallError::RespondToModel(
                    "apply_patch handler received invalid patch input".to_string(),
                ))
            }
            codex_apply_patch::MaybeApplyPatchVerified::NotApplyPatch => {
                Err(FunctionCallError::RespondToModel(
                    "apply_patch handler received non-apply_patch input".to_string(),
                ))
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn intercept_apply_patch(
    command: &[String],
    cwd: &AbsolutePathBuf,
    environment: Arc<codex_exec_server::Environment>,
    fs: Arc<dyn ExecutorFileSystem>,
    sandbox: Option<codex_exec_server::FileSystemSandboxContext>,
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    tracker: Option<&SharedTurnDiffTracker>,
    call_id: &str,
    tool_name: &str,
) -> Result<Option<FunctionToolOutput>, FunctionCallError> {
    match codex_apply_patch::maybe_parse_apply_patch_verified(
        command,
        cwd,
        fs.as_ref(),
        sandbox.as_ref(),
    )
    .await
    {
        codex_apply_patch::MaybeApplyPatchVerified::Body(changes) => {
            session
                .record_model_warning(
                    format!(
                        "apply_patch was requested via {tool_name}. Use the apply_patch tool instead of exec_command."
                    ),
                    turn.as_ref(),
                )
                .await;
            let (approval_keys, effective_additional_permissions, file_system_sandbox_policy) =
                effective_patch_permissions(
                    session.as_ref(),
                    turn.as_ref(),
                    &changes.cwd,
                    &changes,
                )
                .await;
            match apply_patch::apply_patch(turn.as_ref(), &file_system_sandbox_policy, changes)
                .await
            {
                InternalApplyPatchInvocation::Output(item) => {
                    let content = item?;
                    Ok(Some(FunctionToolOutput::from_text(content, Some(true))))
                }
                InternalApplyPatchInvocation::DelegateToRuntime(apply) => {
                    let changes = convert_apply_patch_to_protocol(&apply.action);
                    let emitter = ToolEmitter::apply_patch(changes.clone(), apply.auto_approved);
                    let event_ctx = ToolEventCtx::new(
                        session.as_ref(),
                        turn.as_ref(),
                        call_id,
                        tracker.as_ref().copied(),
                    );
                    emitter.begin(event_ctx).await;

                    let req = ApplyPatchRequest {
                        action: apply.action,
                        environment,
                        file_paths: approval_keys,
                        changes,
                        exec_approval_requirement: apply.exec_approval_requirement,
                        additional_permissions: effective_additional_permissions
                            .additional_permissions,
                        permissions_preapproved: effective_additional_permissions
                            .permissions_preapproved,
                    };

                    let mut orchestrator = ToolOrchestrator::new();
                    let mut runtime = ApplyPatchRuntime::new();
                    let tool_ctx = ToolCtx {
                        session: session.clone(),
                        turn: turn.clone(),
                        call_id: call_id.to_string(),
                        tool_name: tool_name.to_string(),
                    };
                    let out = orchestrator
                        .run(
                            &mut runtime,
                            &req,
                            &tool_ctx,
                            turn.as_ref(),
                            turn.approval_policy.value(),
                        )
                        .await
                        .map(|result| result.output);
                    let event_ctx = ToolEventCtx::new(
                        session.as_ref(),
                        turn.as_ref(),
                        call_id,
                        tracker.as_ref().copied(),
                    );
                    let content = emitter.finish(event_ctx, out).await?;
                    Ok(Some(FunctionToolOutput::from_text(content, Some(true))))
                }
            }
        }
        codex_apply_patch::MaybeApplyPatchVerified::CorrectnessError(parse_error) => {
            Err(FunctionCallError::RespondToModel(format!(
                "apply_patch verification failed: {parse_error}"
            )))
        }
        codex_apply_patch::MaybeApplyPatchVerified::ShellParseError(error) => {
            tracing::trace!("Failed to parse apply_patch input, {error:?}");
            Ok(None)
        }
        codex_apply_patch::MaybeApplyPatchVerified::NotApplyPatch => Ok(None),
    }
}

#[cfg(test)]
#[path = "apply_patch_tests.rs"]
mod tests;
