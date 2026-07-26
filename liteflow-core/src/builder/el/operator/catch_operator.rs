use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 CATCH 异常捕获操作符。
///
/// CATCH 只允许一个普通表达式，DO 后缀负责设置异常后的降级表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.CatchOperator`。
pub(crate) struct CatchOperator;

impl BaseOperator for CatchOperator {
    fn operator_name(&self) -> &'static str {
        "CATCH"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        Ok(El::Catch {
            body: Box::new(OperatorHelper::one_expression(
                objects,
                self.operator_name(),
            )?),
            do_: None,
        })
    }
}
