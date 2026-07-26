//! 本地流程默认常量。
//!
//! 对应 Java: `com.yomahub.liteflow.common.LocalDefaultFlowConstant`。

/// 保存本地执行默认分组名。
pub struct LocalDefaultFlowConstant;

impl LocalDefaultFlowConstant {
    /// 默认分组名，对应 Java `DEFAULT`。
    pub const DEFAULT: &'static str = "default";
}
