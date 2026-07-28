use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 PARALLEL 循环并行操作符。
///
/// Java 接受布尔值；Rust 兼容历史的数字并行度写法，当前执行器以 Option
/// 判断是否并行，数值保留在 AST 中供后续受限并发实现使用。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ParallelOperator`。
pub struct ParallelOperator;

impl BaseOperator for ParallelOperator {
    fn operator_name(&self) -> &'static str {
        "PARALLEL"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let parallel = match objects.as_slice() {
            [Arg::Num(value)] if *value >= 0.0 => Some(*value as usize),
            [Arg::Bool(true)] => Some(0),
            [Arg::Bool(false)] => None,
            _ => {
                return Err(LiteflowError::Parse(
                    "PARALLEL requires a non-negative number or bool".to_string(),
                ));
            }
        };
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::For {
                node, body, brk, ..
            } => Ok(El::For {
                node,
                parallel,
                body,
                brk,
            }),
            El::ForCount {
                count, body, brk, ..
            } => Ok(El::ForCount {
                count,
                parallel,
                body,
                brk,
            }),
            El::While {
                node, body, brk, ..
            } => Ok(El::While {
                node,
                parallel,
                body,
                brk,
            }),
            El::Iter {
                node, body, brk, ..
            } => Ok(El::Iter {
                node,
                parallel,
                body,
                brk,
            }),
            _ => Err(LiteflowError::Parse(
                "PARALLEL must follow FOR/WHILE/ITERATOR".to_string(),
            )),
        }
    }
}
