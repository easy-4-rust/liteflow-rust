use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 ELSE 操作符。
///
/// 只允许跟在 IF/ELIF 之后，并接收一个普通表达式作为最终 false 分支。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ElseOperator`。
pub(crate) struct ElseOperator;

impl BaseOperator for ElseOperator {
    fn operator_name(&self) -> &'static str {
        "ELSE"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let false_branch = OperatorHelper::one_expression(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::If {
                cond, then, elifs, ..
            } => Ok(El::If {
                cond,
                then,
                elifs,
                els: Some(Box::new(false_branch)),
            }),
            _ => Err(LiteflowError::Parse("ELSE must follow IF/ELIF".to_string())),
        }
    }
}
