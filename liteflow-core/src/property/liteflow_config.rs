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

macro_rules! copy_accessors {
    ($(($getter:ident, $setter:ident, $field:ident, $ty:ty, $java:literal)),+ $(,)?) => {
        $(
            #[doc = concat!("返回 `", stringify!($field), "`。对应 Java: `LiteflowConfig#", $java, "`。")]
            #[must_use]
            pub fn $getter(&self) -> $ty {
                self.$field
            }

            #[doc = concat!("设置 `", stringify!($field), "`；与 Java `LiteflowConfig#", $java, "` 访问同一配置项。")]
            pub fn $setter(&mut self, value: $ty) {
                self.$field = value;
            }
        )+
    };
}

macro_rules! option_string_accessors {
    ($(($getter:ident, $setter:ident, $field:ident, $java:literal)),+ $(,)?) => {
        $(
            #[doc = concat!("返回可选的 `", stringify!($field), "`。对应 Java: `LiteflowConfig#get", $java, "`。")]
            #[must_use]
            pub fn $getter(&self) -> Option<&str> {
                self.$field.as_deref()
            }

            #[doc = concat!("设置可选的 `", stringify!($field), "`。对应 Java: `LiteflowConfig#set", $java, "`。")]
            pub fn $setter(&mut self, value: Option<impl Into<String>>) {
                self.$field = value.map(Into::into);
            }
        )+
    };
}

impl LiteflowConfig {
    copy_accessors!(
        (is_enabled, set_enabled, enable, bool, "getEnable"),
        (slot_size, set_slot_size, slot_size, usize, "getSlotSize"),
        (
            when_max_wait_time,
            set_when_max_wait_time,
            when_max_wait_time,
            u64,
            "getWhenMaxWaitTime"
        ),
        (
            when_max_wait_time_unit,
            set_when_max_wait_time_unit,
            when_max_wait_time_unit,
            TimeUnit,
            "getWhenMaxWaitTimeUnit"
        ),
        (
            is_when_thread_pool_isolate,
            set_when_thread_pool_isolate,
            when_thread_pool_isolate,
            bool,
            "getWhenThreadPoolIsolate"
        ),
        (
            is_enable_log,
            set_enable_log,
            enable_log,
            bool,
            "getEnableLog"
        ),
        (
            queue_limit,
            set_queue_limit,
            queue_limit,
            usize,
            "getQueueLimit"
        ),
        (delay, set_delay, delay, u64, "getDelay"),
        (period, set_period, period, u64, "getPeriod"),
        (
            parse_mode,
            set_parse_mode,
            parse_mode,
            ParseModeEnum,
            "getParseMode"
        ),
        (
            is_support_multiple_type,
            set_support_multiple_type,
            support_multiple_type,
            bool,
            "isSupportMultipleType"
        ),
        (
            is_print_banner,
            set_print_banner,
            print_banner,
            bool,
            "getPrintBanner"
        ),
        (
            main_executor_works,
            set_main_executor_works,
            main_executor_works,
            usize,
            "getMainExecutorWorks"
        ),
        (
            is_print_execution_log,
            set_print_execution_log,
            print_execution_log,
            bool,
            "getPrintExecutionLog"
        ),
        (
            is_enable_monitor_file,
            set_enable_monitor_file,
            enable_monitor_file,
            bool,
            "getEnableMonitorFile"
        ),
        (
            is_fallback_cmp_enabled,
            set_fallback_cmp_enabled,
            fallback_cmp_enable,
            bool,
            "getFallbackCmpEnable"
        ),
        (is_fast_load, set_fast_load, fast_load, bool, "getFastLoad"),
        (
            is_enable_node_instance_id,
            set_enable_node_instance_id,
            enable_node_instance_id,
            bool,
            "getEnableNodeInstanceId"
        ),
        (
            is_chain_cache_enabled,
            set_chain_cache_enabled,
            chain_cache_enabled,
            bool,
            "getChainCacheEnabled"
        ),
        (
            chain_cache_capacity,
            set_chain_cache_capacity,
            chain_cache_capacity,
            usize,
            "getChainCacheCapacity"
        ),
        (
            is_enable_virtual_thread,
            set_enable_virtual_thread,
            enable_virtual_thread,
            bool,
            "getEnableVirtualThread"
        ),
        (
            global_thread_pool_size,
            set_global_thread_pool_size,
            global_thread_pool_size,
            usize,
            "getGlobalThreadPoolSize"
        ),
        (
            global_thread_pool_queue_size,
            set_global_thread_pool_queue_size,
            global_thread_pool_queue_size,
            usize,
            "getGlobalThreadPoolQueueSize"
        ),
    );

