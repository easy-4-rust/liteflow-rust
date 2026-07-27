use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ParseModeEnum;
use crate::property::TimeUnit;
use crate::property::agent::AgentConfig;

const DEFAULT_NODE_EXECUTOR: &str = "com.yomahub.liteflow.flow.executor.DefaultNodeExecutor";
const DEFAULT_REQUEST_ID_GENERATOR: &str = "com.yomahub.liteflow.flow.id.DefaultRequestIdGenerator";
const DEFAULT_MAIN_EXECUTOR: &str =
    "com.yomahub.liteflow.thread.LiteFlowDefaultMainExecutorBuilder";
const DEFAULT_GLOBAL_EXECUTOR: &str =
    "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder";

/// LiteFlow 全局配置实体。
///
/// Java 使用包装类型区分“未配置”和显式值，并在 getter 中补默认值；Rust 使用
/// `Default + #[serde(default)]` 在反序列化边界补齐相同默认值。Java
/// `property.agent.AgentConfig` 已位于同一核心 crate，因此 Agent 子配置保持强类型，
/// 由 serde 直接完成 Jackson/Spring 属性绑定语义。
///
/// 对应 Java: `com.yomahub.liteflow.property.LiteflowConfig`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowConfig {
    enable: bool,
    rule_source: Option<String>,
    rule_source_ext_data: Option<String>,
    rule_source_ext_data_map: HashMap<String, String>,
    slot_size: usize,
    when_max_wait_seconds: Option<u64>,
    when_max_wait_time: u64,
    when_max_wait_time_unit: TimeUnit,
    when_thread_pool_isolate: bool,
    enable_log: bool,
    queue_limit: usize,
    delay: u64,
    period: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    when_queue_limit: Option<usize>,
    parse_mode: ParseModeEnum,
    support_multiple_type: bool,
    retry_count: i32,
    node_executor_class: String,
    request_id_generator_class: String,
    print_banner: bool,
    main_executor_works: usize,
    main_executor_class: String,
    print_execution_log: bool,
    enable_monitor_file: bool,
    fallback_cmp_enable: bool,
    fast_load: bool,
    script_setting: HashMap<String, String>,
    enable_node_instance_id: bool,
    instance_id_generator_class: String,
    chain_cache_enabled: bool,
    chain_cache_capacity: usize,
    enable_virtual_thread: bool,
    agent: Option<AgentConfig>,
    global_thread_pool_executor_class: String,
    global_thread_pool_size: usize,
    global_thread_pool_queue_size: usize,
}

impl Default for LiteflowConfig {
    fn default() -> Self {
        Self {
            enable: true,
            rule_source: None,
            rule_source_ext_data: None,
            rule_source_ext_data_map: HashMap::new(),
            slot_size: 1024,
            when_max_wait_seconds: None,
            when_max_wait_time: 15_000,
            when_max_wait_time_unit: TimeUnit::Milliseconds,
            when_thread_pool_isolate: false,
            enable_log: false,
            queue_limit: 200,
            delay: 300_000,
            period: 300_000,
            when_queue_limit: None,
            parse_mode: ParseModeEnum::ParseAllOnStart,
            support_multiple_type: false,
            retry_count: 0,
            node_executor_class: DEFAULT_NODE_EXECUTOR.to_string(),
            request_id_generator_class: DEFAULT_REQUEST_ID_GENERATOR.to_string(),
            print_banner: true,
            main_executor_works: 64,
            main_executor_class: DEFAULT_MAIN_EXECUTOR.to_string(),
            print_execution_log: true,
            enable_monitor_file: false,
            fallback_cmp_enable: false,
            fast_load: false,
            script_setting: HashMap::new(),
            enable_node_instance_id: false,
            instance_id_generator_class: DEFAULT_REQUEST_ID_GENERATOR.to_string(),
            chain_cache_enabled: false,
            chain_cache_capacity: 10_000,
            enable_virtual_thread: true,
            agent: None,
            global_thread_pool_executor_class: DEFAULT_GLOBAL_EXECUTOR.to_string(),
            global_thread_pool_size: 64,
            global_thread_pool_queue_size: 512,
        }
    }
}

impl LiteflowConfig {
    /// 返回是否启用 LiteFlow 自动装配。
    ///
    /// 返回值与 Java 未配置时默认 `true` 的语义一致。
    /// 对应 Java: `LiteflowConfig#getEnable`。
    #[must_use]
    pub fn get_enable(&self) -> bool {
        self.enable
    }

