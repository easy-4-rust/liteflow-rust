//! QLExpress 脚本与 Vernal 组合场景。

use liteflow_core::FlowBus;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use liteflow_script_qlexpress::QlExpressScriptExecutor;
use liteflow_vernal::LiteflowConfig;

/// 注册并真实执行 Java QLExpress 语句语法，验证 Vernal 配置组合。
///
/// 返回脚本引擎注册、链路构建、执行和上下文写回是否全部成功。
/// 对应 Java: `QLExpressScriptExecutor` 与 LiteFlow Vernal 启动组合。
pub async fn run_case() -> bool {
    if QlExpressScriptExecutor::register().is_err()
        || !ScriptExecutorFactory::contains("qlexpress")
        || !LiteflowConfig::new().enable
    {
        return false;
    }

    let bus = FlowBus::new();
    if bus
        .register_script_typed(
            "qlexpress_case",
            "qlexpress",
            ScriptKind::Common,
            "a = 1; b = 2; defaultContext.setData(\"qlexpress\", a + b);",
        )
        .is_err()
        || bus
            .add_chain("qlexpress_case_chain", "THEN(qlexpress_case)")
            .is_err()
    {
        return false;
    }

    let response = bus.execute("qlexpress_case_chain").await;
    response.is_success() && response.data_as::<i64>("qlexpress") == Some(3)
}
