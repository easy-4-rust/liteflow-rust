use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 PERCENTAGE 并行阈值操作符。
///
/// 仅可用于 WHEN/PAR，阈值范围为 0 到 1。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.PercentageOperator`。
pub struct PercentageOperator;

impl BaseOperator for PercentageOperator {
    fn operator_name(&self) -> &'static str {
        "PERCENTAGE"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let percentage = OperatorHelper::one_number(objects, self.operator_name())?;
        if !(0.0..=1.0).contains(&percentage) {
            return Err(LiteflowError::Parse(
                "PERCENTAGE must be between 0 and 1".to_string(),
            ));
        }
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::When { items, mut opts } => {
                opts.percentage = Some(percentage);
                Ok(El::When { items, opts })
            }
            _ => Err(LiteflowError::Parse(
                "PERCENTAGE must follow WHEN/PAR".to_string(),
            )),
        }
    }
}
