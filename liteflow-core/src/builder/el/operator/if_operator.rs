use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 IF 操作符。
///
/// 第一个参数为布尔可执行项，第二个参数为 true 分支，第三个可选参数为
/// false 分支。后续还可通过 ELIF/ELSE 追加分支。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IfOperator`。
pub struct IfOperator;

impl BaseOperator for IfOperator {
    fn operator_name(&self) -> &'static str {
        "IF"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let mut items = OperatorHelper::expressions(objects, self.operator_name(), 2)?;
        if !(2..=3).contains(&items.len()) {
            return Err(LiteflowError::Parse(
                "IF requires exactly two or three expressions".to_string(),
            ));
        }
        OperatorHelper::check_obj_must_be_boolean_type_item(&items[0])?;
        OperatorHelper::check_obj_must_be_common_type_item(&items[1])?;
        if let Some(false_case) = items.get(2) {
            OperatorHelper::check_obj_must_be_common_type_item(false_case)?;
        }
        let els = if items.len() == 3 {
            Some(Box::new(items.remove(2)))
        } else {
            None
        };
        let then = Box::new(items.remove(1));
        let cond = Box::new(items.remove(0));
        Ok(El::If {
            cond,
            then,
            elifs: Vec::new(),
            els,
        })
    }
}
