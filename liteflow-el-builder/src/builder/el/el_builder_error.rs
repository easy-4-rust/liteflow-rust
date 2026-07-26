//! EL 构建错误。

use thiserror::Error;

/// EL 链式构建过程中的结构化错误。
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ELBuilderError {
    /// 参数类型或表达式类别不满足 Java ELBus 约束。
    #[error("EL 参数错误: {0}")]
    InvalidParameter(String),
    /// 并行选项互斥。
    #[error("EL 选项冲突: {0}")]
    ConflictingOptions(String),
    /// 必需的表达式分支缺失。
    #[error("EL 表达式不完整: {0}")]
    MissingExpression(String),
    /// data 对象无法序列化。
    #[error("EL data 序列化失败: {0}")]
    DataSerialization(String),
}
