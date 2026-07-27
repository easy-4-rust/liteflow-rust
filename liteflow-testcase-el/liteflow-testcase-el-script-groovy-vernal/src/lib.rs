//! Groovy 脚本与 Vernal 组合场景。

use liteflow_core::FlowBus;
use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_groovy::GroovyScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册并真实执行 Groovy LiteFlow 上下文绑定基线。
///
/// 返回执行器注册、Vernal 配置、链路执行和上下文写回是否全部成功。
/// 对应 Java: `GroovyScriptExecutor` 的普通脚本 testcase。
pub async fn run_case() -> bool {
    if GroovyScriptExecutor::register().is_err()
        || !ScriptExecutorFactory::contains("groovy")
        || !LiteflowConfig::new().enable
    {
        return false;
    }
    let bus = FlowBus::new();
    if bus
        .register_script(
            "groovy_case",
            "groovy",
            r#"
                def a = 3
                int b = 2
                defaultContext.setData("groovy", a * b)
            "#,
        )
        .is_err()
        || bus
            .add_chain("groovy_case_chain", "THEN(groovy_case)")
            .is_err()
    {
        return false;
    }
    let response = bus.execute("groovy_case_chain").await;
    response.is_success() && response.data_as::<i64>("groovy") == Some(6)
}
