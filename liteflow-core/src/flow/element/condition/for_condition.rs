//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.ForCondition
//!
//! 计数循环、DO/BREAK、PARALLEL 并行（PARALLEL 为 Rust 端扩展形态）。
//!
//! 差异说明：
//! - Java 在 forNode 为空时抛 NoForNodeException；Rust 端 for_node 为
//!   非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 slot.getForResult(类名) 取循环次数；Rust 端 for 节点直接
//!   返回 Value::Number。

use super::loop_condition::{handle_future_list, run_sequential, submit_iteration};
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct ForCondition {
    for_node: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl ForCondition {
    pub fn new(
        for_node: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { for_node, parallel, do_executor, break_item }
    }

    /// 对应 Java ForCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::For
    }
}

#[async_trait]
impl Executable for ForCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 对应 Java ForCondition#executeCondition：先判断 isAccess，
        // 返回 false 则整个 FOR 表达式不执行
        if !self.for_node.is_access(ctx, frame).await {
            return Ok(Value::Null);
        }
        let v = self.for_node.execute(ctx, frame).await?;
        let count = match &v {
            Value::Number(n) => n.as_u64().unwrap_or(0) as usize,
            Value::String(s) => {
                s.parse::<usize>().map_err(|_| LiteflowError::NodeTypeError {
                    node: self.for_node.id().to_string(),
                    expect: "number".into(),
                    actual: v.to_string(),
                })?
            }
            other => {
                return Err(LiteflowError::NodeTypeError {
                    node: self.for_node.id().to_string(),
                    expect: "number".into(),
                    actual: other.to_string(),
                })
            }
        };

        if self.parallel.is_some() {
            let mut set: JoinSet<LFResult<Value>> = JoinSet::new();
            for i in 0..count {
                if !submit_iteration(&mut set, &self.do_executor, self.break_item.as_ref(), ctx, frame, i, None).await? {
                    break;
                }
            }
            return handle_future_list(set).await;
        }

        for i in 0..count {
            if !run_sequential(&self.do_executor, self.break_item.as_ref(), ctx, frame, i, None).await? {
                break;
            }
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "FOR"
    }
}
