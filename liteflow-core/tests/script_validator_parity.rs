use std::collections::HashMap;
use std::sync::Arc;

use liteflow_core::cmp;
use liteflow_core::core::NodeComponent;
use liteflow_core::enums::ScriptTypeEnum;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::script::validator::ScriptValidator;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use serde_json::Value;

fn validating_builder(
    node_id: &str,
    _kind: ScriptKind,
    script: &str,
) -> LFResult<Arc<dyn NodeComponent>> {
    if script != "valid" {
        return Err(LiteflowError::Script {
            node: node_id.to_string(),
            msg: "mock compile error".to_string(),
        });
    }
    Ok(Arc::new(cmp(|_| async { Ok(Value::Null) })))
}

#[test]
fn java_overloads_and_rust_diagnostics_share_real_executor_validation() {
    ScriptExecutorFactory::clean();

    // core 自带 Rhai 时等价于 Java 只发现一个 ScriptExecutor 的分支。
    assert!(ScriptValidator::validate("40 + 2"));
    assert!(!ScriptValidator::validate("let ="));
    assert!(ScriptValidator::validate_with_ex("40 + 2").is_success());
    assert!(!ScriptValidator::validate_with_ex("let =").is_success());
    assert!(!ScriptValidator::validate_for_language("missing", "valid"));

    ScriptExecutorFactory::register("custom", validating_builder)
        .expect("自定义测试执行器应注册成功");

    // 同时存在 Rhai 与自定义语言时，未指定类型必须像 Java 一样拒绝。
    let ambiguous = ScriptValidator::validate_with_ex("valid");
    assert!(!ambiguous.is_success());
    assert!(
        ambiguous
            .cause()
            .expect("多语言错误应保留原因")
            .to_string()
            .contains("language must be specified")
    );

    assert!(ScriptValidator::validate_with_script_type(
        "valid",
        ScriptTypeEnum::Custom
    ));
    assert!(!ScriptValidator::validate_with_script_type(
        "invalid",
        ScriptTypeEnum::Custom
    ));
    assert!(
        ScriptValidator::validate_with_ex_for_script_type("invalid", ScriptTypeEnum::Custom)
            .cause()
            .expect("编译失败应保留执行器错误")
            .to_string()
            .contains("mock compile error")
    );

    let diagnostics = ScriptValidator::validate_batch([
        ("rhai", "40 + 2"),
        ("custom", "invalid"),
        ("missing", "valid"),
    ]);
    assert!(diagnostics["rhai"].is_success());
    assert!(!diagnostics["custom"].is_success());
    assert!(!diagnostics["missing"].is_success());

    let mut valid_scripts = HashMap::new();
    valid_scripts.insert(ScriptTypeEnum::Rhai, "40 + 2");
    valid_scripts.insert(ScriptTypeEnum::Custom, "valid");
    assert!(ScriptValidator::validate_scripts(valid_scripts));

    assert!(!ScriptValidator::validate_scripts([
        (ScriptTypeEnum::Custom, "invalid"),
        (ScriptTypeEnum::Rhai, "40 + 2"),
    ]));
    assert!(ScriptValidator::ensure_valid("custom", "valid").is_ok());
    assert!(ScriptValidator::ensure_valid("custom", "invalid").is_err());

    ScriptExecutorFactory::unregister("custom");
}
