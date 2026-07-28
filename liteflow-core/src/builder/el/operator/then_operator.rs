use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 THEN 串行操作符。
///
/// 参数必须是一个或多个普通可执行表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ThenOperator`。
pub struct ThenOperator;

impl BaseOperator for ThenOperator {
    fn operator_name(&self) -> &'static str {
        "THEN"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let expressions = OperatorHelper::expressions(objects, self.operator_name(), 1)?;
        for expression in &expressions {
            OperatorHelper::check_obj_must_be_common_type_item(expression)?;
        }
        Ok(El::Then(expressions))
    }
}
