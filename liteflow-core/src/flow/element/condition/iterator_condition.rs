//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.IteratorCondition
//!
//! 迭代循环，loopObject 传递（含并行形态，PARALLEL 为 Rust 端扩展形态）。
//!
//! 差异说明：
//! - Java 在 iteratorNode 为空时抛 NoIteratorNodeException；Rust 端
//!   iterator_node 为非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 slot.getIteratorResult(类名) 取 Iterator；Rust 端 iterator
//!   节点直接返回 Value::Array。

use super::loop_condition::{handle_future_list, run_sequential, submit_iteration};
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct IteratorCondition {
    iterator_node: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl IteratorCondition {
    pub fn new(
        iterator_node: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self { iterator_node, parallel, do_executor, break_item }
    }

    /// 对应 Java IteratorCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Iterator
    }
}

#[async_trait]
impl Executable for IteratorCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 对应 Java IteratorCondition#executeCondition：先判断 isAccess，
        // 返回 false 则整个 ITERATOR 表达式不执行
        if !self.iterator_node.is_access(ctx, frame).await {
            return Ok(Value::Null);
        }
        let v = self.iterator_node.execute(ctx, frame).await?;
        let list = match &v {
            Value::Array(a) => a.clone(),
            other => {
                return Err(LiteflowError::NodeTypeError {
                    node: self.iterator_node.id().to_string(),
                    expect: "array".into(),
                    actual: other.to_string(),
                })
            }
        };

        if self.parallel.is_some() {
            let mut set: JoinSet<LFResult<Value>> = JoinSet::new();
            for (i, obj) in list.iter().enumerate() {
                if !submit_iteration(&mut set, &self.do_executor, self.break_item.as_ref(), ctx, frame, i, Some(obj.clone())).await? {
                    break;
                }
            }
            return handle_future_list(set).await;
        }

        for (i, obj) in list.into_iter().enumerate() {
            if !run_sequential(&self.do_executor, self.break_item.as_ref(), ctx, frame, i, Some(obj)).await? {
                break;
            }
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "ITERATOR"
    }
}
