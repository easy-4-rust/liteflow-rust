//! Vernal 容器的 LiteFlow 装配配置。

use std::collections::HashMap;

use liteflow_core::property::TimeUnit;
use liteflow_core::property::agent::AgentConfig;
use serde::{Deserialize, Serialize};

use crate::{LiteflowParseMode, LiteflowRuleFormat};
use liteflow_core::{LiteFlowDefaultGlobalExecutorBuilder, LiteFlowDefaultMainExecutorBuilder};

/// LiteFlow Vernal 类型安全装配配置。
///
/// 核心引擎字段在装配时转换为
/// `com.yomahub.liteflow.property.LiteflowConfig` 对应的核心 Rust 对象；Vernal
/// 额外保留内联规则、规则格式等容器启动参数。本类型是 Rust 宿主的统一配置
/// DTO，不重复对应某个 Java 对象；Spring Boot 属性由
/// `LiteflowPropertyAutoConfiguration` 合并后生成它。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowVernalConfig {
    /// 是否启用 LiteFlow 自动装配。
    pub enable: bool,
    /// 规则文件路径。
    pub rule_source: Option<String>,
    /// 规则资源扩展数据字符串。
    pub rule_source_ext_data: Option<String>,
    /// 规则资源扩展数据映射。
    pub rule_source_ext_data_map: HashMap<String, String>,
    /// Slot 池初始容量。
    pub slot_size: usize,
    /// 内联规则文本，适合测试和程序化模块。
    pub inline_rule: Option<String>,
    /// 规则格式。
    pub rule_format: LiteflowRuleFormat,
    /// 规则解析模式。
    pub parse_mode: LiteflowParseMode,
    /// 废弃的 WHEN 秒级等待配置。
    pub when_max_wait_seconds: Option<u64>,
    /// WHEN 最大等待值。
    pub when_max_wait_time: u64,
    /// WHEN 最大等待值单位。
    pub when_max_wait_time_unit: TimeUnit,
    /// 是否支持同一规则源中的多种配置类型。
    pub support_multiple_type: bool,
    /// 废弃的全局重试次数。
    pub retry_count: i32,
    /// 是否启用首次执行模式下的 Chain 编译缓存淘汰。
    pub chain_cache_enabled: bool,
    /// Chain 编译缓存容量。
    pub chain_cache_capacity: usize,
    /// 是否打印执行日志。
    pub print_execution_log: bool,
    /// 节点执行器稳定注册名。
    pub node_executor_class: String,
    /// Request ID 生成器稳定注册名。
    pub request_id_generator_class: String,
    /// 是否监听规则或脚本文件变更。
    pub enable_monitor_file: bool,
    /// 是否启用组件降级。
    pub fallback_cmp_enable: bool,
    /// 是否使用快速加载模式。
    pub fast_load: bool,
    /// 是否在规则构建时校验节点存在。
    pub check_node_exists: bool,
    /// 脚本引擎特殊设置。
    pub script_setting: HashMap<String, String>,
    /// 是否在组件扫描器创建时打印 LiteFlow 标识。
    pub print_banner: bool,
    /// 是否启用监控日志。
    #[serde(rename = "enableLog", alias = "monitorEnableLog")]
    pub monitor_enable_log: bool,
    /// 每个组件保留的监控样本数量上限。
    pub queue_limit: usize,
    /// 监控任务首次输出前的延迟，单位毫秒。
    pub delay: u64,
    /// 监控任务的固定输出周期，单位毫秒。
    pub period: u64,
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
    /// 是否启用节点实例 ID。
    pub enable_node_instance_id: bool,
    /// Agent 嵌套配置。
    pub agent: Option<AgentConfig>,
}

impl Default for LiteflowVernalConfig {
    fn default() -> Self {
        Self {
            enable: true,
            rule_source: None,
            rule_source_ext_data: None,
            rule_source_ext_data_map: HashMap::new(),
            slot_size: 1024,
            inline_rule: None,
            rule_format: LiteflowRuleFormat::Json,
            parse_mode: LiteflowParseMode::ParseAllOnStart,
            when_max_wait_seconds: None,
            when_max_wait_time: 15_000,
            when_max_wait_time_unit: TimeUnit::Milliseconds,
            support_multiple_type: false,
            retry_count: 0,
            chain_cache_enabled: false,
            chain_cache_capacity: 10_000,
            print_execution_log: true,
            node_executor_class: "com.yomahub.liteflow.flow.executor.DefaultNodeExecutor"
                .to_string(),
            request_id_generator_class: "com.yomahub.liteflow.flow.id.DefaultRequestIdGenerator"
                .to_string(),
            enable_monitor_file: false,
            fallback_cmp_enable: false,
            fast_load: false,
            check_node_exists: true,
            script_setting: HashMap::new(),
            print_banner: true,
            monitor_enable_log: false,
            queue_limit: 200,
            delay: 300_000,
            period: 300_000,
            global_thread_pool_executor_class: LiteFlowDefaultGlobalExecutorBuilder::CLASS_NAME
                .to_string(),
            global_thread_pool_size: 64,
            global_thread_pool_queue_size: 512,
            main_executor_class: LiteFlowDefaultMainExecutorBuilder::CLASS_NAME.to_string(),
            main_executor_works: 64,
            when_thread_pool_isolate: false,
            enable_virtual_thread: true,
            enable_node_instance_id: false,
            agent: None,
        }
    }
}

impl LiteflowVernalConfig {
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

    /// 返回是否启用 Chain 编译缓存。
    ///
    /// 对应 Java: `LiteflowConfig#getChainCacheEnabled`。
    #[must_use]
    pub fn is_chain_cache_enabled(&self) -> bool {
        self.chain_cache_enabled
    }

