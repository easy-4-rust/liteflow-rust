use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 ITERATOR 操作符。
///
/// 参数必须是一个迭代类型节点；DO/BREAK/PARALLEL 后缀负责补充循环行为。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IteratorOperator`。
pub(crate) struct IteratorOperator;

impl BaseOperator for IteratorOperator {
    fn operator_name(&self) -> &'static str {
        "ITERATOR"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let node = OperatorHelper::one_expression(objects, self.operator_name())?;
        OperatorHelper::check_obj_must_be_iterator_type_item(&node)?;
        Ok(El::Iter {
            node: Box::new(node),
            parallel: None,
            body: Box::new(El::Then(Vec::new())),
            brk: None,
        })
    }
}
