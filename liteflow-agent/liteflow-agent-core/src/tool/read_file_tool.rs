use std::sync::Arc;

use agentscope_core::tool::{AgentTool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

use super::WorkspaceFileTools;

/// 将 Java `readFile` 注解方法适配为 AgentScope 工具。
///
/// 对应 Java: `WorkspaceFileTools#readFile`。
pub struct ReadFileTool {
    workspace: Arc<WorkspaceFileTools>,
}

impl ReadFileTool {
    /// 创建绑定到指定会话工作区的读取工具。
    #[must_use]
    pub fn new(workspace: Arc<WorkspaceFileTools>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl AgentTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read a text file in the current workspace"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {"path": {"type": "string", "description": "Relative path"}},
            "required": ["path"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, context: ToolContext) -> ToolResult {
        let Some(path) = context.get_string("path") else {
            return ToolResult::error("read_file requires string argument: path");
        };
        match self.workspace.read_file(path) {
            Ok(output) => ToolResult::success(output),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }
}
