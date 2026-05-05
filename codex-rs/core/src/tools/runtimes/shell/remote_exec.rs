use crate::exec::ExecExpiration;
use crate::exec::StdoutStream;
use crate::exec::is_likely_sandbox_denied;
use crate::sandboxing::ExecRequest;
use crate::tools::runtimes::shell::ShellRequest;
use crate::tools::sandboxing::SandboxAttempt;
use crate::tools::sandboxing::ToolCtx;
use crate::tools::sandboxing::ToolError;
use codex_exec_server::Environment;
use codex_exec_server::ExecOutputStream;
use codex_exec_server::ExecParams as ExecServerParams;
use codex_exec_server::ExecProcess;
use codex_exec_server::ProcessId;
use codex_exec_server::ReadResponse;
use codex_protocol::error::CodexErr;
use codex_protocol::error::SandboxErr;
use codex_protocol::exec_output::ExecToolCallOutput;
use codex_protocol::exec_output::StreamOutput;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecCommandOutputDeltaEvent;
use codex_sandboxing::SandboxType;
use codex_utils_pty::DEFAULT_OUTPUT_BYTES_CAP;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// Executes the remote branch of `ShellRuntime`.
///
/// Local shell requests still go through `execute_env(...)`. Remote
/// environments have a different contract: start a process on the exec
/// backend, page output until the session closes, stream deltas to the client,
/// and then synthesize `ExecToolCallOutput`.
pub(super) struct RemoteShellExecutor {
    environment: Arc<Environment>,
    process_id: String,
    timeout_ms: Option<u64>,
    sandbox: SandboxType,
    stdout_stream: Option<StdoutStream>,
    network_denial_cancellation_token: Option<CancellationToken>,
}

/// Minimal remote-process contract needed by shell's remote exec path.
///
/// The shell runtime only needs retained-output reads plus termination. Keeping
/// this narrow makes the remote-only loop unit-testable without a live
/// exec-server backend.
pub(super) trait RemoteShellProcess: Send + Sync {
    fn read(
        &self,
        after_seq: Option<u64>,
        wait_ms: u64,
    ) -> impl std::future::Future<Output = Result<ReadResponse, String>> + Send;

    fn terminate(&self) -> impl std::future::Future<Output = ()> + Send;
}

struct ExecServerRemoteShellProcess {
    inner: Arc<dyn ExecProcess>,
}

impl RemoteShellExecutor {
    pub(super) fn new(
        req: &ShellRequest,
        exec_env: &ExecRequest,
        attempt: &SandboxAttempt<'_>,
        ctx: &ToolCtx,
    ) -> Self {
        Self {
            environment: Arc::clone(&req.environment),
            process_id: remote_shell_process_id(&ctx.call_id, attempt.sandbox),
            timeout_ms: req.timeout_ms,
            sandbox: exec_env.sandbox,
            stdout_stream: crate::tools::runtimes::shell::ShellRuntime::stdout_stream(ctx),
            network_denial_cancellation_token: attempt.network_denial_cancellation_token.clone(),
        }
    }

    pub(super) async fn run(self, exec_env: ExecRequest) -> Result<ExecToolCallOutput, ToolError> {
        let started = self
            .environment
            .get_exec_backend()
            .start(exec_server_params_for_request(&exec_env, &self.process_id))
            .await
            .map_err(|err| ToolError::Rejected(err.to_string()))?;
        let process = ExecServerRemoteShellProcess {
            inner: started.process,
        };
        self.run_with_process(&process).await
    }

