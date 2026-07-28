use std::collections::HashMap;

use liteflow_core::property::TimeUnit;
use liteflow_core::property::agent::AgentConfig;
use serde::{Deserialize, Serialize};

use crate::LiteflowParseMode;

const DEFAULT_NODE_EXECUTOR: &str = "com.yomahub.liteflow.flow.executor.DefaultNodeExecutor";
const DEFAULT_REQUEST_ID_GENERATOR: &str = "com.yomahub.liteflow.flow.id.DefaultRequestIdGenerator";
const DEFAULT_MAIN_EXECUTOR: &str =
    "com.yomahub.liteflow.thread.LiteFlowDefaultMainExecutorBuilder";
const DEFAULT_GLOBAL_EXECUTOR: &str =
    "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder";

/// Boot 4 `LiteflowProperty` 内部的 Chain 缓存配置。
///
/// 内部类按规则与主对象保留在同一文件。对应 Java:
/// `com.yomahub.liteflow.springboot4.LiteflowProperty.ChainCacheProperty`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowPropertyChainCacheProperty {
    enabled: bool,
    capacity: usize,
}

impl Default for LiteflowPropertyChainCacheProperty {
    fn default() -> Self {
        Self {
            enabled: false,
            capacity: 10_000,
        }
    }
}

impl LiteflowPropertyChainCacheProperty {
    /// 返回是否启用 Chain 缓存。对应 Java: `isEnabled`。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 设置是否启用 Chain 缓存。参数 `enabled` 为新的开关。
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 返回 Chain 缓存容量。对应 Java: `getCapacity`。
    #[must_use]
    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    /// 设置 Chain 缓存容量。参数 `capacity` 为新的容量。
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
    }
}

/// LiteFlow 执行流程的 Spring Boot 4 主配置属性。
///
/// serde 对应 Spring/Jackson 属性绑定，默认值来自 Boot 4 starter 的
/// `META-INF/liteflow-default.properties`。对应 Java:
/// `com.yomahub.liteflow.springboot4.LiteflowProperty`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowProperty {
    enable: bool,
    rule_source: Option<String>,
    rule_source_ext_data: Option<String>,
    rule_source_ext_data_map: HashMap<String, String>,
    slot_size: usize,
    main_executor_works: usize,
    main_executor_class: String,
    when_max_wait_seconds: Option<u64>,
    when_max_wait_time: u64,
    when_max_wait_time_unit: TimeUnit,
    when_thread_pool_isolate: bool,
    parse_mode: LiteflowParseMode,
    support_multiple_type: bool,
    retry_count: i32,
    print_banner: bool,
    node_executor_class: String,
    request_id_generator_class: String,
    print_execution_log: bool,
    enable_monitor_file: bool,
    fallback_cmp_enable: bool,
    fast_load: bool,
    check_node_exists: bool,
    script_setting: HashMap<String, String>,
    global_thread_pool_executor_class: String,
    global_thread_pool_size: usize,
    global_thread_pool_queue_size: usize,
    enable_node_instance_id: bool,
    enable_virtual_thread: bool,
    chain_cache: LiteflowPropertyChainCacheProperty,
    agent: Option<AgentConfig>,
}

impl Default for LiteflowProperty {
    fn default() -> Self {
        Self {
            enable: true,
            rule_source: None,
            rule_source_ext_data: None,
            rule_source_ext_data_map: HashMap::new(),
            slot_size: 1024,
            main_executor_works: 64,
            main_executor_class: DEFAULT_MAIN_EXECUTOR.to_string(),
            when_max_wait_seconds: None,
            when_max_wait_time: 15_000,
            when_max_wait_time_unit: TimeUnit::Milliseconds,
            when_thread_pool_isolate: false,
            parse_mode: LiteflowParseMode::ParseAllOnStart,
            support_multiple_type: false,
            retry_count: 0,
            print_banner: true,
            node_executor_class: DEFAULT_NODE_EXECUTOR.to_string(),
            request_id_generator_class: DEFAULT_REQUEST_ID_GENERATOR.to_string(),
            print_execution_log: true,
            enable_monitor_file: false,
            fallback_cmp_enable: false,
            fast_load: false,
            check_node_exists: true,
            script_setting: HashMap::new(),
            global_thread_pool_executor_class: DEFAULT_GLOBAL_EXECUTOR.to_string(),
            global_thread_pool_size: 64,
            global_thread_pool_queue_size: 512,
            enable_node_instance_id: false,
            enable_virtual_thread: true,
            chain_cache: LiteflowPropertyChainCacheProperty::default(),
            agent: None,
        }
    }
}

