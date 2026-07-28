use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::LFResult;

/// EL 规则中的 IGNORE_ERROR 操作符。
///
/// WHEN/PAR 直接设置并行错误策略，其他表达式包装为 IgnoreErrorCondition。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IgnoreErrorOperator`。
pub struct IgnoreErrorOperator;

impl BaseOperator for IgnoreErrorOperator {
    fn operator_name(&self) -> &'static str {
        "IGNORE_ERROR"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let ignore_error = OperatorHelper::one_bool(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::When { items, mut opts } => {
                opts.ignore_error = ignore_error;
                Ok(El::When { items, opts })
            }
            other => Ok(OperatorHelper::add_mods(
                other,
                Mods {
                    ignore_error,
                    ..Default::default()
                },
            )),
        }
    }
}
