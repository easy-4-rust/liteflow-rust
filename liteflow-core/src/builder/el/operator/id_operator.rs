use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::LFResult;

/// EL 规则中的 ID 操作符。
///
/// Java v2.16 只允许 Condition 设置 id。Rust 端对条件使用通用属性包装，
/// 同时保留历史上 Node 实例别名写法，避免破坏既有规则。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IdOperator`。
pub(crate) struct IdOperator;

impl BaseOperator for IdOperator {
    fn operator_name(&self) -> &'static str {
        "ID"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let id = OperatorHelper::one_string(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::Node(mut node) => {
                node.alias = Some(id);
                Ok(El::Node(node))
            }
            other => Ok(OperatorHelper::add_mods(
                other,
                Mods {
                    id: Some(id),
                    ..Default::default()
                },
            )),
        }
    }
}
