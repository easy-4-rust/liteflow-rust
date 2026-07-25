//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.SwitchCondition
//!
//! "id:tag" 目标匹配规则、default、NoSwitchTargetNodeException。
//!
//! 差异说明：
//! - Java 先校验 switch 节点类型为 SWITCH/SWITCH_SCRIPT（否则抛
//!   SwitchTypeErrorException）；Rust 端节点类型由 builder 保证，结果非
//!   string 时报 NodeTypeError（同语义）。
//! - Java 通过 slot.getSwitchResult(类名) 取选择结果；Rust 端 switch 节点
//!   直接返回 Value::String。

use super::check_not_pre_finally;
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct SwitchCondition {
    switch_node: Arc<dyn Executable>,
    target_list: Vec<Arc<dyn Executable>>,
    default_executor: Option<Arc<dyn Executable>>,
}

impl SwitchCondition {
    pub fn new(
        switch_node: Arc<dyn Executable>,
        target_list: Vec<Arc<dyn Executable>>,
        default_executor: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { switch_node, target_list, default_executor }
    }

    /// 对应 Java SwitchCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Switch
    }
}

#[async_trait]
impl Executable for SwitchCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 对应 Java SwitchCondition#executeCondition：先判断 isAccess，
        // 返回 false 则整个 SWITCH 表达式不执行
        if !self.switch_node.is_access(ctx, frame).await {
            return Ok(Value::Null);
        }
        let v = self.switch_node.execute(ctx, frame).await?;
        let target_id = match &v {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => {
                return Err(LiteflowError::NodeTypeError {
                    node: self.switch_node.id().to_string(),
                    expect: "string".into(),
                    actual: other.to_string(),
                })
            }
        };

        let mut target: Option<&Arc<dyn Executable>> = None;
        if !target_id.is_empty() {
            // 对齐 Java 的 tag 匹配规则："id:tag" / ":tag" / "id"
            if target_id.contains(':') {
                let mut parts = target_id.splitn(2, ':');
                let tid = parts.next().unwrap_or("");
                let ttag = parts.next().unwrap_or("");
                target = self.target_list.iter().find(|e| {
                    (tid.starts_with("tag") && e.tag() == Some(ttag))
                        || ((tid.is_empty() || tid == e.id())
                            && (ttag.is_empty() || e.tag() == Some(ttag)))
                });
            } else {
                target = self.target_list.iter().find(|e| e.id() == target_id);
            }
        }
        let target = target.or(self.default_executor.as_ref());
        match target {
            Some(t) => {
                check_not_pre_finally(t.as_ref(), self.switch_node.id())?;
                t.execute(ctx, frame).await
            }
            None => Err(LiteflowError::NoSwitchTarget(target_id)),
        }
    }

    fn id(&self) -> &str {
        "SWITCH"
    }
}
