use std::collections::HashMap;

use liteflow_core::property::agent::AgentConfig;
use serde::{Deserialize, Serialize};

use crate::LiteflowParseMode;

use super::PathsUtils;

const DEFAULT_NODE_EXECUTOR: &str = "com.yomahub.liteflow.flow.executor.DefaultNodeExecutor";
const DEFAULT_REQUEST_ID_GENERATOR: &str = "com.yomahub.liteflow.flow.id.DefaultRequestIdGenerator";
const DEFAULT_MAIN_EXECUTOR: &str =
    "com.yomahub.liteflow.thread.LiteFlowDefaultMainExecutorBuilder";
const DEFAULT_GLOBAL_EXECUTOR: &str =
    "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder";

/// Solon 主配置中的 Chain 规则缓存配置。
///
/// Java 内部类随主对象保留在同一文件；`Option` 对应 Java 包装类型允许配置
/// 绑定阶段缺省，getter 再提供 Java 默认值。对应 Java:
/// `com.yomahub.liteflow.solon.config.LiteflowProperty.ChainCacheProperty`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowPropertyChainCacheProperty {
    enabled: Option<bool>,
    capacity: Option<usize>,
}

impl Default for LiteflowPropertyChainCacheProperty {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            capacity: Some(10_000),
        }
    }
}

impl LiteflowPropertyChainCacheProperty {
    /// 返回规则缓存开关；未绑定时返回 `false`。对应 Java: `isEnabled`。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    /// 设置规则缓存开关。参数 `enabled` 对应 Java 参数 `enabled`。
    pub fn set_enabled(&mut self, enabled: Option<bool>) {
        self.enabled = enabled;
    }

    /// 返回规则缓存容量；未绑定时使用 Solon 默认值 `10000`。对应 Java:
    /// `getCapacity`。
    #[must_use]
    pub fn get_capacity(&self) -> usize {
        self.capacity.unwrap_or(10_000)
    }

    /// 设置规则缓存容量。参数 `capacity` 对应 Java 参数 `capacity`。
    pub fn set_capacity(&mut self, capacity: Option<usize>) {
        self.capacity = capacity;
    }
}

/// LiteFlow 在 Solon 环境中的主要执行参数对象。
///
/// serde 承担 Solon `@Inject("${liteflow}")` 的结构化配置绑定；字段、缺省值与
/// Java v2.16.0 `META-INF/liteflow-default.properties` 保持一致，并保留已经废弃
/// 但仍可绑定的线程池属性。对应 Java:
/// `com.yomahub.liteflow.solon.config.LiteflowProperty`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowProperty {
    enable: bool,
    #[serde(deserialize_with = "deserialize_rule_source")]
    rule_source: Option<String>,
    rule_source_ext_data: Option<String>,
    rule_source_ext_data_map: HashMap<String, String>,
    slot_size: usize,
    main_executor_works: usize,
    main_executor_class: String,
    thread_executor_class: Option<String>,
    when_max_wait_seconds: u64,
    when_max_workers: usize,
    when_queue_limit: usize,
    parse_mode: LiteflowParseMode,
    support_multiple_type: bool,
    retry_count: i32,
    print_banner: bool,
    node_executor_class: String,
    request_id_generator_class: String,
    print_execution_log: bool,
    parallel_loop_executor_class: Option<String>,
    parallel_max_workers: Option<usize>,
    parallel_queue_limit: Option<usize>,
    fallback_cmp_enable: Option<bool>,
    global_thread_pool_executor_class: Option<String>,
    global_thread_pool_size: Option<usize>,
    global_thread_pool_queue_size: Option<usize>,
    when_thread_pool_isolate: Option<bool>,
    enable_node_instance_id: bool,
    agent: Option<AgentConfig>,
    chain_cache: LiteflowPropertyChainCacheProperty,
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
            thread_executor_class: None,
            when_max_wait_seconds: 15,
            when_max_workers: 0,
            when_queue_limit: 0,
            parse_mode: LiteflowParseMode::ParseAllOnStart,
            support_multiple_type: false,
            retry_count: 0,
            print_banner: true,
            node_executor_class: DEFAULT_NODE_EXECUTOR.to_string(),
            request_id_generator_class: DEFAULT_REQUEST_ID_GENERATOR.to_string(),
            print_execution_log: true,
            parallel_loop_executor_class: None,
            parallel_max_workers: None,
            parallel_queue_limit: None,
            fallback_cmp_enable: Some(false),
            global_thread_pool_executor_class: Some(DEFAULT_GLOBAL_EXECUTOR.to_string()),
            global_thread_pool_size: Some(16),
            global_thread_pool_queue_size: Some(512),
            when_thread_pool_isolate: Some(false),
            enable_node_instance_id: false,
            agent: None,
            chain_cache: LiteflowPropertyChainCacheProperty::default(),
        }
    }
}

