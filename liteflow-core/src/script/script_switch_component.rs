//! 对应 Java: `com.yomahub.liteflow.core.ScriptSwitchComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::{CmpContext, LFResult};

use super::{ScriptComponent, ScriptKind};

/// 选择脚本节点，返回目标节点或标签字符串。
pub struct ScriptSwitchComponent {
    script_component: ScriptComponent,
}

impl ScriptSwitchComponent {
    /// 编译脚本并创建选择脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }
}

#[async_trait]
impl NodeComponent for ScriptSwitchComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        let value = self.script_component.process_script(ctx)?;
        ScriptKind::Switch.check_return(self.name(), value)
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }
}
