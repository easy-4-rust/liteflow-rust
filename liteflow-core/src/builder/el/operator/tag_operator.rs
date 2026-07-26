use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::LFResult;

/// EL 规则中的 TAG 操作符。
///
/// Node 直接保存 tag；Condition 与运行期解析为 Chain 的引用通过属性包装
/// 保存，避免修改全局唯一的 Chain 对象。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.TagOperator`。
pub(crate) struct TagOperator;

impl BaseOperator for TagOperator {
    fn operator_name(&self) -> &'static str {
        "TAG"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let tag = OperatorHelper::one_string(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::Node(mut node) => {
                node.tag = Some(tag);
                Ok(El::Node(node))
            }
            other => Ok(OperatorHelper::add_mods(
                other,
                Mods {
                    tag: Some(tag),
                    ..Default::default()
                },
            )),
        }
    }
}
