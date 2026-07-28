use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vernal_context::{Lifecycle, LifecycleFuture};

use crate::{LiteflowParseMode, LiteflowRuntime, LiteflowVernalError};

/// 在 Spring Boot 4/Vernal 容器启动阶段提前初始化 LiteFlow 执行器。
///
/// Boot 4 版本仍以 `SmartInitializingSingleton` 避免首次业务执行承担规则初始化
/// 开销；Rust 使用 Vernal Lifecycle 保留相同时序。对应 Java:
/// `com.yomahub.liteflow.springboot4.LiteflowExecutorInit`。
pub struct LiteflowExecutorInit {
    liteflow_runtime: Arc<LiteflowRuntime>,
    initialized: AtomicBool,
}

impl LiteflowExecutorInit {
    /// 创建 Boot 4 执行器初始化对象。
    ///
    /// # 参数
    /// - `liteflow_runtime`：容器托管的真实 LiteFlow 运行时。
    #[must_use]
    pub fn new(liteflow_runtime: Arc<LiteflowRuntime>) -> Self {
        Self {
            liteflow_runtime,
            initialized: AtomicBool::new(false),
        }
    }

    /// 在全部单例创建后初始化执行器。
    ///
    /// # 返回
    /// 初始化成功返回 `Ok(())`。对应 Java:
    /// `LiteflowExecutorInit#afterSingletonsInstantiated`。
    pub fn after_singletons_instantiated(&self) -> Result<(), LiteflowVernalError> {
        if !matches!(
            self.liteflow_runtime.config().parse_mode,
            LiteflowParseMode::ParseAllOnStart | LiteflowParseMode::ParseOneOnFirstExec
        ) {
            return Ok(());
        }
        self.liteflow_runtime.initialize_executor()?;
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// 返回 Boot 4 启动初始化是否已经完成。
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

impl Lifecycle for LiteflowExecutorInit {
    fn initialize(&self) -> LifecycleFuture<'_> {
        Box::pin(async move {
            self.after_singletons_instantiated()
                .map_err(|error| Box::new(error) as vernal_core::BoxError)
        })
    }
}
