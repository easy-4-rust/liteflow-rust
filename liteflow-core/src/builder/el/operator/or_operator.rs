use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 表达式中的 OR 布尔操作符。
///
/// 包含至少两个能产生布尔结果的表达式；执行期由 AndOrCondition 做短路判断。
/// 参数数量与 Java `OperatorHelper#checkObjectSizeGteTwo` 保持一致。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.OrOperator`。
pub(crate) struct OrOperator;

impl BaseOperator for OrOperator {
    fn operator_name(&self) -> &'static str {
        "OR"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let expressions = OperatorHelper::expressions(objects, self.operator_name(), 2)?;
        for expression in &expressions {
            OperatorHelper::check_obj_must_be_boolean_type_item(expression)?;
        }
        Ok(El::Or(expressions))
    }
}
