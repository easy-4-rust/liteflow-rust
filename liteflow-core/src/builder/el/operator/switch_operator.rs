use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 SWITCH 操作符。
///
/// 参数必须是一个 SWITCH 类型节点；Rust AST 在构建期保留该可执行项，
/// TO/DEFAULT 后缀负责补充目标分支。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.SwitchOperator`。
pub(crate) struct SwitchOperator;

impl BaseOperator for SwitchOperator {
    fn operator_name(&self) -> &'static str {
        "SWITCH"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let node = OperatorHelper::one_expression(objects, self.operator_name())?;
        OperatorHelper::check_obj_must_be_switch_type_item(&node)?;
        Ok(El::Switch {
            node: Box::new(node),
            targets: Vec::new(),
            default: None,
        })
    }
}
