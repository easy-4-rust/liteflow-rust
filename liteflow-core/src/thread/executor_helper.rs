//! LiteFlow 执行器注册、选择、缓存与关闭中心。

use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use dashmap::DashMap;

use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};

use super::{
    ExecutorBuilder, ExecutorConditionBuilder, ExecutorService,
    LiteFlowDefaultGlobalExecutorBuilder, LiteFlowDefaultMainExecutorBuilder,
};

#[derive(Clone)]
struct ExecutorSettings {
    global_executor_class: String,
    main_executor_class: String,
    global_thread_pool_size: usize,
    global_thread_pool_queue_size: usize,
    main_executor_works: usize,
    when_thread_pool_isolate: bool,
    enable_virtual_thread: bool,
}

impl Default for ExecutorSettings {
    fn default() -> Self {
        Self {
            global_executor_class: LiteFlowDefaultGlobalExecutorBuilder::CLASS_NAME.to_string(),
            main_executor_class: LiteFlowDefaultMainExecutorBuilder::CLASS_NAME.to_string(),
            global_thread_pool_size: 64,
            global_thread_pool_queue_size: 512,
            main_executor_works: 64,
            when_thread_pool_isolate: false,
            enable_virtual_thread: true,
        }
    }
}

/// 线程执行器工具类。
///
/// Java `Class.forName + ContextAware.registerBean` 映射为显式构建器注册表；
/// `ConcurrentHashMap<String, ExecutorService>` 映射为 `DashMap`。缓存键仍由
/// 构建器名称和 Condition/Chain 稳定键组成。
///
/// 对应 Java: `com.yomahub.liteflow.thread.ExecutorHelper`。
pub struct ExecutorHelper {
    executor_service_map: DashMap<String, Arc<ExecutorService>>,
    executor_builder_map: DashMap<String, Arc<dyn ExecutorBuilder>>,
    settings: RwLock<ExecutorSettings>,
}

impl ExecutorHelper {
    fn new() -> Self {
        let helper = Self {
            executor_service_map: DashMap::new(),
            executor_builder_map: DashMap::new(),
            settings: RwLock::new(ExecutorSettings::default()),
        };
        helper.register_executor_builder(
            LiteFlowDefaultGlobalExecutorBuilder::CLASS_NAME,
            Arc::new(LiteFlowDefaultGlobalExecutorBuilder),
        );
        helper.register_executor_builder(
            LiteFlowDefaultMainExecutorBuilder::CLASS_NAME,
            Arc::new(LiteFlowDefaultMainExecutorBuilder),
        );
        helper
    }

    /// 获取进程级执行器辅助对象。
    ///
    /// 对应 Java: `ExecutorHelper#loadInstance`。
    #[must_use]
    pub fn load_instance() -> &'static Self {
        static INSTANCE: OnceLock<ExecutorHelper> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    /// 注册一个 Rust 执行器构建器。
    ///
    /// 这是 Java `Class.forName + ContextAware.registerBean` 的类型安全替代；名称可
    /// 继续使用 Java FQN，从而保持规则和配置兼容。
    pub fn register_executor_builder(
        &self,
        executor_class: impl Into<String>,
        executor_builder: Arc<dyn ExecutorBuilder>,
    ) {
        self.executor_builder_map
            .insert(executor_class.into(), executor_builder);
    }

