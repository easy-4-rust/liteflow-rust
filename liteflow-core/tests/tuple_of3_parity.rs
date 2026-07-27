//! Java `TupleOf3` 访问器语义回归测试。

use liteflow_core::TupleOf3;

/// 验证 Java 命名 getter 与 setter 读取和修改同一组三元值。
#[test]
fn java_named_getters_share_state_with_tuple_setters() {
    let mut tuple = TupleOf3::new("hash-key".to_string(), "chain-a".to_string(), true);
    assert_eq!(tuple.get_a(), "hash-key");
    assert_eq!(tuple.get_b(), "chain-a");
    assert!(*tuple.get_c());

    tuple.set_a("new-hash".to_string());
    tuple.set_b("chain-b".to_string());
    tuple.set_c(false);
    assert_eq!(tuple.get_a(), "new-hash");
    assert_eq!(tuple.get_b(), "chain-b");
    assert!(!*tuple.get_c());
}