    async fn run_with_process<P: RemoteShellProcess>(
        self,
        process: &P,
    ) -> Result<ExecToolCallOutput, ToolError> {
        let start = Instant::now();
        let expiration: ExecExpiration = self.timeout_ms.into();
        let timeout = expiration.timeout_ms().map(Duration::from_millis);
        let deadline = timeout.map(|timeout| start + timeout);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut aggregated_output = Vec::new();
        let mut after_seq = None;
        let mut exit_code = None;

        loop {
            if let Some(cancellation) = self.network_denial_cancellation_token.as_ref()
                && cancellation.is_cancelled()
            {
                process.terminate().await;
                return Err(ToolError::Rejected(
                    "Network access was denied by the Codex sandbox network proxy.".to_string(),
                ));
            }
            let wait_ms = deadline
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .map(|remaining| remaining.min(Duration::from_millis(100)).as_millis() as u64)
                .unwrap_or(100);
            if wait_ms == 0 {
                process.terminate().await;
                let output = shell_output(
                    stdout,
                    stderr,
                    aggregated_output,
                    124,
                    start.elapsed(),
                    /*timed_out*/ true,
                );
                return Err(ToolError::Codex(CodexErr::Sandbox(SandboxErr::Timeout {
                    output: Box::new(output),
                })));
            }

            let response = process
                .read(after_seq, wait_ms)
                .await
                .map_err(ToolError::Rejected)?;
            for chunk in response.chunks {
                let stream = chunk.stream;
                let bytes = chunk.chunk.into_inner();
                if let Some(stdout_stream) = self.stdout_stream.as_ref() {
                    emit_output_delta(stdout_stream, stream, bytes.clone()).await;
                }
                match stream {
                    ExecOutputStream::Stdout | ExecOutputStream::Pty => {
                        append_capped(&mut stdout, &bytes);
                        append_capped(&mut aggregated_output, &bytes);
                    }
                    ExecOutputStream::Stderr => {
                        append_capped(&mut stderr, &bytes);
                        append_capped(&mut aggregated_output, &bytes);
                    }
                }
            }
            if let Some(failure) = response.failure {
                return Err(ToolError::Rejected(failure));
            }
            if response.exited {
                exit_code = response.exit_code;
            }
            if response.closed {
                break;
            }
            after_seq = response.next_seq.checked_sub(1);
        }

        let output = shell_output(
            stdout,
            stderr,
            aggregated_output,
            exit_code.unwrap_or(-1),
            start.elapsed(),
            /*timed_out*/ false,
        );
        if is_likely_sandbox_denied(self.sandbox, &output) {
            return Err(ToolError::Codex(CodexErr::Sandbox(SandboxErr::Denied {
                output: Box::new(output),
                network_policy_decision: None,
            })));
        }
        Ok(output)
    }
}

impl RemoteShellProcess for ExecServerRemoteShellProcess {
    async fn read(&self, after_seq: Option<u64>, wait_ms: u64) -> Result<ReadResponse, String> {
        self.inner
            .read(after_seq, /*max_bytes*/ None, Some(wait_ms))
            .await
            .map_err(|err| err.to_string())
    }

    async fn terminate(&self) {
        let _ = self.inner.terminate().await;
    }
}

fn exec_server_env_for_request(
    request: &ExecRequest,
) -> (
    Option<codex_exec_server::ExecEnvPolicy>,
    HashMap<String, String>,
) {
    if let Some(exec_server_env_config) = &request.exec_server_env_config {
        let env = exec_server_env_config.env_overlay(&request.env);
        (Some(exec_server_env_config.policy.clone()), env)
    } else {
        (None, request.env.clone())
    }
}

fn exec_server_params_for_request(request: &ExecRequest, call_id: &str) -> ExecServerParams {
    let (env_policy, env) = exec_server_env_for_request(request);
    ExecServerParams {
        process_id: ProcessId::from(call_id.to_string()),
        argv: request.command.clone(),
        cwd: request.cwd.to_path_buf(),
        env_policy,
        env,
        tty: false,
        pipe_stdin: false,
        arg0: request.arg0.clone(),
    }
}

fn remote_shell_process_id(call_id: &str, sandbox: SandboxType) -> String {
    match sandbox {
        SandboxType::None => format!("{call_id}:unsandboxed"),
        SandboxType::MacosSeatbelt => format!("{call_id}:seatbelt"),
        SandboxType::LinuxSeccomp => format!("{call_id}:seccomp"),
        SandboxType::WindowsSandbox => format!("{call_id}:windows"),
    }
}

fn append_capped(dst: &mut Vec<u8>, src: &[u8]) {
    if dst.len() >= DEFAULT_OUTPUT_BYTES_CAP {
        return;
    }
    let remaining = DEFAULT_OUTPUT_BYTES_CAP.saturating_sub(dst.len());
    dst.extend_from_slice(&src[..src.len().min(remaining)]);
}

async fn emit_output_delta(
    stdout_stream: &StdoutStream,
    call_stream: ExecOutputStream,
    chunk: Vec<u8>,
) {
    let stream = match call_stream {
        ExecOutputStream::Stdout | ExecOutputStream::Pty => {
            codex_protocol::protocol::ExecOutputStream::Stdout
        }
        ExecOutputStream::Stderr => codex_protocol::protocol::ExecOutputStream::Stderr,
    };
    let msg = EventMsg::ExecCommandOutputDelta(ExecCommandOutputDeltaEvent {
        call_id: stdout_stream.call_id.clone(),
        stream,
        chunk,
    });
    let event = Event {
        id: stdout_stream.sub_id.clone(),
        msg,
    };
    let _ = stdout_stream.tx_event.send(event).await;
}

