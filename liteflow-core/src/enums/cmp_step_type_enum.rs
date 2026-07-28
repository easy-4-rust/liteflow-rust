//! CmpStep 步骤记录的类型标记（Java 为 START / END / SINGLE）。
//!
//! Java 语义：一个节点会产生 START（开始）与 END（结束）两条步骤；
//! 合并记录时为 SINGLE。Rust 端 CmpStep 以单条记录携带开始/结束时间，
//! 因此默认使用 SINGLE。

/// 区分组件开始、结束以及合并后的单条步骤。
///
/// 对应 Java: `com.yomahub.liteflow.enums.CmpStepTypeEnum`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpStepTypeEnum {
    /// 开始步骤
    Start,
    /// 结束步骤
    End,
    /// 合并为单条步骤
    Single,
}

/// `CmpStepTypeEnum` 的兼容类型名。
///
/// 旧版 Rust API 使用 `CmpStepType`；别名与对象定义放在同一文件，crate 根仅重导出。
pub type CmpStepType = CmpStepTypeEnum;

impl CmpStepTypeEnum {
    /// 返回 Java 侧枚举常量名。
    ///
    /// 返回值为 `START`、`END` 或 `SINGLE`，对应 Java 枚举的 `name()` 语义。
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "START",
            Self::End => "END",
            Self::Single => "SINGLE",
        }
    }
}

impl Default for CmpStepTypeEnum {
    fn default() -> Self {
        Self::Single
    }
}
