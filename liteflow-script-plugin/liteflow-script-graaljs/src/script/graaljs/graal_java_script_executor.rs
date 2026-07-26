//! 对应 Java: `com.yomahub.liteflow.script.graaljs.GraalJavaScriptExecutor`。

use liteflow_core::LFResult;
use liteflow_script_javascript::JavaScriptExecutor;

/// GraalJS 语言入口。
///
/// Rust 端不嵌入 JVM/GraalVM，而是把 `graaljs` 语言键注册到同样隔离的
/// Boa ECMAScript 运行时。该映射保留 JavaScript 语义与沙箱边界，但不宣称
/// 支持 GraalVM 的宿主对象互操作扩展。
pub struct GraalJavaScriptExecutor;

impl GraalJavaScriptExecutor {
    /// 注册 `language = "graaljs"`。
    ///
    /// 对应 Java `GraalJavaScriptExecutor` 被 `ScriptExecutorFactory` 发现。
    pub fn register() -> LFResult<()> {
        JavaScriptExecutor::register_language("graaljs")
    }
}
