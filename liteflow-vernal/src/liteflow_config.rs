//! 对应 Java: `LiteflowProperty`、`LiteflowMonitorProperty` 与 `LiteflowConfig`。

use serde::{Deserialize, Serialize};

use crate::{LiteflowParseMode, LiteflowRuleFormat};
use liteflow_core::{LiteFlowDefaultGlobalExecutorBuilder, LiteFlowDefaultMainExecutorBuilder};

/// LiteFlow Vernal 类型安全配置。
///
/// Jackson/Spring `@ConfigurationProperties` 映射为 serde；规则可来自文件或
/// 内联文本，二者同时存在时拒绝启动，避免环境差异决定优先级。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowConfig {
    /// 是否启用 LiteFlow 自动装配。
    pub enable: bool,
    /// 规则文件路径。
    pub rule_source: Option<String>,
    /// 内联规则文本，适合测试和程序化模块。
    pub inline_rule: Option<String>,
    /// 规则格式。
    pub rule_format: LiteflowRuleFormat,
    /// 规则解析模式。
    pub parse_mode: LiteflowParseMode,
    /// 是否打印执行日志。
    pub print_execution_log: bool,
    /// 是否启用监控日志。
    pub monitor_enable_log: bool,
    /// WHEN 与异步循环使用的全局执行器构建器名称。
    pub global_thread_pool_executor_class: String,
    /// 全局执行器最大并发数。
    pub global_thread_pool_size: usize,
    /// 全局执行器等待队列容量。
    pub global_thread_pool_queue_size: usize,
    /// `FlowExecutor#execute2Future` 使用的主执行器构建器名称。
    pub main_executor_class: String,
    /// 主执行器基础 worker 数。
    pub main_executor_works: usize,
    /// 是否为每个 WHEN 创建隔离执行器。
    pub when_thread_pool_isolate: bool,
    /// 是否用 Tokio 轻量任务承担 Java virtual thread 角色。
    pub enable_virtual_thread: bool,
}

impl Default for LiteflowConfig {
    fn default() -> Self {
        Self {
            enable: true,
            rule_source: None,
            inline_rule: None,
            rule_format: LiteflowRuleFormat::Json,
            parse_mode: LiteflowParseMode::ParseAllOnStart,
            print_execution_log: true,
            monitor_enable_log: false,
            global_thread_pool_executor_class: LiteFlowDefaultGlobalExecutorBuilder::CLASS_NAME
                .to_string(),
            global_thread_pool_size: 64,
            global_thread_pool_queue_size: 512,
            main_executor_class: LiteFlowDefaultMainExecutorBuilder::CLASS_NAME.to_string(),
            main_executor_works: 64,
            when_thread_pool_isolate: false,
            enable_virtual_thread: true,
        }
    }
}

impl LiteflowConfig {
    /// 创建默认启用的配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 配置内联规则。
    #[must_use]
    pub fn with_inline_rule(mut self, format: LiteflowRuleFormat, rule: impl Into<String>) -> Self {
        self.rule_format = format;
        self.inline_rule = Some(rule.into());
        self
    }

    /// 配置规则文件。
    #[must_use]
    pub fn with_rule_source(
        mut self,
        format: LiteflowRuleFormat,
        source: impl Into<String>,
    ) -> Self {
        self.rule_format = format;
        self.rule_source = Some(source.into());
        self
    }

    /// 返回全局执行器构建器名称。
    ///
    /// 对应 Java: `LiteflowConfig#getGlobalThreadPoolExecutorClass`。
    #[must_use]
    pub fn global_thread_pool_executor_class(&self) -> &str {
        &self.global_thread_pool_executor_class
    }

    /// 设置全局执行器构建器名称。
    ///
    /// 对应 Java: `LiteflowConfig#setGlobalThreadPoolExecutorClass`。
    pub fn set_global_thread_pool_executor_class(
        &mut self,
        global_thread_pool_executor_class: impl Into<String>,
    ) {
        self.global_thread_pool_executor_class = global_thread_pool_executor_class.into();
    }

    /// 返回全局执行器最大并发数。
    ///
    /// 对应 Java: `LiteflowConfig#getGlobalThreadPoolSize`。
    #[must_use]
    pub fn global_thread_pool_size(&self) -> usize {
        self.global_thread_pool_size
    }

    /// 设置全局执行器最大并发数。
    ///
    /// 对应 Java: `LiteflowConfig#setGlobalThreadPoolSize`。
    pub fn set_global_thread_pool_size(&mut self, global_thread_pool_size: usize) {
        self.global_thread_pool_size = global_thread_pool_size;
    }

    /// 返回全局执行器等待队列容量。
    ///
    /// 对应 Java: `LiteflowConfig#getGlobalThreadPoolQueueSize`。
    #[must_use]
    pub fn global_thread_pool_queue_size(&self) -> usize {
        self.global_thread_pool_queue_size
    }

    /// 设置全局执行器等待队列容量。
    ///
    /// 对应 Java: `LiteflowConfig#setGlobalThreadPoolQueueSize`。
    pub fn set_global_thread_pool_queue_size(&mut self, global_thread_pool_queue_size: usize) {
        self.global_thread_pool_queue_size = global_thread_pool_queue_size;
    }

    /// 返回主执行器构建器名称。
    ///
    /// 对应 Java: `LiteflowConfig#getMainExecutorClass`。
    #[must_use]
    pub fn main_executor_class(&self) -> &str {
        &self.main_executor_class
    }

    /// 设置主执行器构建器名称。
    ///
    /// 对应 Java: `LiteflowConfig#setMainExecutorClass`。
    pub fn set_main_executor_class(&mut self, main_executor_class: impl Into<String>) {
        self.main_executor_class = main_executor_class.into();
    }

    /// 返回主执行器基础 worker 数。
    ///
    /// 对应 Java: `LiteflowConfig#getMainExecutorWorks`。
    #[must_use]
    pub fn main_executor_works(&self) -> usize {
        self.main_executor_works
    }

    /// 设置主执行器基础 worker 数。
    ///
    /// 对应 Java: `LiteflowConfig#setMainExecutorWorks`。
    pub fn set_main_executor_works(&mut self, main_executor_works: usize) {
        self.main_executor_works = main_executor_works;
    }

    /// 返回是否隔离每个 WHEN 的执行器。
    ///
    /// 对应 Java: `LiteflowConfig#getWhenThreadPoolIsolate`。
    #[must_use]
    pub fn is_when_thread_pool_isolate(&self) -> bool {
        self.when_thread_pool_isolate
    }

    /// 设置是否隔离每个 WHEN 的执行器。
    ///
    /// 对应 Java: `LiteflowConfig#setWhenThreadPoolIsolate`。
    pub fn set_when_thread_pool_isolate(&mut self, when_thread_pool_isolate: bool) {
        self.when_thread_pool_isolate = when_thread_pool_isolate;
    }

    /// 返回是否使用 Tokio 轻量任务映射 Java virtual thread。
    ///
    /// 对应 Java: `LiteflowConfig#getEnableVirtualThread`。
    #[must_use]
    pub fn is_enable_virtual_thread(&self) -> bool {
        self.enable_virtual_thread
    }

    /// 设置是否使用 Tokio 轻量任务映射 Java virtual thread。
    ///
    /// 对应 Java: `LiteflowConfig#setEnableVirtualThread`。
    pub fn set_enable_virtual_thread(&mut self, enable_virtual_thread: bool) {
        self.enable_virtual_thread = enable_virtual_thread;
    }
}
