//! Rhai 脚本组件按节点类别构建入口。

use std::sync::Arc;

use crate::LFResult;
use crate::core::NodeComponent;

use super::{
    ScriptBooleanComponent, ScriptCommonComponent, ScriptForComponent, ScriptIteratorComponent,
    ScriptKind, ScriptSwitchComponent,
};

/// 根据脚本类别构建独立的 Java 对等组件对象。
pub fn build_rhai_component(
    node_id: &str,
    kind: ScriptKind,
    script: &str,
) -> LFResult<Arc<dyn NodeComponent>> {
    match kind {
        ScriptKind::Common => Ok(Arc::new(ScriptCommonComponent::new(node_id, script)?)),
        ScriptKind::Boolean => Ok(Arc::new(ScriptBooleanComponent::new(node_id, script)?)),
        ScriptKind::Switch => Ok(Arc::new(ScriptSwitchComponent::new(node_id, script)?)),
        ScriptKind::For => Ok(Arc::new(ScriptForComponent::new(node_id, script)?)),
        ScriptKind::Iterator => Ok(Arc::new(ScriptIteratorComponent::new(node_id, script)?)),
    }
}
