use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 FOR 操作符。
///
/// 支持 FOR 类型节点，也支持 Java v2.16 的整数固定次数重载。DO/BREAK/
/// PARALLEL 后缀会继续完善循环 AST。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ForOperator`。
pub(crate) struct ForOperator;

impl BaseOperator for ForOperator {
    fn operator_name(&self) -> &'static str {
        "FOR"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        OperatorHelper::check_args_not_null(&objects)?;
        OperatorHelper::check_object_size_eq_one(&objects)?;
        match objects.as_slice() {
            [Arg::Expr(node)] => {
                OperatorHelper::check_obj_must_be_for_type_item(node)?;
                Ok(El::For {
                    node: Box::new(node.clone()),
                    parallel: None,
                    body: Box::new(El::Then(Vec::new())),
                    brk: None,
                })
            }
            [Arg::Num(count)] if *count >= 0.0 && count.fract() == 0.0 => Ok(El::ForCount {
                count: *count as usize,
                parallel: None,
                body: Box::new(El::Then(Vec::new())),
                brk: None,
            }),
            _ => Err(LiteflowError::Parse(
                "FOR requires exactly one node or non-negative integer".to_string(),
            )),
        }
    }
}
