//! Rust 端迭代集合脚本组件。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::{CmpContext, LFResult};

use super::{ScriptComponent, ScriptKind};

/// 返回数组的迭代脚本节点。
///
/// Java 由 ScriptComponent 与具体脚本执行器的 `processIterator` 能力组合完成；
/// Rust 为保持 NodeComponent 强类型返回契约而抽出本包装对象，不对应独立 Java 类。
pub struct ScriptIteratorComponent {
    script_component: ScriptComponent,
}

impl ScriptIteratorComponent {
    /// 编译脚本并创建迭代集合脚本组件。
    ///
    /// 参数 `node_id`、`script` 分别是组件 ID 和脚本源码；编译成功返回组件，
    /// 失败返回脚本加载错误。
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

    fn unload_script(&self, _node_id: &str) -> LFResult<bool> {
        self.script_component.unload()?;
        Ok(true)
    }
}
