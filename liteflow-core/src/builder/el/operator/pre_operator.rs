use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 PRE 操作符。
///
/// 接受一个或多个普通可执行项，并构造 THEN 主流程可识别的前置流程。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.PreOperator`。
pub struct PreOperator;

impl BaseOperator for PreOperator {
    fn operator_name(&self) -> &'static str {
        "PRE"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let expressions = OperatorHelper::expressions(objects, self.operator_name(), 1)?;
        for expression in &expressions {
            OperatorHelper::check_obj_must_be_common_type_item(expression)?;
        }
        Ok(El::Pre(Box::new(El::Then(expressions))))
    }
}
