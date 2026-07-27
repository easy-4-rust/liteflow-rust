//! Java `ParameterWrapBean` 访问器与声明式代理元数据语义回归测试。

use liteflow_core::core::proxy::ParameterWrapBean;

#[test]
fn java_named_getters_read_the_same_metadata_written_by_setters() {
    let mut parameter = ParameterWrapBean::new("Arc<OrderFact>", Some("orderFact"), 1);

    assert_eq!(parameter.get_parameter_type(), "Arc<OrderFact>");
    assert_eq!(parameter.get_fact(), Some("orderFact"));
    assert_eq!(parameter.get_index(), 1);

    parameter.set_parameter_type("Arc<CustomerFact>");
    parameter.set_fact(Some("customerFact"));
    parameter.set_index(2);

    assert_eq!(parameter.get_parameter_type(), "Arc<CustomerFact>");
    assert_eq!(parameter.get_fact(), Some("customerFact"));
    assert_eq!(parameter.get_index(), 2);
}
