use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::{AgentConfig, ShellConfig, ShellMode};
use agentscope_core::tool::{AgentTool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

/// 在会话工作区中执行受策略约束的单条外部命令。
///
/// 命令不会交给系统 shell 解释，而是按空白拆分为程序及参数后直接启动。工具拒绝
/// 管道、重定向、命令连接符、绝对路径与父目录跳转，并按照首 token 应用白名单或
/// 黑名单。标准输出与标准错误会被合并到受限缓冲区，超时或取消时强制终止子进程。
///
/// 对应 Java:
/// `com.yomahub.liteflow.agent.tool.ManagedShellCommandTool`。
#[derive(Clone)]
pub struct ManagedShellCommandTool {
    workspace: PathBuf,
    shell: ShellConfig,
}

impl ManagedShellCommandTool {
    /// 创建绑定到指定会话工作区的受控命令工具。
    ///
    /// # 参数
    /// - `workspace`: 当前 conversation 的隔离工作目录。
    /// - `config`: Agent 根配置，工具会复制其中的 Shell 策略。
    ///
    /// # 返回
    /// 可注册到 AgentScope Toolkit 的工具对象。
    ///
    /// 对应 Java: `ManagedShellCommandTool#ManagedShellCommandTool`。
    #[must_use]
    pub fn new(workspace: impl AsRef<Path>, config: &AgentConfig) -> Self {
        Self {
            workspace: workspace.as_ref().to_path_buf(),
            shell: config.shell.clone(),
        }
    }

    /// 执行一条受控命令。
    ///
    /// # 参数
    /// - `command`: 单条命令字符串；不支持 shell 管道、重定向或连接语法。
    ///
    /// # 返回
    /// 成功时返回受大小限制的合并输出；策略拒绝或运行失败时返回
    /// `{"error":"..."}` JSON 文本。
    ///
    /// 对应 Java: `ManagedShellCommandTool#executeCommand`。
    pub async fn execute_command(&self, command: &str) -> String {
        self.execute_command_with_cancellation(command, Arc::new(AtomicBool::new(false)))
            .await
    }

    async fn execute_command_with_cancellation(
        &self,
        command: &str,
        cancelled: Arc<AtomicBool>,
    ) -> String {
        if self.shell.mode == ShellMode::Disabled {
            return error_json("shell execution denied by policy");
        }
        if command.trim().is_empty() {
            return error_json("empty command");
        }
        if contains_unsupported_shell_syntax(command) {
            return error_json(
                "unsupported shell syntax: pipes, redirection, and command chaining are not supported",
            );
        }

        let tokens = command.split_whitespace().collect::<Vec<_>>();
        let executable = tokens[0];
        if self.shell.mode == ShellMode::Whitelist
            && !self
                .shell
                .whitelist
                .iter()
                .any(|allowed| allowed == executable)
        {
            return error_json(format!("command '{executable}' not allowed by whitelist"));
        }
        if self.shell.mode == ShellMode::Blacklist
            && self
                .shell
                .blacklist
                .iter()
                .any(|denied| denied == executable)
        {
            return error_json(format!("command '{executable}' not allowed by blacklist"));
        }
        if tokens
            .iter()
            .skip(1)
            .any(|token| path_argument_escapes(token))
        {
            return error_json("absolute paths and parent-directory traversal are not allowed");
        }

        let workspace = self.workspace.clone();
        let arguments = tokens
            .iter()
            .skip(1)
            .map(|token| (*token).to_string())
            .collect::<Vec<_>>();
        let executable = executable.to_string();
        let timeout = self.shell.timeout;
        let max_output_bytes = self.shell.max_output_bytes;
        match tokio::task::spawn_blocking(move || {
            execute_blocking(
                &workspace,
                &executable,
                &arguments,
                timeout,
                max_output_bytes,
                &cancelled,
            )
        })
        .await
        {
            Ok(output) => output,
            Err(error) => error_json(format!("shell worker failed: {error}")),
        }
    }
}

#[async_trait]
impl AgentTool for ManagedShellCommandTool {
    fn name(&self) -> &str {
        "execute_shell_command"
    }

    fn description(&self) -> &str {
        "Execute a controlled shell command in the current workspace. Path traversal and blacklisted commands are blocked."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Single command string (pipes && || are rejected)"
                }
            },
            "required": ["command"]
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn execute(&self, context: ToolContext) -> ToolResult {
        let Some(command) = context.get_string("command").map(ToOwned::to_owned) else {
            return ToolResult::success(error_json("empty command"));
        };
        let cancelled = Arc::new(AtomicBool::new(context.cancellation.is_cancelled()));
        let cancellation_flag = Arc::clone(&cancelled);
        let cancellation = context.cancellation.clone();
        let watcher = tokio::spawn(async move {
            cancellation.cancelled().await;
            cancellation_flag.store(true, Ordering::Release);
        });
        let output = self
            .execute_command_with_cancellation(&command, cancelled)
            .await;
        watcher.abort();
        // Java 注解工具把策略错误作为普通 JSON 文本返回，而不是抛出异常。
        ToolResult::success(output)
    }
}

fn execute_blocking(
    workspace: &Path,
    executable: &str,
    arguments: &[String],
    timeout: Duration,
    max_output_bytes: u64,
    cancelled: &AtomicBool,
) -> String {
    let mut child = match Command::new(executable)
        .args(arguments)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return error_json(error.to_string()),
    };

    let output = Arc::new(Mutex::new(Vec::new()));
    let (done_sender, done_receiver) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        spawn_output_reader(
            stdout,
            Arc::clone(&output),
            max_output_bytes,
            done_sender.clone(),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_output_reader(
            stderr,
            Arc::clone(&output),
            max_output_bytes,
            done_sender.clone(),
        );
    }
    drop(done_sender);

    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return error_json("shell execution cancelled");
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return error_json(format!("timeout after {}ms", timeout.as_millis()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return error_json(error.to_string());
            }
        }
    }

    // 与 Java outputFuture.get(1s) 对齐：进程退出后最多再等待一秒让两个流读完。
    let output_deadline = Instant::now() + Duration::from_secs(1);
    for _ in 0..2 {
        let Some(remaining) = output_deadline.checked_duration_since(Instant::now()) else {
            return error_json("output read timeout");
        };
        if done_receiver.recv_timeout(remaining).is_err() {
            return error_json("output read timeout");
        }
    }
    let bytes = output
        .lock()
        .expect("managed shell output lock poisoned")
        .clone();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn spawn_output_reader<R>(
    mut reader: R,
    output: Arc<Mutex<Vec<u8>>>,
    max_output_bytes: u64,
    done: mpsc::Sender<()>,
) where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 4_096];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            let mut output = output.lock().expect("managed shell output lock poisoned");
            let remaining = max_output_bytes.saturating_sub(output.len() as u64) as usize;
            output.extend_from_slice(&buffer[..read.min(remaining)]);
        }
        let _ = done.send(());
    });
}

fn contains_unsupported_shell_syntax(command: &str) -> bool {
    command.contains('|')
        || command.contains('<')
        || command.contains('>')
        || command.contains("&&")
        || command.contains("||")
        || command.contains(';')
}

fn path_argument_escapes(token: &str) -> bool {
    let path_escapes = |value: &str| {
        let path = Path::new(value);
        path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
    };
    path_escapes(token)
        || token
            .split_once('=')
            .is_some_and(|(_, value)| path_escapes(value))
}

fn error_json(message: impl AsRef<str>) -> String {
    json!({"error": message.as_ref()}).to_string()
}
