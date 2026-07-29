use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 WHILE 操作符。
///
/// 支持布尔可执行项，也支持 Java 中由匿名 NodeBooleanComponent 实现的
/// 布尔字面量重载。DO/BREAK/PARALLEL 后缀会继续完善循环 AST。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.WhileOperator`。
pub struct WhileOperator;

impl BaseOperator for WhileOperator {
    fn operator_name(&self) -> &'static str {
        "WHILE"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let node = match objects.as_slice() {
            [Arg::Expr(node)] => node.clone(),
            [Arg::Bool(value)] => El::Boolean(*value),
            _ => {
                return Err(LiteflowError::Parse(
                    "WHILE requires exactly one boolean expression or bool".to_string(),
                ));
            }
        };
        Ok(El::While {
            node: Box::new(node),
            parallel: false,
            body: Box::new(El::Then(Vec::new())),
            brk: None,
        })
    }
}
