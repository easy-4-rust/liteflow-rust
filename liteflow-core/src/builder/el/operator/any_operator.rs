use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 ANY 并行策略操作符。
///
/// 仅可用于 WHEN/PAR，表示任意一个任务完成即可结束并行等待。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.AnyOperator`。
pub struct AnyOperator;

impl BaseOperator for AnyOperator {
    fn operator_name(&self) -> &'static str {
        "ANY"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let any = OperatorHelper::one_bool(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::When { items, mut opts } => {
                opts.any = any;
                Ok(El::When { items, opts })
            }
            _ => Err(LiteflowError::Parse("ANY must follow WHEN/PAR".to_string())),
        }
    }
}
