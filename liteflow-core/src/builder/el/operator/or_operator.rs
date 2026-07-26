use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 表达式中的 OR 布尔操作符。
///
/// 包含一个或多个能产生布尔结果的表达式；执行期由 AndOrCondition 做短路判断。
/// Java v2.16 要求至少两个参数，Rust 端保留历史已验证的单参数兼容形式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.OrOperator`。
pub(crate) struct OrOperator;

impl BaseOperator for OrOperator {
    fn operator_name(&self) -> &'static str {
        "OR"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        Ok(El::Or(OperatorHelper::expressions(
            objects,
            self.operator_name(),
            1,
        )?))
    }
}
