use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, NodeRef};
use crate::exception::LFResult;

/// EL 规则中的显式 NODE 操作符。
///
/// 将一个字符串节点 id 转换为 NodeRef。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.NodeOperator`。
pub struct NodeOperator;

impl BaseOperator for NodeOperator {
    fn operator_name(&self) -> &'static str {
        "NODE"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        Ok(El::Node(NodeRef::new(OperatorHelper::one_string(
            objects,
            self.operator_name(),
        )?)))
    }
}
