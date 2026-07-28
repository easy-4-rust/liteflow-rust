use liteflow_core::monitor::MonitorBus;

use crate::{LiteflowRuleFormat, LiteflowVernalConfig};

use super::{LiteflowMonitorProperty, LiteflowProperty};

/// 将 Solon 主属性与监控属性合并为统一 LiteFlow 配置的自动配置对象。
///
/// Java 对象从 Solon `app.properties`/YAML 注入两个属性对象，再逐字段写入
/// `LiteflowConfig`；Rust 保留同一合并边界，serde 负责属性绑定。对应 Java:
/// `com.yomahub.liteflow.solon.config.LiteflowAutoConfiguration`。
#[derive(Debug, Clone, Copy, Default)]
pub struct LiteflowAutoConfiguration {
    enable_log: bool,
}

impl LiteflowAutoConfiguration {
    /// 使用 `liteflow.monitor.enableLog` 创建自动配置对象。
    ///
    /// # 参数
    /// - `enable_log`：是否创建 `MonitorBus`，对应 Java 注入字段 `enableLog`。
    #[must_use]
    pub const fn new(enable_log: bool) -> Self {
        Self { enable_log }
    }

    /// 合并主属性与监控属性。
    ///
    /// # 参数
    /// - `property`：Solon `liteflow.*` 主属性；
    /// - `liteflow_monitor_property`：Solon `liteflow.monitor.*` 属性。
    ///
    /// # 返回
    /// 可交给 Vernal 宿主执行的统一配置。Java 已废弃且不再写入核心配置的
    /// `threadExecutorClass`、`whenMaxWorkers`、`whenQueueLimit` 与并行循环字段
    /// 也按原实现保持“不参与合并”。对应 Java:
    /// `LiteflowAutoConfiguration#liteflowConfig`。
    #[must_use]
    pub fn liteflow_config(
        &self,
        property: &LiteflowProperty,
        liteflow_monitor_property: &LiteflowMonitorProperty,
    ) -> LiteflowVernalConfig {
        LiteflowVernalConfig {
            enable: property.is_enable(),
            rule_source: property.get_rule_source().map(str::to_string),
            rule_source_ext_data: property.get_rule_source_ext_data().map(str::to_string),
            rule_source_ext_data_map: property.get_rule_source_ext_data_map().clone(),
            slot_size: property.get_slot_size(),
            rule_format: LiteflowRuleFormat::Json,
            parse_mode: property.get_parse_mode(),
            when_max_wait_seconds: Some(property.get_when_max_wait_seconds()),
            support_multiple_type: property.is_support_multiple_type(),
            retry_count: property.get_retry_count(),
            chain_cache_enabled: property.get_chain_cache().is_enabled(),
            chain_cache_capacity: property.get_chain_cache().get_capacity(),
            print_execution_log: property.is_print_execution_log(),
            node_executor_class: property.get_node_executor_class().to_string(),
            request_id_generator_class: property.get_request_id_generator_class().to_string(),
            fallback_cmp_enable: property.is_fallback_cmp_enable(),
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
            when_thread_pool_isolate: property.get_when_thread_pool_isolate(),
            enable_node_instance_id: property.is_enable_node_instance_id(),
            agent: property.get_agent().cloned(),
            ..LiteflowVernalConfig::default()
        }
    }

    /// 按注入开关创建监控总线。
    ///
    /// # 参数
    /// - `liteflow_config`：已经合并完成的统一配置。
    ///
    /// # 返回
    /// 开启监控时返回真实 `MonitorBus`，关闭时返回 `None`，对应 Java 返回
    /// `null` 表示未创建 Bean。对应 Java: `LiteflowAutoConfiguration#monitorBus`。
    #[must_use]
    pub fn monitor_bus(&self, liteflow_config: &LiteflowVernalConfig) -> Option<MonitorBus> {
        if !self.enable_log {
            return None;
        }
        let monitor_bus = MonitorBus::new();
        monitor_bus.set_liteflow_config(liteflow_config.to_core_config());
        Some(monitor_bus)
    }
}
