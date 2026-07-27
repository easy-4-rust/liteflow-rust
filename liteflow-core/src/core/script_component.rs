//! 对应 Java: `com.yomahub.liteflow.core.ScriptComponent`。

use crate::exception::LFResult;
use crate::script::{RhaiScriptExecutor, ScriptExecutor};
use crate::slot::CmpContext;
use serde_json::Value;

/// Rhai 脚本组件公共基座：构建期编译，执行期负责上下文绑定与求值。
pub struct ScriptComponent {
    node_id: String,
    executor: RhaiScriptExecutor,
}

impl ScriptComponent {
    /// 编译指定节点的脚本。
    ///
    /// 对应 Java `ScriptComponent#setScript` 与 `ScriptExecutor#load`。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        let executor = RhaiScriptExecutor::new();
        executor.load(node_id, script)?;
        Ok(Self {
            node_id: node_id.to_string(),
            executor,
        })
    }

    /// 执行已编译脚本并返回 JSON 结果。
    ///
    /// 对应 Java `ScriptComponent#processScript`。
    pub fn process_script(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.executor.execute(&self.node_id, ctx)
    }

    /// 返回脚本节点 id。
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}
