//! Java `LiteFlowMethodBean` 访问器与声明式生命周期元数据语义回归测试。

use liteflow_core::core::proxy::LiteFlowMethodBean;
use liteflow_core::enums::LiteFlowMethodEnum;

#[test]
fn java_named_getters_read_the_metadata_used_by_static_dispatch() {
    let mut method = LiteFlowMethodBean::new("processOrder", LiteFlowMethodEnum::ProcessBoolean);

    assert_eq!(method.get_method_name(), "processOrder");
    assert_eq!(method.get_method(), LiteFlowMethodEnum::ProcessBoolean);

    method.set_method_name("isAccess");
    method.set_method(LiteFlowMethodEnum::IsAccess);

    assert_eq!(method.get_method_name(), "isAccess");
    assert_eq!(method.get_method(), LiteFlowMethodEnum::IsAccess);
}
