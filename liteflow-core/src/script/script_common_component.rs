//! 对应 Java: `com.yomahub.liteflow.core.ScriptCommonComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::{CmpContext, LFResult};

use super::ScriptComponent;

/// 普通脚本节点，直接返回脚本执行结果。
pub struct ScriptCommonComponent {
    script_component: ScriptComponent,
}

impl ScriptCommonComponent {
    /// 编译脚本并创建普通脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }
}

#[async_trait]
impl NodeComponent for ScriptCommonComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.script_component.process_script(ctx)
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }
}
