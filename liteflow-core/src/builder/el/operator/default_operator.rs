use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 SWITCH.DEFAULT 操作符。
///
/// 只允许跟在 SWITCH/TO 后，并设置一个默认目标。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.DefaultOperator`。
pub(crate) struct DefaultOperator;

impl BaseOperator for DefaultOperator {
    fn operator_name(&self) -> &'static str {
        "DEFAULT"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let default = OperatorHelper::one_expression(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::Switch { node, targets, .. } => Ok(El::Switch {
                node,
                targets,
                default: Some(Box::new(default)),
            }),
            _ => Err(LiteflowError::Parse(
                "DEFAULT must follow SWITCH/TO".to_string(),
            )),
        }
    }
}
