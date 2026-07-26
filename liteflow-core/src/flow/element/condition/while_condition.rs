//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.WhileCondition
//!
//! 条件循环（含并行形态，PARALLEL 为 Rust 端扩展形态）。
//!
//! 差异说明：
//! - Java 在 whileNode 为空时抛 NoWhileNodeException；Rust 端 while_item 为
//!   非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 slot.getWhileResult(类名) 取循环条件；Rust 端 while 节点直接
//!   返回 Value::Bool。

use super::loop_condition::{handle_future_list, run_sequential, submit_iteration};
use super::{Condition, expect_bool};
use crate::enums::ConditionTypeEnum;
use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::flow::parallel::LoopFutureObj;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorHelper;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct WhileCondition {
    while_item: Arc<dyn Executable>,
    pub parallel: Option<usize>,
    do_executor: Arc<dyn Executable>,
    break_item: Option<Arc<dyn Executable>>,
}

impl WhileCondition {
    pub fn new(
        while_item: Arc<dyn Executable>,
        parallel: Option<usize>,
        do_executor: Arc<dyn Executable>,
        break_item: Option<Arc<dyn Executable>>,
    ) -> Self {
        Self {
            while_item,
            parallel,
            do_executor,
            break_item,
        }
    }

    /// 对应 Java WhileCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::While
    }
}

#[async_trait]
impl Executable for WhileCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 对应 Java WhileCondition#executeCondition：先判断 isAccess，
        // 返回 false 则整个 WHILE 表达式不执行
        if !self.while_item.is_access(ctx, frame).await {
            return Ok(Value::Null);
        }
        let mut index = 0usize;
        if self.parallel.is_some() {
            let condition_key = format!("{:p}", self);
            let executor_service = ExecutorHelper::load_instance().build_executor_service(
                frame.condition_thread_pool(),
                frame.chain_thread_pool(),
                &condition_key,
                &ctx.inner.chain_id,
                ConditionTypeEnum::While,
            )?;
            let mut set: JoinSet<LoopFutureObj> = JoinSet::new();
            loop {
                let f = frame.push(index, None);
                let v = self.while_item.execute(ctx, &f).await?;
                if !expect_bool(self.while_item.id(), &v)? {
                    break;
                }
                if !submit_iteration(
                    &mut set,
                    &self.do_executor,
                    self.break_item.as_ref(),
                    ctx,
                    frame,
                    index,
                    None,
                    &executor_service,
                )
                .await?
                {
                    break;
                }
                index += 1;
            }
            return handle_future_list(set).await;
        }

        loop {
            let f = frame.push(index, None);
            let v = self.while_item.execute(ctx, &f).await?;
            if !expect_bool(self.while_item.id(), &v)? {
                break;
            }
            if !run_sequential(
                &self.do_executor,
                self.break_item.as_ref(),
                ctx,
                frame,
                index,
                None,
            )
            .await?
            {
                break;
            }
            index += 1;
        }
        Ok(Value::Null)
    }

    fn id(&self) -> &str {
        "WHILE"
    }
}

impl Condition for WhileCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        WhileCondition::condition_type(self)
    }
}
