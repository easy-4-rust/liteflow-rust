//! 对应 Java: `com.yomahub.liteflow.core.ScriptForComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::{NodeComponent, ScriptComponent};
use crate::script::ScriptKind;
use crate::{CmpContext, LFResult};

/// 循环次数脚本节点，返回非负数值。
pub struct ScriptForComponent {
    script_component: ScriptComponent,
}

impl ScriptForComponent {
    /// 编译脚本并创建循环次数脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }
}

#[async_trait]
impl NodeComponent for ScriptForComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        let value = self.script_component.process_script(ctx)?;
        ScriptKind::For.check_return(self.name(), value)
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }
}