impl LiteflowProperty {
    /// 返回是否装配 LiteFlow。对应 Java: `isEnable`。
    #[must_use]
    pub fn is_enable(&self) -> bool {
        self.enable
    }

    /// 设置是否装配 LiteFlow。参数 `enable` 对应 Java 参数 `enable`。
    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
    }

    /// 返回流程定义资源地址。对应 Java: `getRuleSource`。
    #[must_use]
    pub fn get_rule_source(&self) -> Option<&str> {
        self.rule_source.as_deref()
    }

    /// 设置流程定义资源地址。
    ///
    /// 包含 `*` 时与 Java 一样立即调用 `PathsUtils#resolvePaths` 并以逗号连接；
    /// 参数 `rule_source` 对应 Java 参数 `ruleSource`。
    pub fn set_rule_source(&mut self, rule_source: Option<impl Into<String>>) {
        self.rule_source = rule_source.map(Into::into).map(|rule_source| {
            if rule_source.contains('*') {
                // Solon 在属性绑定阶段展开通配符，后续核心配置只接收确定路径。
                PathsUtils::resolve_paths(&rule_source).join(",")
            } else {
                rule_source
            }
        });
    }

    /// 返回流程资源扩展数据。对应 Java: `getRuleSourceExtData`。
    #[must_use]
    pub fn get_rule_source_ext_data(&self) -> Option<&str> {
        self.rule_source_ext_data.as_deref()
    }

    /// 设置流程资源扩展数据。参数对应 Java `ruleSourceExtData`。
    pub fn set_rule_source_ext_data(&mut self, rule_source_ext_data: Option<impl Into<String>>) {
        self.rule_source_ext_data = rule_source_ext_data.map(Into::into);
    }

    /// 返回流程资源扩展数据映射。对应 Java: `getRuleSourceExtDataMap`。
    #[must_use]
    pub fn get_rule_source_ext_data_map(&self) -> &HashMap<String, String> {
        &self.rule_source_ext_data_map
    }

    /// 设置流程资源扩展数据映射。参数对应 Java `ruleSourceExtDataMap`。
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

    /// 设置 Slot 数量。参数 `slot_size` 对应 Java 参数 `slotSize`。
    pub fn set_slot_size(&mut self, slot_size: usize) {
        self.slot_size = slot_size;
    }

    /// 返回主执行器 worker 数。对应 Java: `getMainExecutorWorks`。
    #[must_use]
    pub fn get_main_executor_works(&self) -> usize {
        self.main_executor_works
    }

    /// 设置主执行器 worker 数。参数对应 Java `mainExecutorWorks`。
    pub fn set_main_executor_works(&mut self, main_executor_works: usize) {
        self.main_executor_works = main_executor_works;
    }

    /// 返回主执行器构建器类名。对应 Java: `getMainExecutorClass`。
    #[must_use]
    pub fn get_main_executor_class(&self) -> &str {
        non_blank_or(&self.main_executor_class, DEFAULT_MAIN_EXECUTOR)
    }

    /// 设置主执行器构建器类名。参数对应 Java `mainExecutorClass`。
    pub fn set_main_executor_class(&mut self, main_executor_class: impl Into<String>) {
        self.main_executor_class = main_executor_class.into();
    }

    /// 返回旧版并行线程执行器类名。对应 Java: `getThreadExecutorClass`。
    #[must_use]
    pub fn get_thread_executor_class(&self) -> Option<&str> {
        self.thread_executor_class.as_deref()
    }

    /// 设置旧版并行线程执行器类名。参数对应 Java `threadExecutorClass`。
    pub fn set_thread_executor_class(&mut self, thread_executor_class: Option<impl Into<String>>) {
        self.thread_executor_class = thread_executor_class.map(Into::into);
    }

    /// 返回异步任务最大等待秒数。对应 Java: `getWhenMaxWaitSeconds`。
    #[must_use]
    pub fn get_when_max_wait_seconds(&self) -> u64 {
        self.when_max_wait_seconds
    }

    /// 设置异步任务最大等待秒数。参数对应 Java `whenMaxWaitSeconds`。
    pub fn set_when_max_wait_seconds(&mut self, when_max_wait_seconds: u64) {
        self.when_max_wait_seconds = when_max_wait_seconds;
    }

    /// 返回旧版异步线程池最大线程数。对应 Java: `getWhenMaxWorkers`。
    #[must_use]
    pub fn get_when_max_workers(&self) -> usize {
        self.when_max_workers
    }

    /// 设置旧版异步线程池最大线程数。参数对应 Java `whenMaxWorkers`。
    pub fn set_when_max_workers(&mut self, when_max_workers: usize) {
        self.when_max_workers = when_max_workers;
    }

    /// 返回旧版异步线程池最大队列数。对应 Java: `getWhenQueueLimit`。
    #[must_use]
    pub fn get_when_queue_limit(&self) -> usize {
        self.when_queue_limit
    }

    /// 设置旧版异步线程池最大队列数。参数对应 Java `whenQueueLimit`。
    pub fn set_when_queue_limit(&mut self, when_queue_limit: usize) {
        self.when_queue_limit = when_queue_limit;
    }

    /// 返回规则解析模式。对应 Java: `getParseMode`。
    #[must_use]
    pub fn get_parse_mode(&self) -> LiteflowParseMode {
        self.parse_mode
    }

    /// 设置规则解析模式。参数 `parse_mode` 对应 Java 参数 `parseMode`。
    pub fn set_parse_mode(&mut self, parse_mode: LiteflowParseMode) {
        self.parse_mode = parse_mode;
    }

    /// 返回是否支持多种规则类型。对应 Java: `isSupportMultipleType`。
    #[must_use]
    pub fn is_support_multiple_type(&self) -> bool {
        self.support_multiple_type
    }

    /// 设置多规则类型开关。
    ///
    /// 主流程和子流程仍不能分配到不同类型配置文件；参数对应 Java
    /// `supportMultipleType`。
    pub fn set_support_multiple_type(&mut self, support_multiple_type: bool) {
        self.support_multiple_type = support_multiple_type;
    }

    /// 返回全局重试次数。对应 Java: `getRetryCount`。
    #[must_use]
    pub fn get_retry_count(&self) -> i32 {
        self.retry_count
    }

    /// 设置全局重试次数。参数对应 Java `retryCount`。
    pub fn set_retry_count(&mut self, retry_count: i32) {
        self.retry_count = retry_count;
    }

    /// 返回是否打印 LiteFlow Banner。对应 Java: `isPrintBanner`。
    #[must_use]
    pub fn is_print_banner(&self) -> bool {
        self.print_banner
    }

    /// 设置是否打印 LiteFlow Banner。参数对应 Java `printBanner`。
    pub fn set_print_banner(&mut self, print_banner: bool) {
        self.print_banner = print_banner;
    }

    /// 返回节点执行器类名。对应 Java: `getNodeExecutorClass`。
    #[must_use]
    pub fn get_node_executor_class(&self) -> &str {
        non_blank_or(&self.node_executor_class, DEFAULT_NODE_EXECUTOR)
    }

    /// 设置节点执行器类名。参数对应 Java `nodeExecutorClass`。
    pub fn set_node_executor_class(&mut self, node_executor_class: impl Into<String>) {
        self.node_executor_class = node_executor_class.into();
    }

    /// 返回 Request ID 生成器类名。对应 Java: `getRequestIdGeneratorClass`。
    #[must_use]
    pub fn get_request_id_generator_class(&self) -> &str {
        non_blank_or(
            &self.request_id_generator_class,
            DEFAULT_REQUEST_ID_GENERATOR,
        )
    }

    /// 设置 Request ID 生成器类名。参数对应 Java `requestIdGeneratorClass`。
    pub fn set_request_id_generator_class(
        &mut self,
        request_id_generator_class: impl Into<String>,
    ) {
        self.request_id_generator_class = request_id_generator_class.into();
    }

    /// 返回是否打印执行过程日志。对应 Java: `isPrintExecutionLog`。
    #[must_use]
    pub fn is_print_execution_log(&self) -> bool {
        self.print_execution_log
    }

    /// 设置是否打印执行过程日志。参数对应 Java `printExecutionLog`。
    pub fn set_print_execution_log(&mut self, print_execution_log: bool) {
        self.print_execution_log = print_execution_log;
    }

    /// 返回旧版并行循环线程池类名。对应 Java: `getParallelLoopExecutorClass`。
    #[must_use]
    pub fn get_parallel_loop_executor_class(&self) -> Option<&str> {
        self.parallel_loop_executor_class.as_deref()
    }

    /// 设置旧版并行循环线程池类名。参数对应 Java `parallelLoopExecutorClass`。
    pub fn set_parallel_loop_executor_class(
        &mut self,
        parallel_loop_executor_class: Option<impl Into<String>>,
    ) {
        self.parallel_loop_executor_class = parallel_loop_executor_class.map(Into::into);
    }

    /// 返回旧版并行循环最大线程数。对应 Java: `getParallelMaxWorkers`。
    #[must_use]
    pub fn get_parallel_max_workers(&self) -> Option<usize> {
        self.parallel_max_workers
    }

    /// 设置旧版并行循环最大线程数。参数对应 Java `parallelMaxWorkers`。
    pub fn set_parallel_max_workers(&mut self, parallel_max_workers: Option<usize>) {
        self.parallel_max_workers = parallel_max_workers;
    }

    /// 返回旧版并行循环最大队列数。对应 Java: `getParallelQueueLimit`。
    #[must_use]
    pub fn get_parallel_queue_limit(&self) -> Option<usize> {
        self.parallel_queue_limit
    }

    /// 设置旧版并行循环最大队列数。参数对应 Java `parallelQueueLimit`。
    pub fn set_parallel_queue_limit(&mut self, parallel_queue_limit: Option<usize>) {
        self.parallel_queue_limit = parallel_queue_limit;
    }

    /// 返回组件降级开关；未绑定时返回 `false`。对应 Java:
    /// `isFallbackCmpEnable` / `getFallbackCmpEnable`。
    #[must_use]
    pub fn is_fallback_cmp_enable(&self) -> bool {
        self.fallback_cmp_enable.unwrap_or(false)
    }

    /// 设置组件降级开关。参数对应 Java `fallbackCmpEnable`。
    pub fn set_fallback_cmp_enable(&mut self, fallback_cmp_enable: Option<bool>) {
        self.fallback_cmp_enable = fallback_cmp_enable;
    }

    /// 返回组件降级包装值。对应 Java: `getFallbackCmpEnable`。
    #[must_use]
    pub fn get_fallback_cmp_enable(&self) -> Option<bool> {
        self.fallback_cmp_enable
    }

    /// 返回全局线程池构建器，空白时使用 Java 默认实现。对应 Java:
    /// `getGlobalThreadPoolExecutorClass`。
    #[must_use]
    pub fn get_global_thread_pool_executor_class(&self) -> &str {
        self.global_thread_pool_executor_class
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_GLOBAL_EXECUTOR)
    }

    /// 设置全局线程池构建器类名。参数对应 Java `globalThreadPoolExecutorClass`。
    pub fn set_global_thread_pool_executor_class(
        &mut self,
        global_thread_pool_executor_class: Option<impl Into<String>>,
    ) {
        self.global_thread_pool_executor_class = global_thread_pool_executor_class.map(Into::into);
    }

    /// 返回全局线程池大小，未绑定时为 `16`。对应 Java:
    /// `getGlobalThreadPoolSize`。
    #[must_use]
    pub fn get_global_thread_pool_size(&self) -> usize {
        self.global_thread_pool_size.unwrap_or(16)
    }

    /// 设置全局线程池大小。参数对应 Java `globalThreadPoolSize`。
    pub fn set_global_thread_pool_size(&mut self, global_thread_pool_size: Option<usize>) {
        self.global_thread_pool_size = global_thread_pool_size;
    }

    /// 返回全局线程池队列大小，未绑定时为 `512`。对应 Java:
    /// `getGlobalThreadPoolQueueSize`。
    #[must_use]
    pub fn get_global_thread_pool_queue_size(&self) -> usize {
        self.global_thread_pool_queue_size.unwrap_or(512)
    }

    /// 设置全局线程池队列大小。参数对应 Java `globalThreadPoolQueueSize`。
    pub fn set_global_thread_pool_queue_size(
        &mut self,
        global_thread_pool_queue_size: Option<usize>,
    ) {
        self.global_thread_pool_queue_size = global_thread_pool_queue_size;
    }

    /// 返回 WHEN 线程池隔离开关，未绑定时为 `false`。对应 Java:
    /// `getWhenThreadPoolIsolate`。
    #[must_use]
    pub fn get_when_thread_pool_isolate(&self) -> bool {
        self.when_thread_pool_isolate.unwrap_or(false)
    }

    /// 设置 WHEN 线程池隔离开关。参数对应 Java `whenThreadPoolIsolate`。
    pub fn set_when_thread_pool_isolate(&mut self, when_thread_pool_isolate: Option<bool>) {
        self.when_thread_pool_isolate = when_thread_pool_isolate;
    }

    /// 返回是否启用节点实例 ID。对应 Java: `isEnableNodeInstanceId`。
    #[must_use]
    pub fn is_enable_node_instance_id(&self) -> bool {
        self.enable_node_instance_id
    }

    /// 设置节点实例 ID 开关。参数对应 Java `enableNodeInstanceId`。
    pub fn set_enable_node_instance_id(&mut self, enable_node_instance_id: bool) {
        self.enable_node_instance_id = enable_node_instance_id;
    }

    /// 返回 Agent 嵌套配置。对应 Java: `getAgent`。
    #[must_use]
    pub fn get_agent(&self) -> Option<&AgentConfig> {
        self.agent.as_ref()
    }

    /// 设置 Agent 嵌套配置。参数 `agent` 对应 Java 参数 `agent`。
    pub fn set_agent(&mut self, agent: Option<AgentConfig>) {
        self.agent = agent;
    }

    /// 返回 Chain 规则缓存配置。对应 Java: `getChainCache`。
    #[must_use]
    pub fn get_chain_cache(&self) -> &LiteflowPropertyChainCacheProperty {
        &self.chain_cache
    }

    /// 设置 Chain 规则缓存配置。参数 `chain_cache` 对应 Java 参数 `chainCache`。
    pub fn set_chain_cache(&mut self, chain_cache: LiteflowPropertyChainCacheProperty) {
        self.chain_cache = chain_cache;
    }
}

fn non_blank_or<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.trim().is_empty() {
        default
    } else {
        value
    }
}

fn deserialize_rule_source<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let rule_source = Option::<String>::deserialize(deserializer)?;
    Ok(rule_source.map(|rule_source| {
        if rule_source.contains('*') {
            // serde 属性绑定必须经过与 Java setter 相同的通配符展开逻辑。
            PathsUtils::resolve_paths(&rule_source).join(",")
        } else {
            rule_source
        }
    }))
}
