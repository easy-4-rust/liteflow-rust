//! 对应 SwitchCondition："id:tag" 目标匹配规则、default、NoSwitchTargetNodeException。

use super::check_not_pre_finally;
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
}

#[async_trait]
impl Executable for SwitchCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
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
