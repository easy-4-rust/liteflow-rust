use std::sync::Arc;

use agentscope_core::tool::{AgentTool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;

use super::WorkspaceFileTools;

/// 将 Java `listFiles` 注解方法适配为 AgentScope 工具。
///
/// 对应 Java: `WorkspaceFileTools#listFiles`。
pub struct ListFilesTool {
    workspace: Arc<WorkspaceFileTools>,
}

impl ListFilesTool {
    /// 创建绑定到指定会话工作区的目录列表工具。
    #[must_use]
    pub fn new(workspace: Arc<WorkspaceFileTools>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl AgentTool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files in a workspace directory"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path; defaults to current dir"
                }
            }
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, context: ToolContext) -> ToolResult {
        match self.workspace.list_files(context.get_string("path")) {
            Ok(paths) => match serde_json::to_string(&paths) {
                Ok(output) => ToolResult::success(output),
                Err(error) => ToolResult::error(error.to_string()),
            },
            Err(error) => ToolResult::error(error.to_string()),
        }
    }
}
