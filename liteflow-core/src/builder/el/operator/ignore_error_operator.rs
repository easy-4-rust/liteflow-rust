use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 IGNORE_ERROR 操作符。
///
/// 仅允许 WHEN/PAR 设置并行错误策略；Java 会把其他调用方转换
/// `WhenCondition` 失败并抛出 ELParseException。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IgnoreErrorOperator`。
pub struct IgnoreErrorOperator;

impl BaseOperator for IgnoreErrorOperator {
    fn operator_name(&self) -> &'static str {
        "IGNORE_ERROR"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let ignore_error = OperatorHelper::one_bool(objects, self.operator_name())?;
        let caller = OperatorHelper::require_caller(caller, self.operator_name())?;
        OperatorHelper::map_through_property_mods(caller, |caller| match caller {
            El::When { items, mut opts } => {
                opts.ignore_error = ignore_error;
                Ok(El::When { items, opts })
            }
            _ => Err(LiteflowError::Parse(
                "The caller must be WhenCondition item".to_string(),
            )),
        })
    }
}
