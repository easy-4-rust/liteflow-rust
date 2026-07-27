//! Aviator 脚本与 Vernal 组合场景。

use liteflow_core::FlowBus;
use liteflow_core::script::ScriptExecutorFactory;
use liteflow_script_aviator::AviatorScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册并真实执行 LiteFlow Java Aviator 基线脚本。
///
/// 返回执行器注册、Vernal 配置、链路执行和上下文写回是否全部成功。
/// 对应 Java: `AviatorScriptExecutor` 的 common testcase。
pub async fn run_case() -> bool {
    if AviatorScriptExecutor::register().is_err()
        || !ScriptExecutorFactory::contains("aviator")
        || !LiteflowConfig::new().enable
    {
        return false;
    }
    let bus = FlowBus::new();
    if bus
        .register_script(
            "aviator_case",
            "aviator",
            r#"
                a = 2;
                b = 3;
                setData(defaultContext, "aviator", a*b);
            "#,
        )
        .is_err()
        || bus
            .add_chain("aviator_case_chain", "THEN(aviator_case)")
            .is_err()
    {
        return false;
    }
    let response = bus.execute("aviator_case_chain").await;
    response.is_success() && response.data_as::<i64>("aviator") == Some(6)
}
