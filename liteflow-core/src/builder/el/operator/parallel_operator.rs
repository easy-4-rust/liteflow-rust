use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 PARALLEL 循环并行操作符。
///
/// Java 只接受布尔值，并直接写入 LoopCondition.parallel。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ParallelOperator`。
pub struct ParallelOperator;

impl BaseOperator for ParallelOperator {
    fn operator_name(&self) -> &'static str {
        "PARALLEL"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let parallel = match objects.as_slice() {
            [Arg::Bool(parallel)] => *parallel,
            _ => {
                return Err(LiteflowError::Parse(
                    "PARALLEL requires one boolean".to_string(),
                ));
            }
        };
        let caller = OperatorHelper::require_caller(caller, self.operator_name())?;
        OperatorHelper::map_through_property_mods(caller, |caller| match caller {
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
        })
    }
}
