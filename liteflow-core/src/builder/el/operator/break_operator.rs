use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 BREAK 操作符。
///
/// 只允许用于 FOR/WHILE/ITERATOR，并接收一个布尔表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.BreakOperator`。
pub struct BreakOperator;

impl BaseOperator for BreakOperator {
    fn operator_name(&self) -> &'static str {
        "BREAK"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let brk = Some(Box::new(OperatorHelper::one_expression(
            objects,
            self.operator_name(),
        )?));
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::For {
                node,
                parallel,
                body,
                ..
            } => Ok(El::For {
                node,
                parallel,
                body,
                brk,
            }),
            El::ForCount {
                count,
                parallel,
                body,
                ..
            } => Ok(El::ForCount {
                count,
                parallel,
                body,
                brk,
            }),
            El::While {
                node,
                parallel,
                body,
                ..
            } => Ok(El::While {
                node,
                parallel,
                body,
                brk,
            }),
            El::Iter {
                node,
                parallel,
                body,
                ..
            } => Ok(El::Iter {
                node,
                parallel,
                body,
                brk,
            }),
            _ => Err(LiteflowError::Parse(
                "BREAK must follow FOR/WHILE/ITERATOR".to_string(),
            )),
        }
    }
}
