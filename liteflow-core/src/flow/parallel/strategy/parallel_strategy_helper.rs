//! WHEN 并行策略执行器的创建与缓存中心。
//!
//! Java 通过 ContextAware 反射创建策略实例；Rust 使用枚举做穷尽映射，并把无状态
//! 执行器缓存为 `Arc<dyn ParallelStrategyExecutor>`，避免每次 WHEN 执行重复分配。
//!
//! 对应 Java:
//! `com.yomahub.liteflow.flow.parallel.strategy.ParallelStrategyHelper`。

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::enums::ParallelStrategyEnum;

use super::all_of_parallel_executor::AllOfParallelExecutor;
use super::any_of_parallel_executor::AnyOfParallelExecutor;
use super::parallel_strategy_executor::ParallelStrategyExecutor;
use super::percentage_of_parallel_executor::PercentageOfParallelExecutor;
use super::specify_parallel_executor::SpecifyParallelExecutor;

/// WHEN 并行策略执行器辅助对象。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.parallel.strategy.ParallelStrategyHelper`。
pub struct ParallelStrategyHelper {
    strategy_executor_map: RwLock<HashMap<ParallelStrategyEnum, Arc<dyn ParallelStrategyExecutor>>>,
}

impl ParallelStrategyHelper {
    /// 返回进程级策略辅助对象。
    ///
    /// 使用 `OnceLock` 对应 Java 静态内部类 Holder 的线程安全懒加载单例。
    /// 对应 Java: `ParallelStrategyHelper#loadInstance`。
    pub fn load_instance() -> &'static Self {
        static INSTANCE: OnceLock<ParallelStrategyHelper> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            strategy_executor_map: RwLock::new(HashMap::new()),
        })
    }

    /// 根据策略类型返回缓存的并行执行器。
    ///
    /// `None` 与 Java 传入 null 一致，回退到 ALL 策略。返回值可跨任务安全共享。
    /// 对应 Java: `ParallelStrategyHelper#buildParallelExecutor`。
    pub fn build_parallel_executor(
        &self,
        parallel_strategy_enum: Option<ParallelStrategyEnum>,
    ) -> Arc<dyn ParallelStrategyExecutor> {
        let strategy = parallel_strategy_enum.unwrap_or(ParallelStrategyEnum::All);
        if let Some(executor) = self
            .strategy_executor_map
            .read()
            .expect("并行策略缓存读锁中毒")
            .get(&strategy)
            .cloned()
        {
            return executor;
        }

        let mut executors = self
            .strategy_executor_map
            .write()
            .expect("并行策略缓存写锁中毒");
        executors
            .entry(strategy)
            .or_insert_with(|| Self::create_executor(strategy))
            .clone()
    }

    /// 返回默认的 ALL 并行执行器。
    ///
    /// 对应 Java: `ParallelStrategyHelper#buildParallelExecutor()`。
    pub fn build_default_parallel_executor(&self) -> Arc<dyn ParallelStrategyExecutor> {
        self.build_parallel_executor(None)
    }

    /// 清空策略执行器缓存。
    ///
    /// 后续调用会重新创建对应执行器。对应 Java:
    /// `ParallelStrategyHelper#clearStrategyExecutorMap`。
    pub fn clear_strategy_executor_map(&self) {
        self.strategy_executor_map
            .write()
            .expect("并行策略缓存写锁中毒")
            .clear();
    }

    fn create_executor(
        parallel_strategy_enum: ParallelStrategyEnum,
    ) -> Arc<dyn ParallelStrategyExecutor> {
        match parallel_strategy_enum {
            ParallelStrategyEnum::All => Arc::new(AllOfParallelExecutor),
            ParallelStrategyEnum::Any => Arc::new(AnyOfParallelExecutor),
            ParallelStrategyEnum::Percentage => Arc::new(PercentageOfParallelExecutor),
            ParallelStrategyEnum::Specify => Arc::new(SpecifyParallelExecutor),
        }
    }
}
