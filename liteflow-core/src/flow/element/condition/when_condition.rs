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

use super::{Condition, ConditionBase};
use crate::enums::{ConditionTypeEnum, ParallelStrategyEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::parallel::strategy::{ParallelOpts, ParallelStrategyHelper};
use crate::property::TimeUnit;
use crate::slot::{Ctx, Frame};
use crate::thread::ExecutorHelper;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct WhenCondition {
    base: ConditionBase,
    executable_list: Vec<Arc<dyn Executable>>,
    pub ignore_error: bool,
    /// Java 已弃用的 group 字段仍保留读写兼容。
    group: String,
    /// 显式设置的并行策略；未设置时由 any/percentage/must 推导。
    parallel_strategy: Option<ParallelStrategyEnum>,
    pub any: bool,
    pub percentage: Option<f64>,
    /// specifyIdSet（按节点 id / 别名匹配）
    pub must: Vec<String>,
    pub max_wait_ms: Option<u64>,
    /// Java setter/getter 保留的原始等待数值。
    max_wait_time: Option<u64>,
    /// Java setter/getter 保留的原始等待单位。
    max_wait_time_unit: TimeUnit,
    /// threadExecutorClass（交给 ExecutorHelper 解析并缓存）
    pub thread_executor: Option<String>,
}

impl WhenCondition {
    pub fn new(executable_list: Vec<Arc<dyn Executable>>) -> Self {
        Self {
            base: ConditionBase::default(),
            executable_list,
            ignore_error: false,
            group: "default".to_string(),
            parallel_strategy: None,
            any: false,
            percentage: None,
            must: Vec::new(),
            max_wait_ms: None,
            max_wait_time: None,
            max_wait_time_unit: TimeUnit::Milliseconds,
            thread_executor: None,
        }
    }

    fn strategy(&self) -> ParallelStrategyEnum {
        if let Some(parallel_strategy) = self.parallel_strategy {
            parallel_strategy
        } else if !self.must.is_empty() {
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

    /// 执行 WHEN 并行条件。
    ///
    /// 对应 Java: `WhenCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 返回条件类型。对应 Java: `WhenCondition#getConditionType`。
    #[must_use]
    pub fn get_condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::When
    }

    /// 返回分支失败后是否继续等待其他分支。
    ///
    /// 对应 Java: `WhenCondition#isIgnoreError`。
    #[must_use]
    pub fn is_ignore_error(&self) -> bool {
        self.ignore_error
    }

    /// 设置是否忽略并行分支错误。
    ///
    /// 对应 Java: `WhenCondition#setIgnoreError`。
    pub fn set_ignore_error(&mut self, ignore_error: bool) {
        self.ignore_error = ignore_error;
    }

    /// 返回已弃用的分组名称。对应 Java: `WhenCondition#getGroup`。
    #[must_use]
    pub fn get_group(&self) -> &str {
        &self.group
    }

    /// 设置已弃用的分组名称。
    ///
    /// 仅保留对象状态兼容，不参与 Rust 并行任务合并。
    /// 对应 Java: `WhenCondition#setGroup`。
    pub fn set_group(&mut self, group: impl Into<String>) {
        self.group = group.into();
    }

    /// 返回当前并行完成策略。
    ///
    /// 未显式设置时根据 MUST、ANY、PERCENTAGE 字段推导，默认 ALL。
    /// 对应 Java: `WhenCondition#getParallelStrategy`。
    #[must_use]
    pub fn get_parallel_strategy(&self) -> ParallelStrategyEnum {
        self.strategy()
    }

    /// 设置并行完成策略。对应 Java: `WhenCondition#setParallelStrategy`。
    pub fn set_parallel_strategy(&mut self, parallel_strategy: ParallelStrategyEnum) {
        self.parallel_strategy = Some(parallel_strategy);
    }

    /// 返回 MUST 指定的分支 ID 集合。
    ///
    /// 对应 Java: `WhenCondition#getSpecifyIdSet`。
    #[must_use]
    pub fn get_specify_id_set(&self) -> HashSet<String> {
        self.must.iter().cloned().collect()
    }

    /// 设置 MUST 指定的分支 ID 集合。
    ///
    /// 为保证运行和测试可复现，内部按字典序保存。
    /// 对应 Java: `WhenCondition#setSpecifyIdSet`。
    pub fn set_specify_id_set(&mut self, specify_id_set: HashSet<String>) {
        self.must = specify_id_set.into_iter().collect();
        self.must.sort();
    }

    /// 返回 Condition 级线程池构建器名称。
    ///
    /// 对应 Java: `WhenCondition#getThreadExecutorClass`。
    #[must_use]
    pub fn get_thread_executor_class(&self) -> Option<&str> {
        self.thread_executor.as_deref()
    }

    /// 设置并预创建 Condition 级线程池。
    ///
    /// Java 为避免运行期并发创建问题，在 setter 内立即构建线程池；Rust 保持同一
    /// 时机，构建失败通过 `Result` 返回。对应 Java:
    /// `WhenCondition#setThreadExecutorClass`。
    pub fn set_thread_executor_class(
        &mut self,
        thread_executor_class: impl Into<String>,
    ) -> LFResult<()> {
        let thread_executor_class = thread_executor_class.into();
        ExecutorHelper::load_instance().build_when_executor_for(Some(&thread_executor_class))?;
        self.thread_executor = Some(thread_executor_class);
        Ok(())
    }

    /// 返回最大等待数值。
    ///
    /// 直接由 EL 毫秒值构建时返回毫秒值。对应 Java:
    /// `WhenCondition#getMaxWaitTime`。
    #[must_use]
    pub fn get_max_wait_time(&self) -> Option<u64> {
        self.max_wait_time.or(self.max_wait_ms)
    }

    /// 设置最大等待数值，并按当前单位刷新运行时毫秒值。
    ///
    /// 对应 Java: `WhenCondition#setMaxWaitTime`。
    pub fn set_max_wait_time(&mut self, max_wait_time: u64) {
        self.max_wait_time = Some(max_wait_time);
        self.max_wait_ms = Some(
            self.max_wait_time_unit
                .to_duration(max_wait_time)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        );
    }

    /// 返回最大等待单位。
    ///
    /// 对应 Java: `WhenCondition#getMaxWaitTimeUnit`。
    #[must_use]
    pub fn get_max_wait_time_unit(&self) -> TimeUnit {
        self.max_wait_time_unit
    }

    /// 设置最大等待单位，并在已有等待数值时刷新运行时毫秒值。
    ///
    /// 对应 Java: `WhenCondition#setMaxWaitTimeUnit`。
    pub fn set_max_wait_time_unit(&mut self, max_wait_time_unit: TimeUnit) {
        self.max_wait_time_unit = max_wait_time_unit;
        if let Some(max_wait_time) = self.max_wait_time {
            self.set_max_wait_time(max_wait_time);
        }
    }

    /// 返回 PERCENTAGE 策略阈值。对应 Java: `WhenCondition#getPercentage`。
    #[must_use]
    pub fn get_percentage(&self) -> Option<f64> {
        self.percentage
    }

    /// 设置 PERCENTAGE 策略阈值。
    ///
    /// 范围校验由 `PercentageOperator` 和并行策略执行器共同承担。
    /// 对应 Java: `WhenCondition#setPercentage`。
    pub fn set_percentage(&mut self, percentage: f64) {
        self.percentage = Some(percentage);
    }

    /// 返回条件类型的 Rust 惯用别名。
    pub fn condition_type(&self) -> ConditionTypeEnum {
        self.get_condition_type()
    }
}

#[async_trait]
impl Executable for WhenCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        super::execute_condition_with_lifecycle(self, ctx, frame, async {
            // 对应 Java WhenCondition#executeAsyncCondition 的 stream 过滤：
            // 1. 过滤掉 PreCondition / FinallyCondition（EL Chain 处理时已提出）
            // 2. 过滤 isAccess 为 false 的分支（不过滤的话 any 模式下它会最快返回）
            let mut items: Vec<Arc<dyn Executable>> =
                Vec::with_capacity(self.executable_list.len());
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
            let executor = ParallelStrategyHelper::load_instance()
                .build_parallel_executor(Some(self.strategy()));
            let timeout_item_ids = items
                .iter()
                .map(|executable| executable.id().to_string())
                .collect::<Vec<_>>();
            // 并行分支共享同一 slot（Java 同一 slotIndex）
            let fut = executor.execute(items, &opts, ctx.clone(), frame.clone());
            match self.max_wait_ms {
                Some(ms) => match tokio::time::timeout(Duration::from_millis(ms), fut).await {
                    Ok(r) => r,
                    Err(_) => {
                        // Java 会把超时的 WHEN 执行项写入 Slot.timeoutItemList。Rust 的
                        // 整体 timeout 会取消仍在运行的 Future，因此把本次参与并行的
                        // 分支标识全部登记为未在期限内完成的候选项。
                        for executor_item in timeout_item_ids {
                            ctx.inner.add_timeout_item(executor_item);
                        }
                        Err(LiteflowError::WhenTimeout("when timeout".to_string()))
                    }
                },
                None => fut.await,
            }
        })
        .await
    }

    fn collect_node_ids(&self) -> Vec<String> {
        Condition::get_all_node_in_condition(self)
    }

    fn id(&self) -> &str {
        "WHEN"
    }
}

impl Condition for WhenCondition {
    fn condition_base(&self) -> &ConditionBase {
        &self.base
    }

    fn condition_base_mut(&mut self) -> &mut ConditionBase {
        &mut self.base
    }

    fn typed_executable_group(&self) -> HashMap<String, Vec<Arc<dyn Executable>>> {
        HashMap::from([("DEFAULT_KEY".to_string(), self.executable_list.clone())])
    }

    fn replace_typed_executable_group(
        &mut self,
        group_key: &str,
        executable_list: Vec<Arc<dyn Executable>>,
    ) -> bool {
        if group_key == "DEFAULT_KEY" {
            self.executable_list = executable_list;
            true
        } else {
            false
        }
    }

    fn condition_type(&self) -> ConditionTypeEnum {
        WhenCondition::condition_type(self)
    }
}
