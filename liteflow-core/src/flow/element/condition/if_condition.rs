//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.IfCondition
//!
//! 条件结果驱动 true/false 分支（含 ELIF）。
//! isAccess=false 直接返回；true/false 目标不可为 pre/finally。
//!
//! 差异说明：
//! - Java 先校验 if 节点类型为 IF/IF_SCRIPT（否则抛 IfTypeErrorException）；
//!   Rust 端节点类型由 builder 保证，结果非 bool 时报 NodeTypeError（同语义）。
//! - Java 通过 slot.getIfResult(类名) 取条件结果；Rust 端条件节点直接返回
//!   Value::Bool（见 crate::flow::element::node）。

use super::{check_not_pre_finally, expect_bool};
use crate::enums::ConditionTypeEnum;
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

    /// 对应 Java IfCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::If
    }
}

#[async_trait]
impl Executable for IfCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 对应 Java IfCondition#executeCondition：先判断 isAccess，
        // 返回 false 则整个 IF 表达式不执行
        if !self.if_item.is_access(ctx, frame).await {
            return Ok(Value::Null);
        }
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
