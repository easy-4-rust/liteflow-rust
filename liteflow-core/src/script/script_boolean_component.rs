//! 对应 Java: `com.yomahub.liteflow.core.ScriptBooleanComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::{CmpContext, LFResult};

use super::{ScriptComponent, ScriptKind};

/// 布尔脚本节点，用于 IF/WHILE/BREAK 条件判断。
pub struct ScriptBooleanComponent {
    script_component: ScriptComponent,
}

impl ScriptBooleanComponent {
    /// 编译脚本并创建布尔脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }
}

#[async_trait]
impl NodeComponent for ScriptBooleanComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        let value = self.script_component.process_script(ctx)?;
        ScriptKind::Boolean.check_return(self.name(), value)
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }
}
