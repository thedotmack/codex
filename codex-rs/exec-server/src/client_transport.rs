use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tracing::debug;
use tracing::warn;

use crate::ExecServerClient;
use crate::ExecServerError;
use crate::client_api::RemoteExecServerConnectArgs;
use crate::client_api::StdioExecServerCommand;
use crate::client_api::StdioExecServerConnectArgs;
use crate::connection::JsonRpcConnection;

const ENVIRONMENT_CLIENT_NAME: &str = "codex-environment";
const ENVIRONMENT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const ENVIRONMENT_INITIALIZE_TIMEOUT: Duration = Duration::from_secs(5);

impl ExecServerClient {
    pub(crate) async fn connect_for_environment(
        transport: crate::client_api::ExecServerTransport,
    ) -> Result<Self, ExecServerError> {
        match transport {
            crate::client_api::ExecServerTransport::WebSocketUrl(websocket_url) => {
                Self::connect_websocket(RemoteExecServerConnectArgs {
                    websocket_url,
                    client_name: ENVIRONMENT_CLIENT_NAME.to_string(),
                    connect_timeout: ENVIRONMENT_CONNECT_TIMEOUT,
                    initialize_timeout: ENVIRONMENT_INITIALIZE_TIMEOUT,
                    resume_session_id: None,
                })
                .await
            }
            crate::client_api::ExecServerTransport::StdioCommand(command) => {
                Self::connect_stdio_command(StdioExecServerConnectArgs {
                    command,
                    client_name: ENVIRONMENT_CLIENT_NAME.to_string(),
                    initialize_timeout: ENVIRONMENT_INITIALIZE_TIMEOUT,
                    resume_session_id: None,
                })
                .await
            }
        }
    }

    pub async fn connect_websocket(
        args: RemoteExecServerConnectArgs,
    ) -> Result<Self, ExecServerError> {
        let websocket_url = args.websocket_url.clone();
        let connect_timeout = args.connect_timeout;
        let (stream, _) = timeout(connect_timeout, connect_async(websocket_url.as_str()))
            .await
            .map_err(|_| ExecServerError::WebSocketConnectTimeout {
                url: websocket_url.clone(),
                timeout: connect_timeout,
            })?
            .map_err(|source| ExecServerError::WebSocketConnect {
                url: websocket_url.clone(),
                source,
            })?;

        Self::connect(
            JsonRpcConnection::from_websocket(
                stream,
                format!("exec-server websocket {websocket_url}"),
            ),
            args.into(),
        )
        .await
    }

    pub async fn connect_stdio_command(
        args: StdioExecServerConnectArgs,
    ) -> Result<Self, ExecServerError> {
        let mut child = stdio_command_process(&args.command)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(ExecServerError::Spawn)?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ExecServerError::Protocol("spawned exec-server command has no stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ExecServerError::Protocol("spawned exec-server command has no stdout".to_string())
        })?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                loop {
                    match lines.next_line().await {
                        Ok(Some(line)) => debug!("exec-server stdio stderr: {line}"),
                        Ok(None) => break,
                        Err(err) => {
                            warn!("failed to read exec-server stdio stderr: {err}");
                            break;
                        }
                    }
                }
            });
        }

        Self::connect(
            JsonRpcConnection::from_stdio(stdout, stdin, "exec-server stdio command".to_string())
                .with_transport_lifetime(Box::new(StdioChildGuard::spawn(child))),
            args.into(),
        )
        .await
    }
}

struct StdioChildGuard {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl StdioChildGuard {
    fn spawn(child: Child) -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        tokio::spawn(supervise_stdio_child(child, shutdown_rx));
        Self {
            shutdown_tx: Some(shutdown_tx),
        }
    }
}

impl Drop for StdioChildGuard {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

async fn supervise_stdio_child(mut child: Child, shutdown_rx: oneshot::Receiver<()>) {
    let shutdown_requested = tokio::select! {
        result = child.wait() => {
            if let Err(err) = result {
                debug!("failed to wait for exec-server stdio child: {err}");
            }
            false
        }
        _ = shutdown_rx => true,
    };

    if shutdown_requested {
        kill_stdio_child(&mut child);
        if let Err(err) = child.wait().await {
            debug!("failed to wait for exec-server stdio child after shutdown: {err}");
        }
    }
}

fn kill_stdio_child(child: &mut Child) {
    if let Err(err) = child.start_kill() {
        debug!("failed to terminate exec-server stdio child: {err}");
    }
}

fn stdio_command_process(stdio_command: &StdioExecServerCommand) -> Command {
    let mut command = Command::new(&stdio_command.program);
    command.args(&stdio_command.args);
    command.envs(&stdio_command.env);
    if let Some(cwd) = &stdio_command.cwd {
        command.current_dir(cwd);
    }
    command
}