fn shell_output(
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    aggregated_output: Vec<u8>,
    exit_code: i32,
    duration: Duration,
    timed_out: bool,
) -> ExecToolCallOutput {
    let stdout = StreamOutput {
        text: stdout,
        truncated_after_lines: None,
    };
    let stderr = StreamOutput {
        text: stderr,
        truncated_after_lines: None,
    };
    ExecToolCallOutput {
        exit_code,
        stdout: stdout.from_utf8_lossy(),
        stderr: stderr.from_utf8_lossy(),
        aggregated_output: StreamOutput {
            text: codex_protocol::exec_output::bytes_to_string_smart(&aggregated_output),
            truncated_after_lines: None,
        },
        duration,
        timed_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_exec_server::Environment;
    use codex_exec_server::ExecEnvPolicy;
    use codex_exec_server::ExecOutputStream;
    use codex_exec_server::ProcessOutputChunk;
    use codex_protocol::config_types::ShellEnvironmentPolicyInherit;
    use pretty_assertions::assert_eq;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct FakeRemoteShellProcess {
        responses: Mutex<VecDeque<ReadResponse>>,
        terminated: Mutex<bool>,
    }

    impl FakeRemoteShellProcess {
        fn new(responses: Vec<ReadResponse>) -> Self {
            Self {
                responses: Mutex::new(VecDeque::from(responses)),
                terminated: Mutex::new(false),
            }
        }

        fn terminated(&self) -> bool {
            *self
                .terminated
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        }
    }

    impl RemoteShellProcess for FakeRemoteShellProcess {
        async fn read(
            &self,
            _after_seq: Option<u64>,
            _wait_ms: u64,
        ) -> Result<ReadResponse, String> {
            self.responses
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pop_front()
                .ok_or_else(|| "no more responses".to_string())
        }

        async fn terminate(&self) {
            *self
                .terminated
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        }
    }

    fn chunk(seq: u64, stream: ExecOutputStream, bytes: &[u8]) -> ProcessOutputChunk {
        ProcessOutputChunk {
            seq,
            stream,
            chunk: bytes.to_vec().into(),
        }
    }

    fn test_executor(timeout_ms: Option<u64>) -> RemoteShellExecutor {
        RemoteShellExecutor {
            environment: Arc::new(Environment::default_for_tests()),
            process_id: "call-1".to_string(),
            timeout_ms,
            sandbox: SandboxType::None,
            stdout_stream: None,
            network_denial_cancellation_token: None,
        }
    }

    fn read_response(
        chunks: Vec<ProcessOutputChunk>,
        next_seq: u64,
        exited: bool,
        exit_code: Option<i32>,
        closed: bool,
        failure: Option<String>,
    ) -> ReadResponse {
        ReadResponse {
            chunks,
            next_seq,
            exited,
            exit_code,
            closed,
            failure,
        }
    }

    #[tokio::test]
    async fn remote_shell_executor_collects_output_until_closed() {
        let executor = test_executor(/*timeout_ms*/ None);
        let process = FakeRemoteShellProcess::new(vec![
            read_response(
                vec![
                    chunk(1, ExecOutputStream::Stdout, b"hello "),
                    chunk(2, ExecOutputStream::Stderr, b"warn"),
                ],
                /*next_seq*/ 3,
                /*exited*/ false,
                /*exit_code*/ None,
                /*closed*/ false,
                /*failure*/ None,
            ),
            read_response(
                vec![chunk(3, ExecOutputStream::Stdout, b"world")],
                /*next_seq*/ 4,
                /*exited*/ true,
                /*exit_code*/ Some(0),
                /*closed*/ true,
                /*failure*/ None,
            ),
        ]);

        let output = executor
            .run_with_process(&process)
            .await
            .expect("remote shell output should succeed");

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.text, "hello world");
        assert_eq!(output.stderr.text, "warn");
        assert_eq!(output.aggregated_output.text, "hello warnworld");
        assert!(!output.timed_out);
    }

    #[tokio::test]
    async fn remote_shell_executor_times_out_and_terminates_process() {
        let executor = test_executor(/*timeout_ms*/ Some(0));
        let process = FakeRemoteShellProcess::new(Vec::new());

        let err = executor
            .run_with_process(&process)
            .await
            .expect_err("timeout should fail");

        assert!(process.terminated());
        match err {
            ToolError::Codex(CodexErr::Sandbox(SandboxErr::Timeout { .. })) => {}
            other => panic!("expected timeout error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn remote_shell_executor_rejects_exec_backend_failure() {
        let executor = test_executor(/*timeout_ms*/ None);
        let process = FakeRemoteShellProcess::new(vec![read_response(
            Vec::new(),
            /*next_seq*/ 1,
            /*exited*/ false,
            /*exit_code*/ None,
            /*closed*/ false,
            /*failure*/ Some("backend disconnected".to_string()),
        )]);

        let err = executor
            .run_with_process(&process)
            .await
            .expect_err("remote exec failure should fail");

        match err {
            ToolError::Rejected(message) => assert_eq!(message, "backend disconnected"),
            other => panic!("expected rejected error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn remote_shell_executor_terminates_on_network_denial_cancellation() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let executor = RemoteShellExecutor {
            environment: Arc::new(Environment::default_for_tests()),
            process_id: "call-1".to_string(),
            timeout_ms: None,
            sandbox: SandboxType::None,
            stdout_stream: None,
            network_denial_cancellation_token: Some(cancellation),
        };
        let process = FakeRemoteShellProcess::new(Vec::new());

        let err = executor
            .run_with_process(&process)
            .await
            .expect_err("network denial should fail");

        assert!(process.terminated());
        match err {
            ToolError::Rejected(message) => assert_eq!(
                message,
                "Network access was denied by the Codex sandbox network proxy."
            ),
            other => panic!("expected rejected error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn remote_shell_executor_maps_likely_sandbox_denials() {
        let executor = RemoteShellExecutor {
            environment: Arc::new(Environment::default_for_tests()),
            process_id: "call-1".to_string(),
            timeout_ms: None,
            sandbox: SandboxType::LinuxSeccomp,
            stdout_stream: None,
            network_denial_cancellation_token: None,
        };
        let process = FakeRemoteShellProcess::new(vec![read_response(
            vec![chunk(
                1,
                ExecOutputStream::Stderr,
                b"operation not permitted: sandbox denied",
            )],
            /*next_seq*/ 2,
            /*exited*/ true,
            /*exit_code*/ Some(1),
            /*closed*/ true,
            /*failure*/ None,
        )]);

        let err = executor
            .run_with_process(&process)
            .await
            .expect_err("sandbox denial should fail");

        match err {
            ToolError::Codex(CodexErr::Sandbox(SandboxErr::Denied { .. })) => {}
            other => panic!("expected sandbox denied error, got {other:?}"),
        }
    }

    #[test]
    fn exec_server_params_use_env_policy_overlay_contract() {
        let cwd: codex_utils_absolute_path::AbsolutePathBuf = std::env::current_dir()
            .expect("current dir")
            .try_into()
            .expect("absolute path");
        let request = ExecRequest {
            command: vec!["bash".to_string(), "-lc".to_string(), "true".to_string()],
            cwd: cwd.clone(),
            env: HashMap::from([
                ("HOME".to_string(), "/client-home".to_string()),
                ("PATH".to_string(), "/sandbox-path".to_string()),
                ("CODEX_THREAD_ID".to_string(), "thread-1".to_string()),
            ]),
            exec_server_env_config: Some(crate::sandboxing::ExecServerEnvConfig {
                policy: ExecEnvPolicy {
                    inherit: ShellEnvironmentPolicyInherit::Core,
                    ignore_default_excludes: false,
                    exclude: Vec::new(),
                    r#set: HashMap::new(),
                    include_only: Vec::new(),
                },
                local_policy_env: HashMap::from([
                    ("HOME".to_string(), "/client-home".to_string()),
                    ("PATH".to_string(), "/client-path".to_string()),
                ]),
            }),
            network: None,
            expiration: crate::exec::ExecExpiration::DefaultTimeout,
            capture_policy: crate::exec::ExecCapturePolicy::ShellTool,
            sandbox: SandboxType::None,
            windows_sandbox_policy_cwd: cwd,
            windows_sandbox_level: codex_protocol::config_types::WindowsSandboxLevel::Disabled,
            windows_sandbox_private_desktop: false,
            permission_profile: codex_protocol::models::PermissionProfile::Disabled,
            file_system_sandbox_policy:
                codex_protocol::permissions::FileSystemSandboxPolicy::unrestricted(),
            network_sandbox_policy: codex_protocol::permissions::NetworkSandboxPolicy::Restricted,
            windows_sandbox_filesystem_overrides: None,
            arg0: None,
        };

        let params = exec_server_params_for_request(&request, "call-123");

        assert_eq!(params.process_id.as_str(), "call-123");
        assert!(params.env_policy.is_some());
        assert_eq!(
            params.env,
            HashMap::from([
                ("PATH".to_string(), "/sandbox-path".to_string()),
                ("CODEX_THREAD_ID".to_string(), "thread-1".to_string()),
            ])
        );
    }

    #[test]
    fn remote_shell_process_id_distinguishes_retry_attempts() {
        assert_eq!(
            remote_shell_process_id("call-123", SandboxType::MacosSeatbelt),
            "call-123:seatbelt"
        );
        assert_eq!(
            remote_shell_process_id("call-123", SandboxType::None),
            "call-123:unsandboxed"
        );
    }
}
