use std::sync::Arc;

use agentscope_core::tool::{AgentTool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

use super::WorkspaceFileTools;

/// 将 Java `deleteFile` 注解方法适配为 AgentScope 工具。
///
/// 对应 Java: `WorkspaceFileTools#deleteFile`。
pub struct DeleteFileTool {
    workspace: Arc<WorkspaceFileTools>,
}

impl DeleteFileTool {
    /// 创建绑定到指定会话工作区的删除工具。
    #[must_use]
    pub fn new(workspace: Arc<WorkspaceFileTools>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl AgentTool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file in the current workspace"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {"path": {"type": "string", "description": "Relative path"}},
            "required": ["path"]
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn execute(&self, context: ToolContext) -> ToolResult {
        let Some(path) = context.get_string("path") else {
            return ToolResult::error("delete_file requires string argument: path");
        };
        match self.workspace.delete_file(path) {
            Ok(output) => ToolResult::success(output),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }
}
