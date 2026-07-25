//! 对应 IfCondition：条件结果驱动 true/false 分支（含 ELIF）。
//! isAccess=false 直接返回；true/false 目标不可为 pre/finally。

use super::{check_not_pre_finally, expect_bool};
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct IfCondition {
    if_item: Arc<dyn Executable>,
    true_case: Arc<dyn Executable>,
    /// (elif 条件, elif 目标)
    elif_list: Vec<(Arc<dyn Executable>, Arc<dyn Executable>)>,
    false_case: Option<Arc<dyn Executable>>,
}

impl IfCondition {
    pub fn new(
        if_item: Arc<dyn Executable>,
        true_case: Arc<dyn Executable>,
        elif_list: Vec<(Arc<dyn Executable>, Arc<dyn Executable>)>,
        false_case: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { if_item, true_case, elif_list, false_case }
    }
}

#[async_trait]
impl Executable for IfCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let v = self.if_item.execute(ctx, frame).await?;
        if expect_bool(self.if_item.id(), &v)? {
            check_not_pre_finally(self.true_case.as_ref(), self.if_item.id())?;
            return self.true_case.execute(ctx, frame).await;
        }
        for (c, t) in &self.elif_list {
            let v = c.execute(ctx, frame).await?;
            if expect_bool(c.id(), &v)? {
                check_not_pre_finally(t.as_ref(), self.if_item.id())?;
                return t.execute(ctx, frame).await;
            }
        }
        if let Some(f) = &self.false_case {
            check_not_pre_finally(f.as_ref(), self.if_item.id())?;
            return f.execute(ctx, frame).await;
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "IF"
    }
}
