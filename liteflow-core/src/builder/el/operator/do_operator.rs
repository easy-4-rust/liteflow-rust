use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 DO 操作符。
///
/// 支持 FOR/WHILE/ITERATOR 循环体和 CATCH 降级体两类调用。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.DoOperator`。
pub(crate) struct DoOperator;

impl BaseOperator for DoOperator {
    fn operator_name(&self) -> &'static str {
        "DO"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let body = Box::new(OperatorHelper::one_expression(
            objects,
            self.operator_name(),
        )?);
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::For {
                node,
                parallel,
                brk,
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
                brk,
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
                brk,
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
                brk,
                ..
            } => Ok(El::Iter {
                node,
                parallel,
                body,
                brk,
            }),
            El::Catch { body: caught, .. } => Ok(El::Catch {
                body: caught,
                do_: Some(body),
            }),
            _ => Err(LiteflowError::Parse(
                "DO must follow FOR/WHILE/ITERATOR/CATCH".to_string(),
            )),
        }
    }
}
