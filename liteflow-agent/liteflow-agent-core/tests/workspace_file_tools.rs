//! `WorkspaceFileTools` 的真实文件系统与 AgentScope 工具执行测试。

use std::sync::Arc;

use agentscope_core::tool::{AgentTool, ToolContext};
use liteflow_agent_core::{AgentError, WorkspaceConfig, WorkspaceFileTools};
use serde_json::json;

fn workspace_config(root: &std::path::Path) -> WorkspaceConfig {
    WorkspaceConfig {
        root: Some(root.to_string_lossy().into_owned()),
        max_file_bytes: 5,
        max_list_size: 2,
        ..WorkspaceConfig::default()
    }
}

#[tokio::test]
async fn workspace_tools_execute_real_write_read_list_and_delete() {
    let temporary = tempfile::tempdir().expect("应创建临时目录");
    let config = workspace_config(temporary.path());
    let root = WorkspaceFileTools::prepare_root(&config)
        .expect("应初始化工作区")
        .expect("已配置根目录");
    let workspace =
        WorkspaceFileTools::for_conversation(&root, "会话 / 1", &config).expect("应创建会话工作区");
    let workspace_path = workspace.workspace().to_path_buf();
    let tools = workspace.tools();

    let write = tool(&tools, "write_file");
    let result = write
        .execute(ToolContext::with_name(
            "write-1",
            "write_file",
            json!({"path": "nested/note.txt", "content": "123456789"}),
        ))
        .await;
    assert!(result.success, "{:?}", result.error);
    assert_eq!(
        std::fs::read_to_string(workspace_path.join("nested/note.txt")).expect("应真实写入文件"),
        "123456789"
    );

    let read = tool(&tools, "read_file");
    let result = read
        .execute(ToolContext::with_name(
            "read-1",
            "read_file",
            json!({"path": "nested/note.txt"}),
        ))
        .await;
    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.output, "12345", "读取应按 max_file_bytes 截断");

    std::fs::write(workspace_path.join("second.txt"), "2").expect("应创建第二个文件");
    std::fs::write(workspace_path.join("third.txt"), "3").expect("应创建第三个文件");
    let list = tool(&tools, "list_files");
    let result = list
        .execute(ToolContext::with_name("list-1", "list_files", json!({})))
        .await;
    assert!(result.success, "{:?}", result.error);
    let listed: Vec<String> = serde_json::from_str(&result.output).expect("列表应为 JSON 数组");
    assert_eq!(listed.len(), 2, "列表应受 max_list_size 限制");

    let delete = tool(&tools, "delete_file");
    let result = delete
        .execute(ToolContext::with_name(
            "delete-1",
            "delete_file",
            json!({"path": "nested/note.txt"}),
        ))
        .await;
    assert!(result.success, "{:?}", result.error);
    assert!(!workspace_path.join("nested/note.txt").exists());
}

#[test]
fn workspace_rejects_absolute_parent_and_symlink_escape_paths() {
    let temporary = tempfile::tempdir().expect("应创建临时目录");
    let outside = tempfile::tempdir().expect("应创建工作区外目录");
    let config = workspace_config(temporary.path());
    let root = WorkspaceFileTools::prepare_root(&config)
        .expect("应初始化工作区")
        .expect("已配置根目录");
    let workspace =
        WorkspaceFileTools::for_conversation(&root, "safe", &config).expect("应创建会话工作区");

    assert!(matches!(
        workspace.write_file("../escape.txt", "denied"),
        Err(AgentError::WorkspacePathDenied(_))
    ));
    assert!(matches!(
        workspace.read_file("/etc/passwd"),
        Err(AgentError::WorkspacePathDenied(_))
    ));

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), workspace.workspace().join("outside"))
            .expect("应创建测试符号链接");
        assert!(matches!(
            workspace.write_file("outside/escape.txt", "denied"),
            Err(AgentError::WorkspacePathDenied(_))
        ));
        assert!(!outside.path().join("escape.txt").exists());
    }
}

#[test]
fn workspace_root_respects_auto_create_configuration() {
    let temporary = tempfile::tempdir().expect("应创建临时目录");
    let missing = temporary.path().join("missing");
    let mut config = workspace_config(&missing);
    config.auto_create = false;
    assert!(matches!(
        WorkspaceFileTools::prepare_root(&config),
        Err(AgentError::WorkspaceRootDoesNotExist(_))
    ));

    config.auto_create = true;
    let root = WorkspaceFileTools::prepare_root(&config)
        .expect("auto_create 应创建目录")
        .expect("已配置根目录");
    assert!(root.is_dir());
}

fn tool<'a>(tools: &'a [Arc<dyn AgentTool>], name: &str) -> &'a Arc<dyn AgentTool> {
    tools
        .iter()
        .find(|tool| tool.name() == name)
        .unwrap_or_else(|| panic!("缺少工具: {name}"))
}
