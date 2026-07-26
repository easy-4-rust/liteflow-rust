use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 表达式中的 NOT 布尔取反操作符。
///
/// 只允许一个能产生布尔结果的表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.NotOperator`。
pub(crate) struct NotOperator;

impl BaseOperator for NotOperator {
    fn operator_name(&self) -> &'static str {
        "NOT"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        Ok(El::Not(Box::new(OperatorHelper::one_expression(
            objects,
            self.operator_name(),
        )?)))
    }
}
