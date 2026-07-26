//! 脚本执行器初始化完成后的生命周期扩展点。

use super::life_cycle::LifeCycle;

/// 脚本执行器初始化完成后的生命周期钩子。
///
/// 对应 Java:
/// `com.yomahub.liteflow.lifecycle.PostProcessScriptEngineInitLifeCycle`。
///
/// Java 将异构脚本引擎作为 `Object` 传入。Rust 插件的引擎类型并不共享安全的
/// 公共父类型，并且部分引擎不能跨线程传递，因此这里传递稳定的语言标识；回调
/// 发生在脚本组件完成真实构建、尚未注册为流程节点的时刻。
pub trait PostProcessScriptEngineInitLifeCycle: LifeCycle {
    /// 在脚本执行器完成初始化后执行。
    ///
    /// `language` 是注册脚本节点时使用的语言标识，例如 `rhai`、`lua` 或
    /// `javascript`。对应 Java:
    /// `PostProcessScriptEngineInitLifeCycle#postProcessAfterScriptEngineInit`。
    fn post_process_after_script_engine_init(&self, language: &str);
}
