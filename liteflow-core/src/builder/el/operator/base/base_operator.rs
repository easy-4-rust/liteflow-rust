use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 操作符公共构建协议。
///
/// Java 版通过 QLExpress 把 `Object[]` 传给 `build`；Rust 版以类型化的
/// `Arg` 和可选调用者 AST 代替动态数组，同时保留“一操作符一构建器”的职责。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.base.BaseOperator`。
pub(crate) trait BaseOperator {
    /// 返回 EL 关键字名称。对应 Java: `BaseOperator#operatorName`。
    fn operator_name(&self) -> &'static str;

    /// 根据调用者和参数构建 AST。
    ///
    /// # 参数
    /// - `caller`: 后缀操作符的左侧表达式；主表达式操作符为 `None`。
    /// - `objects`: 解析后的类型化参数。
    ///
    /// # 返回
    /// 构建完成的 EL AST，或精确的解析错误。
    /// 对应 Java: `BaseOperator#build(Object[])`。
    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El>;
}
