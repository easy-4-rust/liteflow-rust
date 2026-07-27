//! QLExpress Java 对象级缓存与编译契约测试。

use liteflow_core::script::ScriptExecutor;
use liteflow_script_qlexpress::QlExpressScriptExecutor;

#[test]
fn executor_compiles_caches_unloads_and_rejects_invalid_qlexpress() {
    let executor = QlExpressScriptExecutor::new();
    let script = r#"
        // Java QLExpress 支持单行注释与单词形式的逻辑运算符。
        count = defaultContext.getData("count");
        if (count > 100 and not defaultContext.hasData("blocked")) {
            return "a";
        } else {
            return "b";
        }
    "#;

    assert!(executor.validate(script));
    assert!(!executor.validate("if (count >) { return true; }"));

    executor.load("switch_node", script).unwrap();
    executor.load("for_node", "return 3;").unwrap();
    assert_eq!(
        executor.node_ids().unwrap(),
        vec!["for_node".to_string(), "switch_node".to_string()]
    );

    executor.unload("switch_node").unwrap();
    assert_eq!(executor.node_ids().unwrap(), vec!["for_node".to_string()]);
    executor.clean_cache().unwrap();
    assert!(executor.node_ids().unwrap().is_empty());
}
