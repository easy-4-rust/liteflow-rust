//! Java `DefaultsConfig` 默认值和访问器语义回归测试。

use liteflow_core::property::agent::DefaultsConfig;

/// 验证默认迭代上限及 Java 命名 getter 与 setter 共享同一字段。
#[test]
fn max_iterations_getter_preserves_java_default_and_mutation() {
    let mut config = DefaultsConfig::default();
    assert_eq!(config.get_max_iterations(), 50);

    config.set_max_iterations(12);
    assert_eq!(config.get_max_iterations(), 12);
    assert_eq!(config.max_iterations(), 12);
}
