//! Java `InstanceInfoDto` 访问器及 Jackson 字段语义回归测试。

use liteflow_core::InstanceInfoDto;
use serde_json::json;

#[test]
fn java_named_accessors_and_serde_share_the_same_fields() {
    let mut instance = InstanceInfoDto::default();
    assert_eq!(instance.get_chain_id(), None);
    assert_eq!(instance.get_node_id(), None);
    assert_eq!(instance.get_instance_id(), None);
    assert_eq!(instance.get_index(), None);

    instance.set_chain_id("chain1");
    instance.set_node_id("a");
    instance.set_instance_id("a_runtime_0");
    instance.set_index(0);

    assert_eq!(instance.get_chain_id(), Some("chain1"));
    assert_eq!(instance.get_node_id(), Some("a"));
    assert_eq!(instance.get_instance_id(), Some("a_runtime_0"));
    assert_eq!(instance.get_index(), Some(0));
    assert_eq!(
        serde_json::to_value(&instance).expect("InstanceInfoDto 应可序列化"),
        json!({
            "chainId": "chain1",
            "nodeId": "a",
            "instanceId": "a_runtime_0",
            "index": 0
        })
    );
}