    option_string_accessors!(
        (rule_source, set_rule_source, rule_source, "RuleSource"),
        (
            rule_source_ext_data,
            set_rule_source_ext_data,
            rule_source_ext_data,
            "RuleSourceExtData"
        ),
    );

    /// 返回废弃的秒级最大等待值；`0` 与未配置均返回 `None`。
    #[deprecated(note = "使用 when_max_wait_time 与 when_max_wait_time_unit")]
    #[must_use]
    pub fn when_max_wait_seconds(&self) -> Option<u64> {
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
    pub fn retry_count(&self) -> u32 {
        self.retry_count.max(0) as u32
    }

    /// 设置废弃的重试次数。对应 Java: `setRetryCount`。
    #[deprecated]
    pub fn set_retry_count(&mut self, value: i32) {
        self.retry_count = value;
    }

    /// 返回规则资源扩展数据映射。对应 Java: `getRuleSourceExtDataMap`。
    #[must_use]
    pub fn rule_source_ext_data_map(&self) -> &HashMap<String, String> {
        &self.rule_source_ext_data_map
    }

    /// 设置规则资源扩展数据映射。对应 Java: `setRuleSourceExtDataMap`。
    pub fn set_rule_source_ext_data_map(&mut self, value: HashMap<String, String>) {
        self.rule_source_ext_data_map = value;
    }

    /// 返回脚本设置映射。对应 Java: `getScriptSetting`。
    #[must_use]
    pub fn script_setting(&self) -> &HashMap<String, String> {
        &self.script_setting
    }

    /// 设置脚本设置映射。对应 Java: `setScriptSetting`。
    pub fn set_script_setting(&mut self, value: HashMap<String, String>) {
        self.script_setting = value;
    }

    /// 返回节点执行器类名，空白值回退到 Java 默认实现。
    #[must_use]
    pub fn node_executor_class(&self) -> &str {
        non_blank_or(&self.node_executor_class, DEFAULT_NODE_EXECUTOR)
    }

    /// 设置节点执行器类名。对应 Java: `setNodeExecutorClass`。
    pub fn set_node_executor_class(&mut self, value: impl Into<String>) {
        self.node_executor_class = value.into();
    }

    /// 返回 Request ID 生成器类名，空白值回退到 Java 默认实现。
    #[must_use]
    pub fn request_id_generator_class(&self) -> &str {
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
    pub fn main_executor_class(&self) -> &str {
        non_blank_or(&self.main_executor_class, DEFAULT_MAIN_EXECUTOR)
    }

    /// 设置主执行器构建器类名。对应 Java: `setMainExecutorClass`。
    pub fn set_main_executor_class(&mut self, value: impl Into<String>) {
        self.main_executor_class = value.into();
    }

    /// 返回实例 ID 生成器类名，空白值回退到 Java 当前默认实现。
    #[must_use]
    pub fn instance_id_generator_class(&self) -> &str {
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
    pub fn global_thread_pool_executor_class(&self) -> &str {
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
    pub fn agent(&self) -> Option<&AgentConfig> {
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
        assert!(config.is_enabled());
        assert_eq!(config.slot_size(), 1024);
        assert_eq!(config.when_max_wait_time(), 15_000);
        assert_eq!(config.when_max_wait_time_unit(), TimeUnit::Milliseconds);
        assert_eq!(config.parse_mode(), ParseModeEnum::ParseAllOnStart);
        assert_eq!(config.node_executor_class(), super::DEFAULT_NODE_EXECUTOR);
        assert_eq!(config.chain_cache_capacity(), 10_000);
        assert!(config.is_enable_virtual_thread());
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
}
