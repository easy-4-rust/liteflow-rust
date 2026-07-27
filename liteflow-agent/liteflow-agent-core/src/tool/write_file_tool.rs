use std::sync::Arc;

use agentscope_core::tool::{AgentTool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

use super::WorkspaceFileTools;

/// 将 Java `writeFile` 注解方法适配为 AgentScope 工具。
///
/// 对应 Java: `WorkspaceFileTools#writeFile`。
pub struct WriteFileTool {
    workspace: Arc<WorkspaceFileTools>,
}

impl WriteFileTool {
    /// 创建绑定到指定会话工作区的写入工具。
    #[must_use]
    pub fn new(workspace: Arc<WorkspaceFileTools>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl AgentTool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write text to a file in the current workspace (overwrite)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative path"},
                "content": {"type": "string", "description": "File content"}
            },
            "required": ["path", "content"]
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn execute(&self, context: ToolContext) -> ToolResult {
        let Some(path) = context.get_string("path") else {
            return ToolResult::error("write_file requires string argument: path");
        };
        let Some(content) = context.get_string("content") else {
            return ToolResult::error("write_file requires string argument: content");
        };
        match self.workspace.write_file(path, content) {
            Ok(output) => ToolResult::success(output),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }
}
