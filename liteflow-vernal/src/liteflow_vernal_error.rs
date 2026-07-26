//! LiteFlow Vernal 桥接层错误对象。

use thiserror::Error;

/// 配置、组件装配或规则初始化失败。
#[derive(Debug, Error)]
pub enum LiteflowVernalError {
    /// 规则文件和内联规则不能同时配置。
    #[error("liteflow rule_source and inline_rule cannot both be configured")]
    ConflictingRuleSource,
    /// 组件注册失败。
    #[error("liteflow component[{component_id}] registration failed: {message}")]
    ComponentRegistration {
        /// 组件 id。
        component_id: String,
        /// 脱离具体错误类型后的诊断。
        message: String,
    },
    /// 规则解析或装载失败。
    #[error("liteflow rule initialization failed: {0}")]
    RuleInitialization(String),
}