    /// 从 Vernal 或独立运行时写入 Java `LiteflowConfig` 的线程参数。
    ///
    /// 配置变化会关闭并清空旧缓存，确保后续任务使用新的容量。
    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &self,
        global_executor_class: impl Into<String>,
        main_executor_class: impl Into<String>,
        global_thread_pool_size: usize,
        global_thread_pool_queue_size: usize,
        main_executor_works: usize,
        when_thread_pool_isolate: bool,
        enable_virtual_thread: bool,
    ) {
        *self.settings.write().expect("执行器配置写锁中毒") = ExecutorSettings {
            global_executor_class: global_executor_class.into(),
            main_executor_class: main_executor_class.into(),
            global_thread_pool_size: global_thread_pool_size.max(1),
            global_thread_pool_queue_size,
            main_executor_works: main_executor_works.max(1),
            when_thread_pool_isolate,
            enable_virtual_thread,
        };
        self.clear_executor_service_map();
    }

    /// 构建默认 WHEN 执行器。
    ///
    /// 对应 Java: `ExecutorHelper#buildWhenExecutor()`。
    pub fn build_when_executor(&self) -> LFResult<Arc<ExecutorService>> {
        self.build_when_executor_for(None)
    }

    /// 按构建器名称构建或复用 WHEN 执行器。
    ///
    /// 对应 Java: `ExecutorHelper#buildWhenExecutor(String)`。
    pub fn build_when_executor_for(
        &self,
        executor_class: Option<&str>,
    ) -> LFResult<Arc<ExecutorService>> {
        let default_executor_class = self
            .settings
            .read()
            .expect("执行器配置读锁中毒")
            .global_executor_class
            .clone();
        let executor_class = non_blank(executor_class).unwrap_or(&default_executor_class);
        self.get_executor_service(executor_class, None)
    }

    /// 按构建器名称和 Condition/Chain 稳定键构建隔离执行器。
    ///
    /// 对应 Java: `ExecutorHelper#buildWhenExecutorWithHash`。
    pub fn build_when_executor_with_hash(
        &self,
        executor_class: Option<&str>,
        hash: &str,
    ) -> LFResult<Arc<ExecutorService>> {
        let default_executor_class = self
            .settings
            .read()
            .expect("执行器配置读锁中毒")
            .global_executor_class
            .clone();
        let executor_class = non_blank(executor_class).unwrap_or(&default_executor_class);
        self.get_executor_service(executor_class, non_blank(Some(hash)))
    }

    /// 构建或复用 `FlowExecutor` 主执行器。
    ///
    /// 对应 Java: `ExecutorHelper#buildMainExecutor`。
    pub fn build_main_executor(
        &self,
        executor_class: Option<&str>,
    ) -> LFResult<Arc<ExecutorService>> {
        let default_executor_class = self
            .settings
            .read()
            .expect("执行器配置读锁中毒")
            .main_executor_class
            .clone();
        let executor_class = non_blank(executor_class).unwrap_or(&default_executor_class);
        self.get_executor_service(executor_class, None)
    }

    /// 根据 Condition > Chain > 全局优先级选择执行器。
    ///
    /// `condition_key` 与 `chain_key` 对应 Java 的对象 hash；Rust 使用稳定对象地址
    /// 和 chainId，避免依赖进程随机 Hash 实现。
    /// 对应 Java: `ExecutorHelper#buildExecutorService`。
    pub fn build_executor_service(
        &self,
        condition_executor_class: Option<&str>,
        chain_executor_class: Option<&str>,
        condition_key: &str,
        chain_key: &str,
        condition_type: ConditionTypeEnum,
    ) -> LFResult<Arc<ExecutorService>> {
        let settings = self.settings.read().expect("执行器配置读锁中毒").clone();
        let executor_condition = ExecutorConditionBuilder::build_executor_condition(
            condition_executor_class,
            chain_executor_class,
            settings.when_thread_pool_isolate,
            &settings.global_executor_class,
            condition_type,
        )?;
        if executor_condition.is_condition_level() {
            return self.get_executor_service(
                executor_condition
                    .condition_executor_class()
                    .ok_or_else(|| {
                        LiteflowError::ThreadExecutorServiceCreate(
                            "condition executor class is missing".to_string(),
                        )
                    })?,
                non_blank(Some(condition_key)),
            );
        }
        if executor_condition.is_chain_level() {
            return self.get_executor_service(
                non_blank(chain_executor_class).ok_or_else(|| {
                    LiteflowError::ThreadExecutorServiceCreate(
                        "chain executor class is missing".to_string(),
                    )
                })?,
                non_blank(Some(chain_key)),
            );
        }
        self.get_executor_service(&settings.global_executor_class, None)
    }

    /// 关闭执行器并等待活动任务结束。
    ///
    /// 对应 Java: `ExecutorHelper#shutdownAwaitTermination`。
    pub async fn shutdown_await_termination(
        &self,
        executor_service: &ExecutorService,
        timeout: Duration,
    ) -> bool {
        executor_service.shutdown();
        executor_service.await_termination(timeout).await
    }

    /// 关闭并移除全部缓存执行器。
    ///
    /// 对应 Java `clearExecutorServiceMap`，同时利用 Rust RAII 主动停止新任务。
    pub fn clear_executor_service_map(&self) {
        for executor in self.executor_service_map.iter() {
            executor.value().shutdown();
        }
        self.executor_service_map.clear();
    }

    /// 返回当前缓存执行器数量，供运行时监控与测试使用。
    #[must_use]
    pub fn executor_service_count(&self) -> usize {
        self.executor_service_map.len()
    }

    /// 返回 Tokio 轻量任务是否承担 Java virtual thread 角色。
    ///
    /// 对应 Java: `ExecutorHelper#isEnabledVirtualThreads`。
    #[must_use]
    pub fn is_enabled_virtual_threads(&self) -> bool {
        self.settings
            .read()
            .expect("执行器配置读锁中毒")
            .enable_virtual_thread
    }

    /// 返回全局最大并发数。
    #[must_use]
    pub fn global_thread_pool_size(&self) -> usize {
        self.settings
            .read()
            .expect("执行器配置读锁中毒")
            .global_thread_pool_size
    }

    /// 返回全局等待队列容量。
    #[must_use]
    pub fn global_thread_pool_queue_size(&self) -> usize {
        self.settings
            .read()
            .expect("执行器配置读锁中毒")
            .global_thread_pool_queue_size
    }

    /// 返回主执行器 worker 数。
    #[must_use]
    pub fn main_executor_works(&self) -> usize {
        self.settings
            .read()
            .expect("执行器配置读锁中毒")
            .main_executor_works
    }

    fn get_executor_service(
        &self,
        executor_class: &str,
        hash: Option<&str>,
    ) -> LFResult<Arc<ExecutorService>> {
        let key = match hash {
            Some(hash) => format!("{executor_class}_{hash}"),
            None => executor_class.to_string(),
        };
        if let Some(executor_service) = self.executor_service_map.get(&key) {
            return Ok(executor_service.clone());
        }
        let executor_builder = self
            .executor_builder_map
            .get(executor_class)
            .map(|entry| entry.clone())
            .ok_or_else(|| {
                LiteflowError::ThreadExecutorServiceCreate(format!(
                    "executor builder[{executor_class}] is not registered"
                ))
            })?;
        let executor_service = executor_builder.build_executor();
        Ok(self
            .executor_service_map
            .entry(key)
            .or_insert(executor_service)
            .clone())
    }
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
