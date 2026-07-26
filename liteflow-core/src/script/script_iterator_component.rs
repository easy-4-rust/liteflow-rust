//! Rust 端迭代集合脚本组件。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::{CmpContext, LFResult};

use super::{ScriptComponent, ScriptKind};

/// 返回数组的迭代脚本节点。
pub struct ScriptIteratorComponent {
    script_component: ScriptComponent,
}

impl ScriptIteratorComponent {
    /// 编译脚本并创建迭代集合脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }
}

#[async_trait]
impl NodeComponent for ScriptIteratorComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        let value = self.script_component.process_script(ctx)?;
        ScriptKind::Iterator.check_return(self.name(), value)
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }
}
