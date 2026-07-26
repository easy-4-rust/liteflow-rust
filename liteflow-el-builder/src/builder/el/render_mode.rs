//! EL 包装器渲染模式。

/// 包装器内部渲染模式。
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Java 完整语句。
    JavaStatement,
    /// Rust 执行期表达式。
    RuntimeExpression,
}
