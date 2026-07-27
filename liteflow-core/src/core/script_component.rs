//! 对应 Java: `com.yomahub.liteflow.core.ScriptComponent`。

use crate::exception::{LFResult, LiteflowError};
use crate::script::{RhaiScriptExecutor, ScriptExecuteWrap, ScriptExecutor};
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

    /// 重新加载当前节点的 Rhai 脚本。
    ///
    /// 参数 `script`、`language` 对应 Java `ScriptComponent#loadScript`；本对象是
    /// core 内建 Rhai 基座，其他语言由 `ScriptExecutorFactory` 构建各自组件。
    pub fn load_script(&self, script: &str, language: &str) -> LFResult<()> {
        if language != "rhai" {
            return Err(LiteflowError::Script {
                node: self.node_id.clone(),
                msg: format!(
                    "ScriptComponent only owns the rhai executor, unsupported language: {language}"
                ),
            });
        }
        self.executor.load(&self.node_id, script)
    }

    /// 返回脚本节点 id。
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 返回底层 Rhai 脚本执行器。
    ///
    /// 四种 Java 对等脚本组件通过该入口委托访问、切面与回滚钩子。
    #[must_use]
    pub fn executor(&self) -> &RhaiScriptExecutor {
        &self.executor
    }

    /// 基于当前上下文创建 Java `ScriptExecuteWrap` 快照。
    #[must_use]
    pub fn build_wrap(&self, context: &CmpContext) -> ScriptExecuteWrap {
        ScriptExecuteWrap::from_context(context)
    }

    /// 卸载当前节点的 Rhai 编译产物。
    ///
    /// 对应 Java: `ScriptExecutor#unLoad`。
    pub fn unload(&self) -> LFResult<()> {
        self.executor.unload(&self.node_id)
    }
}
