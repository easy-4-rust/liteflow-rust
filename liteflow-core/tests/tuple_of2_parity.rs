//! Java `TupleOf2` 访问器语义回归测试。

use liteflow_core::TupleOf2;

/// 验证 Java 命名 getter 与 setter 读取和修改同一对值。
#[test]
fn java_named_getters_share_state_with_tuple_setters() {
    let mut tuple = TupleOf2::new("chain-a".to_string(), 1_u32);
    assert_eq!(tuple.get_a(), "chain-a");
    assert_eq!(*tuple.get_b(), 1);

    tuple.set_a("chain-b".to_string());
    tuple.set_b(2);
    assert_eq!(tuple.get_a(), "chain-b");
    assert_eq!(*tuple.get_b(), 2);
}
