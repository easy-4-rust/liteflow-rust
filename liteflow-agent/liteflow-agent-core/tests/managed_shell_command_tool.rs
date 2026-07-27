//! `ManagedShellCommandTool` 的真实子进程、安全策略和 AgentScope 适配测试。

use std::time::{Duration, Instant};

use agentscope_core::tool::{AgentTool, ToolContext};
use liteflow_agent_core::{AgentConfig, ManagedShellCommandTool, ShellMode};
use serde_json::{Value, json};

fn tool_config(workspace: &std::path::Path) -> AgentConfig {
    let mut config = AgentConfig::default();
    config.workspace.root = Some(workspace.to_string_lossy().into_owned());
    config
}

#[tokio::test]
async fn shell_tool_executes_in_workspace_and_limits_combined_output() {
    let workspace = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = tool_config(workspace.path());
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);

    let output = tool.execute_command("pwd").await;
    assert_eq!(
        output.trim(),
        workspace
            .path()
            .canonicalize()
            .expect("应规范化临时目录")
            .to_string_lossy()
    );

    config.shell.max_output_bytes = 5;
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);
    let output = tool.execute_command("printf 123456789").await;
    assert_eq!(output, "12345", "输出应按 max_output_bytes 截断");
}

#[tokio::test]
async fn shell_tool_enforces_modes_syntax_and_path_boundaries() {
    let workspace = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = tool_config(workspace.path());
    config.shell.whitelist = vec!["echo".to_string(), "cat".to_string()];
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);

    assert_error_contains(
        &tool.execute_command("pwd").await,
        "not allowed by whitelist",
    );
    assert_error_contains(
        &tool.execute_command("echo hello | cat").await,
        "unsupported shell syntax",
    );
    assert_error_contains(
        &tool.execute_command("cat ../outside.txt").await,
        "parent-directory traversal",
    );
    assert_error_contains(
        &tool.execute_command("cat /etc/passwd").await,
        "absolute paths",
    );

    config.shell.mode = ShellMode::Blacklist;
    config.shell.blacklist = vec!["echo".to_string()];
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);
    assert_error_contains(
        &tool.execute_command("echo denied").await,
        "not allowed by blacklist",
    );

    config.shell.mode = ShellMode::Disabled;
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);
    assert_error_contains(
        &tool.execute_command("pwd").await,
        "shell execution denied by policy",
    );
}

#[tokio::test]
async fn shell_tool_kills_process_after_configured_timeout() {
    let workspace = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = tool_config(workspace.path());
    config.shell.whitelist.push("sleep".to_string());
    config.shell.timeout = Duration::from_millis(25);
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);

    let started = Instant::now();
    let output = tool.execute_command("sleep 2").await;
    assert_error_contains(&output, "timeout after 25ms");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "超时命令必须被主动终止"
    );
}

#[tokio::test]
async fn agentscope_adapter_returns_java_compatible_json_error_text() {
    let workspace = tempfile::tempdir().expect("应创建临时工作区");
    let config = tool_config(workspace.path());
    let tool = ManagedShellCommandTool::new(workspace.path(), &config);
    let result = tool
        .execute(ToolContext::with_name(
            "shell-1",
            "execute_shell_command",
            json!({"command": "rm forbidden"}),
        ))
        .await;

    assert!(result.success, "Java 工具将策略拒绝作为普通 JSON 文本返回");
    assert_error_contains(&result.output, "not allowed by whitelist");
}

fn assert_error_contains(output: &str, expected: &str) {
    let parsed: Value = serde_json::from_str(output)
        .unwrap_or_else(|error| panic!("应返回 JSON 错误，实际 {output:?}: {error}"));
    let message = parsed
        .get("error")
        .and_then(Value::as_str)
        .expect("JSON 应包含 error 字段");
    assert!(
        message.contains(expected),
        "错误 {message:?} 应包含 {expected:?}"
    );
}
