//! Java `MethodWrapBean` 元数据访问语义回归测试。

use liteflow_core::core::proxy::{LiteFlowMethodBean, MethodWrapBean, ParameterWrapBean};
use liteflow_core::enums::{LiteFlowMethodEnum, NodeTypeEnum};

#[test]
fn java_named_getters_share_the_metadata_used_by_decl_component_proxy() {
    let parameters = vec![ParameterWrapBean::new(
        "Arc<OrderFact>",
        Some("orderFact"),
        1,
    )];
    let mut method = MethodWrapBean::new(
        LiteFlowMethodBean::new("checkOrder", LiteFlowMethodEnum::ProcessBoolean),
        LiteFlowMethodEnum::ProcessBoolean,
        NodeTypeEnum::Boolean,
        Some(2),
        vec!["java.text.ParseException".to_string()],
        parameters,
    );

    assert_eq!(method.get_method().method_name(), "checkOrder");
    assert_eq!(
        method.get_liteflow_method(),
        LiteFlowMethodEnum::ProcessBoolean
    );
    assert_eq!(method.get_liteflow_retry(), Some(2));
    assert_eq!(method.get_parameter_wrap_bean_list().len(), 1);
    assert_eq!(
        method.get_parameter_wrap_bean_list()[0].fact(),
        Some("orderFact")
    );

    method.set_method(LiteFlowMethodBean::new(
        "checkOrderV2",
        LiteFlowMethodEnum::ProcessBoolean,
    ));
    method.set_liteflow_method(LiteFlowMethodEnum::IsAccess);
    method.set_liteflow_retry(None);
    method.set_parameter_wrap_bean_list(Vec::new());

    assert_eq!(method.get_method().method_name(), "checkOrderV2");
    assert_eq!(method.get_liteflow_method(), LiteFlowMethodEnum::IsAccess);
    assert_eq!(method.get_liteflow_retry(), None);
    assert!(method.get_parameter_wrap_bean_list().is_empty());
}