impl LiteflowProperty {
    /// 返回是否启用 LiteFlow 自动装配。对应 Java: `isEnable`。
    #[must_use]
    pub fn is_enable(&self) -> bool {
        self.enable
    }

    /// 设置自动装配开关。参数 `enable` 对应 Java 参数。
    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
    }

    /// 返回规则资源地址。对应 Java: `getRuleSource`。
    #[must_use]
    pub fn get_rule_source(&self) -> Option<&str> {
        self.rule_source.as_deref()
    }

    /// 设置规则资源地址。参数 `rule_source` 对应 Java 参数。
    pub fn set_rule_source(&mut self, rule_source: Option<impl Into<String>>) {
        self.rule_source = rule_source.map(Into::into);
    }

    /// 返回规则资源扩展字符串。对应 Java: `getRuleSourceExtData`。
    #[must_use]
    pub fn get_rule_source_ext_data(&self) -> Option<&str> {
        self.rule_source_ext_data.as_deref()
    }

    /// 设置规则资源扩展字符串。参数 `rule_source_ext_data` 对应 Java 参数。
    pub fn set_rule_source_ext_data(&mut self, rule_source_ext_data: Option<impl Into<String>>) {
        self.rule_source_ext_data = rule_source_ext_data.map(Into::into);
    }

    /// 返回规则资源扩展映射。对应 Java: `getRuleSourceExtDataMap`。
    #[must_use]
    pub fn get_rule_source_ext_data_map(&self) -> &HashMap<String, String> {
        &self.rule_source_ext_data_map
    }

    /// 设置规则资源扩展映射。参数名与 Java 语义一致。
    pub fn set_rule_source_ext_data_map(
        &mut self,
        rule_source_ext_data_map: HashMap<String, String>,
    ) {
        self.rule_source_ext_data_map = rule_source_ext_data_map;
    }

    /// 返回 Slot 数量。对应 Java: `getSlotSize`。
    #[must_use]
    pub fn get_slot_size(&self) -> usize {
        self.slot_size
    }

    /// 设置 Slot 数量。参数 `slot_size` 对应 Java 参数。
    pub fn set_slot_size(&mut self, slot_size: usize) {
        self.slot_size = slot_size;
    }

    /// 返回主执行器 worker 数。对应 Java: `getMainExecutorWorks`。
    #[must_use]
    pub fn get_main_executor_works(&self) -> usize {
        self.main_executor_works
    }

    /// 设置主执行器 worker 数。参数 `main_executor_works` 对应 Java 参数。
    pub fn set_main_executor_works(&mut self, main_executor_works: usize) {
        self.main_executor_works = main_executor_works;
    }

    /// 返回主执行器构建器名，空白时回退默认值。
    #[must_use]
    pub fn get_main_executor_class(&self) -> &str {
        non_blank_or(&self.main_executor_class, DEFAULT_MAIN_EXECUTOR)
    }

    /// 设置主执行器构建器名。参数 `main_executor_class` 对应 Java 参数。
    pub fn set_main_executor_class(&mut self, main_executor_class: impl Into<String>) {
        self.main_executor_class = main_executor_class.into();
    }

    /// 返回废弃的秒级等待值。对应 Java: `getWhenMaxWaitSeconds`。
    #[deprecated(note = "使用 get_when_max_wait_time")]
    #[must_use]
    pub fn get_when_max_wait_seconds(&self) -> Option<u64> {
        self.when_max_wait_seconds
    }

    /// 设置废弃的秒级等待值。参数 `when_max_wait_seconds` 对应 Java 参数。
    #[deprecated(note = "使用 set_when_max_wait_time")]
    pub fn set_when_max_wait_seconds(&mut self, when_max_wait_seconds: Option<u64>) {
        self.when_max_wait_seconds = when_max_wait_seconds;
    }

    /// 返回 WHEN 最大等待值。对应 Java: `getWhenMaxWaitTime`。
    #[must_use]
    pub fn get_when_max_wait_time(&self) -> u64 {
        self.when_max_wait_time
    }

    /// 设置 WHEN 最大等待值。参数 `when_max_wait_time` 对应 Java 参数。
    pub fn set_when_max_wait_time(&mut self, when_max_wait_time: u64) {
        self.when_max_wait_time = when_max_wait_time;
    }

    /// 返回 WHEN 等待单位。对应 Java: `getWhenMaxWaitTimeUnit`。
    #[must_use]
    pub fn get_when_max_wait_time_unit(&self) -> TimeUnit {
        self.when_max_wait_time_unit
    }

    /// 设置 WHEN 等待单位。参数 `when_max_wait_time_unit` 对应 Java 参数。
    pub fn set_when_max_wait_time_unit(&mut self, when_max_wait_time_unit: TimeUnit) {
        self.when_max_wait_time_unit = when_max_wait_time_unit;
    }

    /// 返回 WHEN 线程池是否隔离。对应 Java: `isWhenThreadPoolIsolate`。
    #[must_use]
    pub fn is_when_thread_pool_isolate(&self) -> bool {
        self.when_thread_pool_isolate
    }

    /// 设置 WHEN 线程池隔离开关。参数名与 Java 语义一致。
    pub fn set_when_thread_pool_isolate(&mut self, when_thread_pool_isolate: bool) {
        self.when_thread_pool_isolate = when_thread_pool_isolate;
    }

    /// 返回解析模式。对应 Java: `getParseMode`。
    #[must_use]
    pub fn get_parse_mode(&self) -> LiteflowParseMode {
        self.parse_mode
    }

    /// 设置解析模式。参数 `parse_mode` 对应 Java 参数。
    pub fn set_parse_mode(&mut self, parse_mode: LiteflowParseMode) {
        self.parse_mode = parse_mode;
    }

    /// 返回是否支持多种规则格式。对应 Java: `isSupportMultipleType`。
    #[must_use]
    pub fn is_support_multiple_type(&self) -> bool {
        self.support_multiple_type
    }

    /// 设置多规则格式开关。参数名与 Java 语义一致。
    pub fn set_support_multiple_type(&mut self, support_multiple_type: bool) {
        self.support_multiple_type = support_multiple_type;
    }

    /// 返回废弃的全局重试次数。对应 Java: `getRetryCount`。
    #[deprecated]
    #[must_use]
    pub fn get_retry_count(&self) -> i32 {
        self.retry_count
    }

    /// 设置废弃的全局重试次数。参数 `retry_count` 对应 Java 参数。
    #[deprecated]
    pub fn set_retry_count(&mut self, retry_count: i32) {
        self.retry_count = retry_count;
    }

    /// 返回是否打印 Banner。对应 Java: `isPrintBanner`。
    #[must_use]
    pub fn is_print_banner(&self) -> bool {
        self.print_banner
    }

    /// 设置 Banner 开关。参数 `print_banner` 对应 Java 参数。
    pub fn set_print_banner(&mut self, print_banner: bool) {
        self.print_banner = print_banner;
    }

    /// 返回节点执行器类名，空白时回退 Java 默认实现。
    #[must_use]
    pub fn get_node_executor_class(&self) -> &str {
        non_blank_or(&self.node_executor_class, DEFAULT_NODE_EXECUTOR)
    }

    /// 设置节点执行器类名。参数 `node_executor_class` 对应 Java 参数。
    pub fn set_node_executor_class(&mut self, node_executor_class: impl Into<String>) {
        self.node_executor_class = node_executor_class.into();
    }

    /// 返回 Request ID 生成器类名，空白时回退 Java 默认实现。
    #[must_use]
    pub fn get_request_id_generator_class(&self) -> &str {
        non_blank_or(
            &self.request_id_generator_class,
            DEFAULT_REQUEST_ID_GENERATOR,
        )
    }

    /// 设置 Request ID 生成器类名。参数名与 Java 语义一致。
    pub fn set_request_id_generator_class(
        &mut self,
        request_id_generator_class: impl Into<String>,
    ) {
        self.request_id_generator_class = request_id_generator_class.into();
    }

    /// 返回是否打印执行日志。对应 Java: `isPrintExecutionLog`。
    #[must_use]
    pub fn is_print_execution_log(&self) -> bool {
        self.print_execution_log
    }

    /// 设置执行日志开关。参数 `print_execution_log` 对应 Java 参数。
    pub fn set_print_execution_log(&mut self, print_execution_log: bool) {
        self.print_execution_log = print_execution_log;
    }

    /// 返回是否监听规则文件。对应 Java: `isEnableMonitorFile`。
    #[must_use]
    pub fn is_enable_monitor_file(&self) -> bool {
        self.enable_monitor_file
    }

    /// 设置规则文件监听开关。参数 `enable_monitor_file` 对应 Java 参数。
    pub fn set_enable_monitor_file(&mut self, enable_monitor_file: bool) {
        self.enable_monitor_file = enable_monitor_file;
    }

    /// 返回是否启用组件降级。对应 Java: `isFallbackCmpEnable`。
    #[must_use]
    pub fn is_fallback_cmp_enable(&self) -> bool {
        self.fallback_cmp_enable
    }

    /// 设置组件降级开关。参数 `fallback_cmp_enable` 对应 Java 参数。
    pub fn set_fallback_cmp_enable(&mut self, fallback_cmp_enable: bool) {
        self.fallback_cmp_enable = fallback_cmp_enable;
    }

    /// 返回是否快速加载。对应 Java: `isFastLoad`。
    #[must_use]
    pub fn is_fast_load(&self) -> bool {
        self.fast_load
    }

    /// 设置快速加载开关。参数 `fast_load` 对应 Java 参数。
    pub fn set_fast_load(&mut self, fast_load: bool) {
        self.fast_load = fast_load;
    }

    /// 返回是否校验节点存在。对应 Java: `isCheckNodeExists`。
    #[must_use]
    pub fn is_check_node_exists(&self) -> bool {
        self.check_node_exists
    }

    /// 设置节点存在校验开关。参数 `check_node_exists` 对应 Java 参数。
    pub fn set_check_node_exists(&mut self, check_node_exists: bool) {
        self.check_node_exists = check_node_exists;
    }

    /// 返回脚本特殊设置。对应 Java: `getScriptSetting`。
    #[must_use]
    pub fn get_script_setting(&self) -> &HashMap<String, String> {
        &self.script_setting
    }

    /// 设置脚本特殊设置。参数 `script_setting` 对应 Java 参数。
    pub fn set_script_setting(&mut self, script_setting: HashMap<String, String>) {
        self.script_setting = script_setting;
    }

    /// 返回全局执行器并发数。对应 Java: `getGlobalThreadPoolSize`。
    #[must_use]
    pub fn get_global_thread_pool_size(&self) -> usize {
        self.global_thread_pool_size
    }

    /// 设置全局执行器并发数。参数 `global_thread_pool_size` 对应 Java 参数。
    pub fn set_global_thread_pool_size(&mut self, global_thread_pool_size: usize) {
        self.global_thread_pool_size = global_thread_pool_size;
    }

    /// 返回全局执行器队列容量。对应 Java: `getGlobalThreadPoolQueueSize`。
    #[must_use]
    pub fn get_global_thread_pool_queue_size(&self) -> usize {
        self.global_thread_pool_queue_size
    }

    /// 设置全局执行器队列容量。参数名与 Java 语义一致。
    pub fn set_global_thread_pool_queue_size(&mut self, global_thread_pool_queue_size: usize) {
        self.global_thread_pool_queue_size = global_thread_pool_queue_size;
    }

    /// 返回全局执行器构建器名，空白时回退 Java 默认实现。
    #[must_use]
    pub fn get_global_thread_pool_executor_class(&self) -> &str {
        non_blank_or(
            &self.global_thread_pool_executor_class,
            DEFAULT_GLOBAL_EXECUTOR,
        )
    }

    /// 设置全局执行器构建器名。参数名与 Java 语义一致。
    pub fn set_global_thread_pool_executor_class(
        &mut self,
        global_thread_pool_executor_class: impl Into<String>,
    ) {
        self.global_thread_pool_executor_class = global_thread_pool_executor_class.into();
    }

    /// 返回是否启用节点实例 ID。对应 Java: `isEnableNodeInstanceId`。
    #[must_use]
    pub fn is_enable_node_instance_id(&self) -> bool {
        self.enable_node_instance_id
    }

    /// 设置节点实例 ID 开关。参数 `enable_node_instance_id` 对应 Java 参数。
    pub fn set_enable_node_instance_id(&mut self, enable_node_instance_id: bool) {
        self.enable_node_instance_id = enable_node_instance_id;
    }

    /// 返回 Chain 缓存嵌套配置。对应 Java: `getChainCache`。
    #[must_use]
    pub fn get_chain_cache(&self) -> &LiteflowPropertyChainCacheProperty {
        &self.chain_cache
    }

    /// 设置 Chain 缓存嵌套配置。参数 `chain_cache` 对应 Java 参数。
    pub fn set_chain_cache(&mut self, chain_cache: LiteflowPropertyChainCacheProperty) {
        self.chain_cache = chain_cache;
    }

    /// 返回是否启用 Tokio 轻量任务。对应 Java: `isEnableVirtualThread`。
    #[must_use]
    pub fn is_enable_virtual_thread(&self) -> bool {
        self.enable_virtual_thread
    }

    /// 设置轻量任务开关。参数 `enable_virtual_thread` 对应 Java 参数。
    pub fn set_enable_virtual_thread(&mut self, enable_virtual_thread: bool) {
        self.enable_virtual_thread = enable_virtual_thread;
    }

    /// 返回 Agent 配置。对应 Java: `getAgent`。
    #[must_use]
    pub fn get_agent(&self) -> Option<&AgentConfig> {
        self.agent.as_ref()
    }

    /// 设置 Agent 配置。参数 `agent` 对应 Java 参数。
    pub fn set_agent(&mut self, agent: Option<AgentConfig>) {
        self.agent = agent;
    }
}

fn non_blank_or<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.trim().is_empty() {
        default
    } else {
        value
    }
}