    /// 设置是否启用 LiteFlow 自动装配。
    ///
    /// 参数 `enable` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setEnable`。
    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
    }

    /// 返回流程定义资源地址；未配置时返回 `None`。
    ///
    /// 对应 Java: `LiteflowConfig#getRuleSource`。
    #[must_use]
    pub fn get_rule_source(&self) -> Option<&str> {
        self.rule_source.as_deref()
    }

    /// 设置流程定义资源地址。
    ///
    /// 参数 `rule_source` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setRuleSource`。
    pub fn set_rule_source(&mut self, rule_source: Option<impl Into<String>>) {
        self.rule_source = rule_source.map(Into::into);
    }

    /// 返回 Slot 池的初始容量。
    ///
    /// 未配置时返回 Java 默认值 `1024`。对应 Java: `LiteflowConfig#getSlotSize`。
    #[must_use]
    pub fn get_slot_size(&self) -> usize {
        self.slot_size
    }

    /// 设置 Slot 池的初始容量。
    ///
    /// 参数 `slot_size` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setSlotSize`。
    pub fn set_slot_size(&mut self, slot_size: usize) {
        self.slot_size = slot_size;
    }

    /// 返回 WHEN 与异步循环的最大等待时间。
    ///
    /// 未配置时返回 Java 默认值 `15000`。对应 Java: `LiteflowConfig#getWhenMaxWaitTime`。
    #[must_use]
    pub fn get_when_max_wait_time(&self) -> u64 {
        self.when_max_wait_time
    }

    /// 设置 WHEN 与异步循环的最大等待时间。
    ///
    /// 参数 `when_max_wait_time` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setWhenMaxWaitTime`。
    pub fn set_when_max_wait_time(&mut self, when_max_wait_time: u64) {
        self.when_max_wait_time = when_max_wait_time;
    }

    /// 返回 WHEN 最大等待时间的单位。
    ///
    /// 未配置时返回毫秒。对应 Java: `LiteflowConfig#getWhenMaxWaitTimeUnit`。
    #[must_use]
    pub fn get_when_max_wait_time_unit(&self) -> TimeUnit {
        self.when_max_wait_time_unit
    }

    /// 设置 WHEN 最大等待时间的单位。
    ///
    /// 参数 `when_max_wait_time_unit` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setWhenMaxWaitTimeUnit`。
    pub fn set_when_max_wait_time_unit(&mut self, when_max_wait_time_unit: TimeUnit) {
        self.when_max_wait_time_unit = when_max_wait_time_unit;
    }

    /// 返回 WHEN 线程池是否按流程隔离。
    ///
    /// 对应 Java: `LiteflowConfig#getWhenThreadPoolIsolate`。
    #[must_use]
    pub fn get_when_thread_pool_isolate(&self) -> bool {
        self.when_thread_pool_isolate
    }

    /// 设置 WHEN 线程池是否按流程隔离。
    ///
    /// 参数 `when_thread_pool_isolate` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setWhenThreadPoolIsolate`。
    pub fn set_when_thread_pool_isolate(&mut self, when_thread_pool_isolate: bool) {
        self.when_thread_pool_isolate = when_thread_pool_isolate;
    }

    /// 返回是否打印监控日志。
    ///
    /// 对应 Java: `LiteflowConfig#getEnableLog`。
    #[must_use]
    pub fn get_enable_log(&self) -> bool {
        self.enable_log
    }

    /// 设置是否打印监控日志。
    ///
    /// 参数 `enable_log` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setEnableLog`。
    pub fn set_enable_log(&mut self, enable_log: bool) {
        self.enable_log = enable_log;
    }

    /// 返回每个组件保留的监控样本上限。
    ///
    /// 未配置时返回 Java 默认值 `200`。对应 Java: `LiteflowConfig#getQueueLimit`。
    #[must_use]
    pub fn get_queue_limit(&self) -> usize {
        self.queue_limit
    }

    /// 设置每个组件保留的监控样本上限。
    ///
    /// 参数 `queue_limit` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setQueueLimit`。
    pub fn set_queue_limit(&mut self, queue_limit: usize) {
        self.queue_limit = queue_limit;
    }

    /// 返回监控任务首次输出前的延迟毫秒数。
    ///
    /// 对应 Java: `LiteflowConfig#getDelay`。
    #[must_use]
    pub fn get_delay(&self) -> u64 {
        self.delay
    }

    /// 设置监控任务首次输出前的延迟毫秒数。
    ///
    /// 参数 `delay` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setDelay`。
    pub fn set_delay(&mut self, delay: u64) {
        self.delay = delay;
    }

    /// 返回监控任务的固定输出周期毫秒数。
    ///
    /// 对应 Java: `LiteflowConfig#getPeriod`。
    #[must_use]
    pub fn get_period(&self) -> u64 {
        self.period
    }

    /// 设置监控任务的固定输出周期毫秒数。
    ///
    /// 参数 `period` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setPeriod`。
    pub fn set_period(&mut self, period: u64) {
        self.period = period;
    }

    /// 返回规则解析模式。
    ///
    /// 未配置时返回启动阶段解析全部规则。对应 Java: `LiteflowConfig#getParseMode`。
    #[must_use]
    pub fn get_parse_mode(&self) -> ParseModeEnum {
        self.parse_mode
    }

    /// 设置规则解析模式。
    ///
    /// 参数 `parse_mode` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setParseMode`。
    pub fn set_parse_mode(&mut self, parse_mode: ParseModeEnum) {
        self.parse_mode = parse_mode;
    }

    /// 返回是否支持多种规则配置类型。
    ///
    /// 主流程和子流程仍不能分布在不同类型的配置文件中。
    /// 对应 Java: `LiteflowConfig#isSupportMultipleType`。
    #[must_use]
    pub fn is_support_multiple_type(&self) -> bool {
        self.support_multiple_type
    }

    /// 设置是否支持多种规则配置类型。
    ///
    /// 参数 `support_multiple_type` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setSupportMultipleType`。
    pub fn set_support_multiple_type(&mut self, support_multiple_type: bool) {
        self.support_multiple_type = support_multiple_type;
    }

    /// 返回是否打印 LiteFlow Banner。
    ///
    /// 对应 Java: `LiteflowConfig#getPrintBanner`。
    #[must_use]
    pub fn get_print_banner(&self) -> bool {
        self.print_banner
    }

    /// 设置是否打印 LiteFlow Banner。
    ///
    /// 参数 `print_banner` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setPrintBanner`。
    pub fn set_print_banner(&mut self, print_banner: bool) {
        self.print_banner = print_banner;
    }

    /// 返回 `execute2Future` 主执行器的基础 worker 数。
    ///
    /// 对应 Java: `LiteflowConfig#getMainExecutorWorks`。
    #[must_use]
    pub fn get_main_executor_works(&self) -> usize {
        self.main_executor_works
    }

    /// 设置 `execute2Future` 主执行器的基础 worker 数。
    ///
    /// 参数 `main_executor_works` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setMainExecutorWorks`。
    pub fn set_main_executor_works(&mut self, main_executor_works: usize) {
        self.main_executor_works = main_executor_works;
    }

    /// 返回是否打印流程执行日志。
    ///
    /// 对应 Java: `LiteflowConfig#getPrintExecutionLog`。
    #[must_use]
    pub fn get_print_execution_log(&self) -> bool {
        self.print_execution_log
    }

    /// 设置是否打印流程执行日志。
    ///
    /// 参数 `print_execution_log` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setPrintExecutionLog`。
    pub fn set_print_execution_log(&mut self, print_execution_log: bool) {
        self.print_execution_log = print_execution_log;
    }

    /// 返回是否监听规则文件或脚本文件变更。
    ///
    /// 对应 Java: `LiteflowConfig#getEnableMonitorFile`。
    #[must_use]
    pub fn get_enable_monitor_file(&self) -> bool {
        self.enable_monitor_file
    }

    /// 设置是否监听规则文件或脚本文件变更。
    ///
    /// 参数 `enable_monitor_file` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setEnableMonitorFile`。
    pub fn set_enable_monitor_file(&mut self, enable_monitor_file: bool) {
        self.enable_monitor_file = enable_monitor_file;
    }

    /// 返回是否启用组件降级。
    ///
    /// 对应 Java: `LiteflowConfig#getFallbackCmpEnable`。
    #[must_use]
    pub fn get_fallback_cmp_enable(&self) -> bool {
        self.fallback_cmp_enable
    }

    /// 设置是否启用组件降级。
    ///
    /// 参数 `fallback_cmp_enable` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setFallbackCmpEnable`。
    pub fn set_fallback_cmp_enable(&mut self, fallback_cmp_enable: bool) {
        self.fallback_cmp_enable = fallback_cmp_enable;
    }

    /// 返回是否使用快速规则加载模式。
    ///
    /// 快速加载意味着不使用 copy-on-write 机制。对应 Java: `LiteflowConfig#getFastLoad`。
    #[must_use]
    pub fn get_fast_load(&self) -> bool {
        self.fast_load
    }

    /// 设置是否使用快速规则加载模式。
    ///
    /// 参数 `fast_load` 对应 Java 同名参数。对应 Java: `LiteflowConfig#setFastLoad`。
    pub fn set_fast_load(&mut self, fast_load: bool) {
        self.fast_load = fast_load;
    }

    /// 返回是否启用节点实例 ID。
    ///
    /// 对应 Java: `LiteflowConfig#getEnableNodeInstanceId`。
    #[must_use]
    pub fn get_enable_node_instance_id(&self) -> bool {
        self.enable_node_instance_id
    }

    /// 设置是否启用节点实例 ID。
    ///
    /// 参数 `enable_node_instance_id` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setEnableNodeInstanceId`。
    pub fn set_enable_node_instance_id(&mut self, enable_node_instance_id: bool) {
        self.enable_node_instance_id = enable_node_instance_id;
    }

    /// 返回是否启用 Chain 缓存。
    ///
    /// 对应 Java: `LiteflowConfig#getChainCacheEnabled`。
    #[must_use]
    pub fn get_chain_cache_enabled(&self) -> bool {
        self.chain_cache_enabled
    }

    /// 设置是否启用 Chain 缓存。
    ///
    /// 参数 `chain_cache_enabled` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setChainCacheEnabled`。
    pub fn set_chain_cache_enabled(&mut self, chain_cache_enabled: bool) {
        self.chain_cache_enabled = chain_cache_enabled;
    }

    /// 返回 Chain 缓存容量。
    ///
    /// 对应 Java: `LiteflowConfig#getChainCacheCapacity`。
    #[must_use]
    pub fn get_chain_cache_capacity(&self) -> usize {
        self.chain_cache_capacity
    }

    /// 设置 Chain 缓存容量。
    ///
    /// 参数 `chain_cache_capacity` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setChainCacheCapacity`。
    pub fn set_chain_cache_capacity(&mut self, chain_cache_capacity: usize) {
        self.chain_cache_capacity = chain_cache_capacity;
    }

    /// 返回是否启用 Rust 轻量任务承担 Java 虚拟线程角色。
    ///
    /// 对应 Java: `LiteflowConfig#getEnableVirtualThread`。
    #[must_use]
    pub fn get_enable_virtual_thread(&self) -> bool {
        self.enable_virtual_thread
    }

    /// 设置是否启用 Rust 轻量任务承担 Java 虚拟线程角色。
    ///
    /// 参数 `enable_virtual_thread` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setEnableVirtualThread`。
    pub fn set_enable_virtual_thread(&mut self, enable_virtual_thread: bool) {
        self.enable_virtual_thread = enable_virtual_thread;
    }

    /// 返回 WHEN 与异步循环的全局执行器最大并发数。
    ///
    /// 对应 Java: `LiteflowConfig#getGlobalThreadPoolSize`。
    #[must_use]
    pub fn get_global_thread_pool_size(&self) -> usize {
        self.global_thread_pool_size
    }

    /// 设置 WHEN 与异步循环的全局执行器最大并发数。
    ///
    /// 参数 `global_thread_pool_size` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setGlobalThreadPoolSize`。
    pub fn set_global_thread_pool_size(&mut self, global_thread_pool_size: usize) {
        self.global_thread_pool_size = global_thread_pool_size;
    }

    /// 返回 WHEN 与异步循环的全局执行器队列容量。
    ///
    /// 对应 Java: `LiteflowConfig#getGlobalThreadPoolQueueSize`。
    #[must_use]
    pub fn get_global_thread_pool_queue_size(&self) -> usize {
        self.global_thread_pool_queue_size
    }

    /// 设置 WHEN 与异步循环的全局执行器队列容量。
    ///
    /// 参数 `global_thread_pool_queue_size` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setGlobalThreadPoolQueueSize`。
    pub fn set_global_thread_pool_queue_size(&mut self, global_thread_pool_queue_size: usize) {
        self.global_thread_pool_queue_size = global_thread_pool_queue_size;
    }

    /// 返回流程资源扩展数据。
    ///
    /// 对应 Java: `LiteflowConfig#getRuleSourceExtData`。
    #[must_use]
    pub fn get_rule_source_ext_data(&self) -> Option<&str> {
        self.rule_source_ext_data.as_deref()
    }

    /// 设置流程资源扩展数据。
    ///
    /// 参数 `rule_source_ext_data` 对应 Java 同名参数。
    /// 对应 Java: `LiteflowConfig#setRuleSourceExtData`。
    pub fn set_rule_source_ext_data(&mut self, rule_source_ext_data: Option<impl Into<String>>) {
        self.rule_source_ext_data = rule_source_ext_data.map(Into::into);
    }

    /// 返回废弃的秒级最大等待值；`0` 与未配置均返回 `None`。
    #[deprecated(note = "使用 when_max_wait_time 与 when_max_wait_time_unit")]
    #[must_use]
    pub fn get_when_max_wait_seconds(&self) -> Option<u64> {
        self.when_max_wait_seconds.filter(|value| *value > 0)
    }

    /// 设置废弃的秒级最大等待值。对应 Java: `setWhenMaxWaitSeconds`。
    #[deprecated(note = "使用 set_when_max_wait_time 与 set_when_max_wait_time_unit")]
    pub fn set_when_max_wait_seconds(&mut self, value: Option<u64>) {
        self.when_max_wait_seconds = value;
    }

    /// 返回非负重试次数。对应 Java: `getRetryCount`。
    #[deprecated]
    #[must_use]
    pub fn get_retry_count(&self) -> u32 {
        self.retry_count.max(0) as u32
    }

    /// 设置废弃的重试次数。对应 Java: `setRetryCount`。
    #[deprecated]
    pub fn set_retry_count(&mut self, value: i32) {
        self.retry_count = value;
    }

    /// 返回规则资源扩展数据映射。对应 Java: `getRuleSourceExtDataMap`。
    #[must_use]
    pub fn get_rule_source_ext_data_map(&self) -> &HashMap<String, String> {
        &self.rule_source_ext_data_map
    }

    /// 设置规则资源扩展数据映射。对应 Java: `setRuleSourceExtDataMap`。
    pub fn set_rule_source_ext_data_map(&mut self, value: HashMap<String, String>) {
        self.rule_source_ext_data_map = value;
    }

    /// 返回脚本设置映射。对应 Java: `getScriptSetting`。
    #[must_use]
    pub fn get_script_setting(&self) -> &HashMap<String, String> {
        &self.script_setting
    }

    /// 设置脚本设置映射。对应 Java: `setScriptSetting`。
    pub fn set_script_setting(&mut self, value: HashMap<String, String>) {
        self.script_setting = value;
    }

    /// 返回节点执行器类名，空白值回退到 Java 默认实现。
    #[must_use]
    pub fn get_node_executor_class(&self) -> &str {
        non_blank_or(&self.node_executor_class, DEFAULT_NODE_EXECUTOR)
    }

    /// 设置节点执行器类名。对应 Java: `setNodeExecutorClass`。
    pub fn set_node_executor_class(&mut self, value: impl Into<String>) {
        self.node_executor_class = value.into();
    }

    /// 返回 Request ID 生成器类名，空白值回退到 Java 默认实现。
    #[must_use]
    pub fn get_request_id_generator_class(&self) -> &str {
        non_blank_or(
            &self.request_id_generator_class,
            DEFAULT_REQUEST_ID_GENERATOR,
        )
    }

    /// 设置 Request ID 生成器类名。对应 Java: `setRequestIdGeneratorClass`。
    pub fn set_request_id_generator_class(&mut self, value: impl Into<String>) {
        self.request_id_generator_class = value.into();
    }

    /// 返回主执行器构建器类名，空白值回退到 Java 默认实现。
    #[must_use]
    pub fn get_main_executor_class(&self) -> &str {
        non_blank_or(&self.main_executor_class, DEFAULT_MAIN_EXECUTOR)
    }

    /// 设置主执行器构建器类名。对应 Java: `setMainExecutorClass`。
    pub fn set_main_executor_class(&mut self, value: impl Into<String>) {
        self.main_executor_class = value.into();
    }

    /// 返回实例 ID 生成器类名，空白值回退到 Java 当前默认实现。
    #[must_use]
    pub fn get_instance_id_generator_class(&self) -> &str {
        non_blank_or(
            &self.instance_id_generator_class,
            DEFAULT_REQUEST_ID_GENERATOR,
        )
    }

    /// 设置实例 ID 生成器类名。对应 Java: `setInstanceIdGeneratorClass`。
    pub fn set_instance_id_generator_class(&mut self, value: impl Into<String>) {
        self.instance_id_generator_class = value.into();
    }

    /// 返回全局执行器构建器类名，空白值回退到 Java 默认实现。
    #[must_use]
    pub fn get_global_thread_pool_executor_class(&self) -> &str {
        non_blank_or(
            &self.global_thread_pool_executor_class,
            DEFAULT_GLOBAL_EXECUTOR,
        )
    }

    /// 设置全局执行器构建器类名。对应 Java: `setGlobalThreadPoolExecutorClass`。
    pub fn set_global_thread_pool_executor_class(&mut self, value: impl Into<String>) {
        self.global_thread_pool_executor_class = value.into();
    }

    /// 返回 Agent 配置；未配置时返回 `None`。
    ///
    /// 对应 Java: `LiteflowConfig#getAgent`。
    #[must_use]
    pub fn get_agent(&self) -> Option<&AgentConfig> {
        self.agent.as_ref()
    }

    /// 设置 Agent 配置。对应 Java: `LiteflowConfig#setAgent`。
    pub fn set_agent(&mut self, value: Option<AgentConfig>) {
        self.agent = value;
    }

    /// 返回等待超时的 Rust `Duration`。
    #[must_use]
    pub fn when_max_wait_duration(&self) -> std::time::Duration {
        self.when_max_wait_time_unit
            .to_duration(self.when_max_wait_time)
    }

    /// 返回是否启用 LiteFlow；是 `get_enable` 的 Rust 兼容别名。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.get_enable()
    }

    /// 设置是否启用 LiteFlow；是 `set_enable` 的 Rust 兼容别名。
    pub fn set_enabled(&mut self, enable: bool) {
        self.set_enable(enable);
    }

    /// 返回规则资源地址；是 `get_rule_source` 的 Rust 兼容别名。
    #[must_use]
    pub fn rule_source(&self) -> Option<&str> {
        self.get_rule_source()
    }

    /// 返回 Slot 初始容量；是 `get_slot_size` 的 Rust 兼容别名。
    #[must_use]
    pub fn slot_size(&self) -> usize {
        self.get_slot_size()
    }

    /// 返回 WHEN 最大等待时间；是 `get_when_max_wait_time` 的 Rust 兼容别名。
    #[must_use]
    pub fn when_max_wait_time(&self) -> u64 {
        self.get_when_max_wait_time()
    }

    /// 返回 WHEN 最大等待单位；是 `get_when_max_wait_time_unit` 的 Rust 兼容别名。
    #[must_use]
    pub fn when_max_wait_time_unit(&self) -> TimeUnit {
        self.get_when_max_wait_time_unit()
    }

    /// 返回 WHEN 线程池是否隔离；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_when_thread_pool_isolate(&self) -> bool {
        self.get_when_thread_pool_isolate()
    }

    /// 返回是否启用监控日志；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_enable_log(&self) -> bool {
        self.get_enable_log()
    }

    /// 返回监控样本上限；是 `get_queue_limit` 的 Rust 兼容别名。
    #[must_use]
    pub fn queue_limit(&self) -> usize {
        self.get_queue_limit()
    }

    /// 返回监控延迟；是 `get_delay` 的 Rust 兼容别名。
    #[must_use]
    pub fn delay(&self) -> u64 {
        self.get_delay()
    }

    /// 返回监控周期；是 `get_period` 的 Rust 兼容别名。
    #[must_use]
    pub fn period(&self) -> u64 {
        self.get_period()
    }

    /// 返回解析模式；是 `get_parse_mode` 的 Rust 兼容别名。
    #[must_use]
    pub fn parse_mode(&self) -> ParseModeEnum {
        self.get_parse_mode()
    }

    /// 返回是否打印 Banner；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_print_banner(&self) -> bool {
        self.get_print_banner()
    }

    /// 返回主执行器 worker 数；是 `get_main_executor_works` 的 Rust 兼容别名。
    #[must_use]
    pub fn main_executor_works(&self) -> usize {
        self.get_main_executor_works()
    }

    /// 返回是否打印执行日志；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_print_execution_log(&self) -> bool {
        self.get_print_execution_log()
    }

    /// 返回是否监听规则文件；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_enable_monitor_file(&self) -> bool {
        self.get_enable_monitor_file()
    }

    /// 返回是否启用组件降级；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_fallback_cmp_enabled(&self) -> bool {
        self.get_fallback_cmp_enable()
    }

    /// 设置是否启用组件降级；是 Java 命名方法的 Rust 兼容别名。
    pub fn set_fallback_cmp_enabled(&mut self, fallback_cmp_enable: bool) {
        self.set_fallback_cmp_enable(fallback_cmp_enable);
    }

    /// 返回是否快速加载规则；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_fast_load(&self) -> bool {
        self.get_fast_load()
    }

    /// 返回流程资源扩展数据；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn rule_source_ext_data(&self) -> Option<&str> {
        self.get_rule_source_ext_data()
    }

    /// 返回流程资源扩展数据映射；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn rule_source_ext_data_map(&self) -> &HashMap<String, String> {
        self.get_rule_source_ext_data_map()
    }

    /// 返回脚本设置；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn script_setting(&self) -> &HashMap<String, String> {
        self.get_script_setting()
    }

    /// 返回节点执行器类名；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn node_executor_class(&self) -> &str {
        self.get_node_executor_class()
    }

    /// 返回 Request ID 生成器类名；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn request_id_generator_class(&self) -> &str {
        self.get_request_id_generator_class()
    }

    /// 返回主执行器类名；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn main_executor_class(&self) -> &str {
        self.get_main_executor_class()
    }

    /// 返回实例 ID 生成器类名；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn instance_id_generator_class(&self) -> &str {
        self.get_instance_id_generator_class()
    }

    /// 返回是否启用节点实例 ID；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_enable_node_instance_id(&self) -> bool {
        self.get_enable_node_instance_id()
    }

    /// 返回是否启用 Chain 缓存；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_chain_cache_enabled(&self) -> bool {
        self.get_chain_cache_enabled()
    }

    /// 返回 Chain 缓存容量；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn chain_cache_capacity(&self) -> usize {
        self.get_chain_cache_capacity()
    }

    /// 返回是否启用轻量任务；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn is_enable_virtual_thread(&self) -> bool {
        self.get_enable_virtual_thread()
    }

    /// 返回全局执行器最大并发数；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn global_thread_pool_size(&self) -> usize {
        self.get_global_thread_pool_size()
    }

    /// 返回全局执行器队列容量；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn global_thread_pool_queue_size(&self) -> usize {
        self.get_global_thread_pool_queue_size()
    }

    /// 返回全局执行器类名；是 Java 命名方法的 Rust 兼容别名。
    #[must_use]
    pub fn global_thread_pool_executor_class(&self) -> &str {
        self.get_global_thread_pool_executor_class()
    }

    /// 返回 Agent 配置；是 `get_agent` 的 Rust 兼容别名。
    #[must_use]
    pub fn agent(&self) -> Option<&AgentConfig> {
        self.get_agent()
    }
}

