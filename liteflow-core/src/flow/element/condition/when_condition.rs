//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.WhenCondition
//!
//! 并行器：并行执行，委托给 ParallelStrategyExecutor。
//! 字段对齐 Java：ignoreError / any / percentage / specifyIdSet(must) /
//! maxWaitTime / threadExecutorClass。
//!
//! 差异说明：
//! - Java 的 group 字段在 2.10.0 已弃用，未迁移。
//! - Java 用独立线程池（threadExecutorClass）+ CompletableFuture；Rust 通过
//!   ExecutorHelper 选择有界 Tokio 执行器，并保持 Condition > Chain > 全局优先级。
//! - Java 超时以 WhenFutureObj.timeOut 标记并 warn；Rust 整体超时返回
//!   LiteflowError::WhenTimeout。

use super::Condition;
use crate::enums::{ConditionTypeEnum, ParallelStrategyEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::parallel::strategy::{ParallelOpts, ParallelStrategyHelper};
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorHelper;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

pub struct WhenCondition {
    executable_list: Vec<Arc<dyn Executable>>,
    pub ignore_error: bool,
    pub any: bool,
    pub percentage: Option<f64>,
    /// specifyIdSet（按节点 id / 别名匹配）
    pub must: Vec<String>,
    pub max_wait_ms: Option<u64>,
    /// threadExecutorClass（交给 ExecutorHelper 解析并缓存）
    pub thread_executor: Option<String>,
}

impl WhenCondition {
    pub fn new(executable_list: Vec<Arc<dyn Executable>>) -> Self {
        Self {
            executable_list,
            ignore_error: false,
            any: false,
            percentage: None,
            must: Vec::new(),
            max_wait_ms: None,
            thread_executor: None,
        }
    }

    fn strategy(&self) -> ParallelStrategyEnum {
        if !self.must.is_empty() {
            ParallelStrategyEnum::Specify
        } else if self.any {
            ParallelStrategyEnum::Any
        } else if self.percentage.is_some() {
            ParallelStrategyEnum::Percentage
        } else {
            ParallelStrategyEnum::All
        }
    }

    /// must id → 分支序号
    fn must_idx(items: &[Arc<dyn Executable>], must: &[String]) -> HashSet<usize> {
        items
            .iter()
            .enumerate()
            .filter(|(_, e)| must.iter().any(|m| m == e.id()))
            .map(|(i, _)| i)
            .collect()
    }

    /// 对应 Java WhenCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::When
    }
}

#[async_trait]
impl Executable for WhenCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        // 对应 Java WhenCondition#executeAsyncCondition 的 stream 过滤：
        // 1. 过滤掉 PreCondition / FinallyCondition（EL Chain 处理时已提出）
        // 2. 过滤 isAccess 为 false 的分支（不过滤的话 any 模式下它会最快返回）
        let mut items: Vec<Arc<dyn Executable>> = Vec::with_capacity(self.executable_list.len());
        for e in &self.executable_list {
            if e.is_pre_or_finally() {
                continue;
            }
            if e.is_access(ctx, frame).await {
                items.push(e.clone());
            }
        }
        if items.is_empty() {
            return Ok(Value::Null);
        }
        let condition_key = format!("{:p}", self);
        let executor_service = ExecutorHelper::load_instance().build_executor_service(
            self.thread_executor.as_deref(),
            frame.chain_thread_pool(),
            &condition_key,
            &ctx.inner.chain_id,
            ConditionTypeEnum::When,
        )?;
        let opts = ParallelOpts {
            ignore_error: self.ignore_error,
            must_idx: Self::must_idx(&items, &self.must),
            percentage: self.percentage,
            executor_service,
        };
        let executor =
            ParallelStrategyHelper::load_instance().build_parallel_executor(Some(self.strategy()));
        // 并行分支共享同一 slot（Java 同一 slotIndex）
        let fut = executor.execute(items, &opts, ctx.clone(), frame.clone());
        match self.max_wait_ms {
            Some(ms) => match tokio::time::timeout(Duration::from_millis(ms), fut).await {
                Ok(r) => r,
                Err(_) => Err(LiteflowError::WhenTimeout),
            },
            None => fut.await,
        }
    }

    fn id(&self) -> &str {
        "WHEN"
    }
}

impl Condition for WhenCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        WhenCondition::condition_type(self)
    }
}
