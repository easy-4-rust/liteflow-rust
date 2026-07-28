use crate::springboot::{LiteflowMonitorProperty, LiteflowProperty};
use crate::{LiteflowRuleFormat, LiteflowVernalConfig};

/// 合并 LiteFlow 主属性与监控属性的自动配置对象。
///
/// Java 从 `liteflow-default.properties` 和 Spring Environment 绑定两个属性对象，
/// 再逐字段写入统一 `LiteflowConfig`；Rust 由 serde/default 完成绑定，本对象
/// 保留同一合并边界。对应 Java:
/// `com.yomahub.liteflow.springboot.config.LiteflowPropertyAutoConfiguration`。
#[derive(Debug, Clone, Copy, Default)]
pub struct LiteflowPropertyAutoConfiguration;

impl LiteflowPropertyAutoConfiguration {
    /// 创建属性自动配置对象。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 合并主配置和监控配置。
    ///
    /// # 参数
    /// - `property`：`liteflow.*` 主属性；
    /// - `liteflow_monitor_property`：`liteflow.monitor.*` 监控属性。
    ///
    /// # 返回
    /// 可直接交给 Vernal 主自动配置的统一配置。对应 Java:
    /// `LiteflowPropertyAutoConfiguration#liteflowConfig`。
    #[must_use]
    pub fn liteflow_config(
        &self,
        property: &LiteflowProperty,
        liteflow_monitor_property: &LiteflowMonitorProperty,
    ) -> LiteflowVernalConfig {
        #[allow(deprecated)]
        let when_max_wait_seconds = property.get_when_max_wait_seconds();
        #[allow(deprecated)]
        let retry_count = property.get_retry_count();

        LiteflowVernalConfig {
            enable: property.is_enable(),
            rule_source: property.get_rule_source().map(str::to_string),
            rule_source_ext_data: property.get_rule_source_ext_data().map(str::to_string),
            rule_source_ext_data_map: property.get_rule_source_ext_data_map().clone(),
            slot_size: property.get_slot_size(),
            inline_rule: None,
            rule_format: LiteflowRuleFormat::Json,
            parse_mode: property.get_parse_mode(),
            when_max_wait_seconds,
            when_max_wait_time: property.get_when_max_wait_time(),
            when_max_wait_time_unit: property.get_when_max_wait_time_unit(),
            support_multiple_type: property.is_support_multiple_type(),
            retry_count,
            chain_cache_enabled: property.get_chain_cache().is_enabled(),
            chain_cache_capacity: property.get_chain_cache().get_capacity(),
            print_execution_log: property.is_print_execution_log(),
            node_executor_class: property.get_node_executor_class().to_string(),
            request_id_generator_class: property.get_request_id_generator_class().to_string(),
            enable_monitor_file: property.is_enable_monitor_file(),
            fallback_cmp_enable: property.is_fallback_cmp_enable(),
            fast_load: property.is_fast_load(),
            check_node_exists: property.is_check_node_exists(),
            script_setting: property.get_script_setting().clone(),
            print_banner: property.is_print_banner(),
            monitor_enable_log: liteflow_monitor_property.is_enable_log(),
            queue_limit: liteflow_monitor_property.get_queue_limit(),
            delay: liteflow_monitor_property.get_delay(),
            period: liteflow_monitor_property.get_period(),
            global_thread_pool_executor_class: property
                .get_global_thread_pool_executor_class()
                .to_string(),
            global_thread_pool_size: property.get_global_thread_pool_size(),
            global_thread_pool_queue_size: property.get_global_thread_pool_queue_size(),
            main_executor_class: property.get_main_executor_class().to_string(),
            main_executor_works: property.get_main_executor_works(),
            when_thread_pool_isolate: property.is_when_thread_pool_isolate(),
            enable_virtual_thread: property.is_enable_virtual_thread(),
            enable_node_instance_id: property.is_enable_node_instance_id(),
            agent: property.get_agent().cloned(),
        }
    }
}