fn non_blank_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::LiteflowConfig;
    use crate::ParseModeEnum;
    use crate::property::TimeUnit;
    use crate::property::agent::AgentConfig;

    #[test]
    fn java_defaults_are_preserved() {
        let config = LiteflowConfig::default();
        assert!(config.get_enable());
        assert_eq!(config.get_slot_size(), 1024);
        assert_eq!(config.get_when_max_wait_time(), 15_000);
        assert_eq!(config.get_when_max_wait_time_unit(), TimeUnit::Milliseconds);
        assert_eq!(config.get_parse_mode(), ParseModeEnum::ParseAllOnStart);
        assert_eq!(
            config.get_node_executor_class(),
            super::DEFAULT_NODE_EXECUTOR
        );
        assert_eq!(config.get_chain_cache_capacity(), 10_000);
        assert!(config.get_enable_virtual_thread());
    }

    #[test]
    fn serde_defaults_and_typed_agent_boundary_work() {
        let config: LiteflowConfig = serde_json::from_value(json!({
            "slotSize": 32,
            "whenMaxWaitTime": 2,
            "whenMaxWaitTimeUnit": "SECONDS",
            "agent": {
                "defaults": {"maxIterations": 7},
                "logging": {"reactEnabled": false}
            }
        }))
        .expect("配置应按 camelCase 反序列化");

        assert_eq!(config.slot_size(), 32);
        assert_eq!(
            config.when_max_wait_duration(),
            std::time::Duration::from_secs(2)
        );
        let agent: &AgentConfig = config.agent().expect("Agent 配置应保持强类型");
        assert_eq!(agent.defaults().max_iterations(), 7);
        assert!(!agent.logging().is_react_enabled());
    }

    #[test]
    #[allow(deprecated)]
    fn java_named_accessors_round_trip_values() {
        let mut config = LiteflowConfig::default();
        config.set_enable(false);
        config.set_rule_source(Some("flow.json"));
        config.set_slot_size(17);
        config.set_when_max_wait_seconds(Some(3));
        config.set_when_max_wait_time(4);
        config.set_when_max_wait_time_unit(TimeUnit::Seconds);
        config.set_queue_limit(19);
        config.set_delay(20);
        config.set_period(21);
        config.set_enable_log(true);
        config.set_support_multiple_type(true);
        config.set_retry_count(-1);
        config.set_print_banner(false);
        config.set_node_executor_class("custom.NodeExecutor");
        config.set_request_id_generator_class("custom.RequestId");
        config.set_main_executor_works(22);
        config.set_main_executor_class("custom.MainExecutor");
        config.set_print_execution_log(false);
        config.set_rule_source_ext_data(Some("tenant=a"));
        config.set_when_thread_pool_isolate(true);
        config.set_fallback_cmp_enable(true);
        config.set_fast_load(true);
        config.set_global_thread_pool_size(23);
        config.set_global_thread_pool_queue_size(24);
        config.set_global_thread_pool_executor_class("custom.GlobalExecutor");
        config.set_enable_node_instance_id(true);
        config.set_instance_id_generator_class("custom.InstanceId");
        config.set_chain_cache_enabled(true);
        config.set_chain_cache_capacity(25);
        config.set_enable_virtual_thread(false);

        assert!(!config.get_enable());
        assert_eq!(config.get_rule_source(), Some("flow.json"));
        assert_eq!(config.get_slot_size(), 17);
        assert_eq!(config.get_when_max_wait_seconds(), Some(3));
        assert_eq!(config.get_when_max_wait_time(), 4);
        assert_eq!(config.get_when_max_wait_time_unit(), TimeUnit::Seconds);
        assert_eq!(config.get_queue_limit(), 19);
        assert_eq!(config.get_delay(), 20);
        assert_eq!(config.get_period(), 21);
        assert!(config.get_enable_log());
        assert!(config.is_support_multiple_type());
        assert_eq!(config.get_retry_count(), 0);
        assert!(!config.get_print_banner());
        assert_eq!(config.get_node_executor_class(), "custom.NodeExecutor");
        assert_eq!(config.get_request_id_generator_class(), "custom.RequestId");
        assert_eq!(config.get_main_executor_works(), 22);
        assert_eq!(config.get_main_executor_class(), "custom.MainExecutor");
        assert!(!config.get_print_execution_log());
        assert_eq!(config.get_rule_source_ext_data(), Some("tenant=a"));
        assert!(config.get_when_thread_pool_isolate());
        assert!(config.get_fallback_cmp_enable());
        assert!(config.get_fast_load());
        assert_eq!(config.get_global_thread_pool_size(), 23);
        assert_eq!(config.get_global_thread_pool_queue_size(), 24);
        assert_eq!(
            config.get_global_thread_pool_executor_class(),
            "custom.GlobalExecutor"
        );
        assert!(config.get_enable_node_instance_id());
        assert_eq!(
            config.get_instance_id_generator_class(),
            "custom.InstanceId"
        );
        assert!(config.get_chain_cache_enabled());
        assert_eq!(config.get_chain_cache_capacity(), 25);
        assert!(!config.get_enable_virtual_thread());
    }
}