    /// 设置是否启用 Chain 编译缓存。
    ///
    /// 对应 Java: `LiteflowConfig#setChainCacheEnabled`。
    pub fn set_chain_cache_enabled(&mut self, chain_cache_enabled: bool) {
        self.chain_cache_enabled = chain_cache_enabled;
    }

    /// 返回 Chain 编译缓存容量。
    ///
    /// 对应 Java: `LiteflowConfig#getChainCacheCapacity`。
    #[must_use]
    pub fn chain_cache_capacity(&self) -> usize {
        self.chain_cache_capacity
    }

    /// 设置 Chain 编译缓存容量。
    ///
    /// 对应 Java: `LiteflowConfig#setChainCacheCapacity`。
    pub fn set_chain_cache_capacity(&mut self, chain_cache_capacity: usize) {
        self.chain_cache_capacity = chain_cache_capacity;
    }

    /// 返回是否启用监控日志。
    ///
    /// 对应 Java: `LiteflowConfig#getEnableLog`。
    #[must_use]
    pub fn is_enable_log(&self) -> bool {
        self.monitor_enable_log
    }

    /// 设置是否启用监控日志。
    ///
    /// 对应 Java: `LiteflowConfig#setEnableLog`。
    pub fn set_enable_log(&mut self, enable_log: bool) {
        self.monitor_enable_log = enable_log;
    }

    /// 返回每个组件保留的监控样本数量上限。
    ///
    /// 对应 Java: `LiteflowConfig#getQueueLimit`。
    #[must_use]
    pub fn queue_limit(&self) -> usize {
        self.queue_limit
    }

    /// 设置每个组件保留的监控样本数量上限。
    ///
    /// 对应 Java: `LiteflowConfig#setQueueLimit`。
    pub fn set_queue_limit(&mut self, queue_limit: usize) {
        self.queue_limit = queue_limit;
    }

    /// 返回监控任务首次输出前的延迟毫秒数。
    ///
    /// 对应 Java: `LiteflowConfig#getDelay`。
    #[must_use]
    pub fn delay(&self) -> u64 {
        self.delay
    }

    /// 设置监控任务首次输出前的延迟毫秒数。
    ///
    /// 对应 Java: `LiteflowConfig#setDelay`。
    pub fn set_delay(&mut self, delay: u64) {
        self.delay = delay;
    }

    /// 返回监控任务的固定输出周期毫秒数。
    ///
    /// 对应 Java: `LiteflowConfig#getPeriod`。
    #[must_use]
    pub fn period(&self) -> u64 {
        self.period
    }

    /// 设置监控任务的固定输出周期毫秒数。
    ///
    /// 对应 Java: `LiteflowConfig#setPeriod`。
    pub fn set_period(&mut self, period: u64) {
        self.period = period;
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

    /// 将 Vernal 装配字段转换为核心 `LiteflowConfig`。
    ///
    /// 只有核心 Java 对象拥有的字段会进入返回值；`inline_rule`、`rule_format`
    /// 等 Vernal 启动参数继续由 `LiteflowRuntime` 消费。
    #[must_use]
    pub fn to_core_config(&self) -> liteflow_core::LiteflowConfig {
        let mut core = liteflow_core::LiteflowConfig::default();
        core.set_enabled(self.enable);
        core.set_rule_source(self.rule_source.clone());
        core.set_rule_source_ext_data(self.rule_source_ext_data.clone());
        core.set_rule_source_ext_data_map(self.rule_source_ext_data_map.clone());
        core.set_slot_size(self.slot_size);
        #[allow(deprecated)]
        core.set_when_max_wait_seconds(self.when_max_wait_seconds);
        core.set_when_max_wait_time(self.when_max_wait_time);
        core.set_when_max_wait_time_unit(self.when_max_wait_time_unit);
        core.set_support_multiple_type(self.support_multiple_type);
        #[allow(deprecated)]
        core.set_retry_count(self.retry_count);
        core.set_parse_mode(match self.parse_mode {
            LiteflowParseMode::ParseAllOnStart => liteflow_core::ParseModeEnum::ParseAllOnStart,
            LiteflowParseMode::ParseAllOnFirstExec => {
                liteflow_core::ParseModeEnum::ParseAllOnFirstExec
            }
            LiteflowParseMode::ParseOneOnFirstExec => {
                liteflow_core::ParseModeEnum::ParseOneOnFirstExec
            }
        });
        core.set_chain_cache_enabled(self.chain_cache_enabled);
        core.set_chain_cache_capacity(self.chain_cache_capacity);
        core.set_print_execution_log(self.print_execution_log);
        core.set_print_banner(self.print_banner);
        core.set_node_executor_class(self.node_executor_class.clone());
        core.set_request_id_generator_class(self.request_id_generator_class.clone());
        core.set_enable_monitor_file(self.enable_monitor_file);
        core.set_fallback_cmp_enable(self.fallback_cmp_enable);
        core.set_fast_load(self.fast_load);
        core.set_script_setting(self.script_setting.clone());
        core.set_enable_log(self.monitor_enable_log);
        core.set_queue_limit(self.queue_limit);
        core.set_delay(self.delay);
        core.set_period(self.period);
        core.set_global_thread_pool_executor_class(self.global_thread_pool_executor_class.clone());
        core.set_global_thread_pool_size(self.global_thread_pool_size);
        core.set_global_thread_pool_queue_size(self.global_thread_pool_queue_size);
        core.set_main_executor_class(self.main_executor_class.clone());
        core.set_main_executor_works(self.main_executor_works);
        core.set_when_thread_pool_isolate(self.when_thread_pool_isolate);
        core.set_enable_virtual_thread(self.enable_virtual_thread);
        core.set_enable_node_instance_id(self.enable_node_instance_id);
        core.set_agent(self.agent.clone());
        core
    }
}
