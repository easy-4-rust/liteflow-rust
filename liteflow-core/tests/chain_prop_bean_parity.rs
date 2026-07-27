use liteflow_core::{ChainPropBean, ConditionTypeEnum};

/// 验证 ChainPropBean 保留 Java 的链式 setter、getter 与 camelCase 配置字段。
#[test]
fn chain_prop_bean_exposes_java_equivalent_accessors() {
    let bean = ChainPropBean::default()
        .set_cond_value_str("THEN(a,b)")
        .set_group("order")
        .set_error_resume("true")
        .set_any("false")
        .set_thread_executor_class("demo.OrderExecutor")
        .set_condition_type(ConditionTypeEnum::Then);

    assert_eq!(bean.get_cond_value_str(), Some("THEN(a,b)"));
    assert_eq!(bean.get_group(), Some("order"));
    assert_eq!(bean.get_error_resume(), Some("true"));
    assert_eq!(bean.get_any(), Some("false"));
    assert_eq!(bean.get_thread_executor_class(), Some("demo.OrderExecutor"));
    assert_eq!(bean.get_condition_type(), Some(ConditionTypeEnum::Then));

    let json = serde_json::to_value(&bean).expect("ChainPropBean 应可序列化");
    assert_eq!(json["condValueStr"], "THEN(a,b)");
    assert_eq!(json["threadExecutorClass"], "demo.OrderExecutor");
    assert_eq!(json["conditionType"], "then");
}
