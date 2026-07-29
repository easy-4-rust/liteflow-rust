use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 TAG 操作符。
///
/// Node 直接保存 tag；Condition 与运行期解析为 Chain 的引用通过属性包装
/// 保存，避免修改全局唯一的 Chain 对象。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.TagOperator`。
pub struct TagOperator;

impl BaseOperator for TagOperator {
    fn operator_name(&self) -> &'static str {
        "TAG"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let tag = OperatorHelper::one_string(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::Node(mut node) => {
                // 标识第一个 Chain 属性操作。若运行期解析为普通 Node，该字段
                // 不参与构建；若解析为 Chain，则必须创建 Java 的 ThenCondition。
                if node.tag.is_none() && node.bind.is_empty() {
                    node.chain_tag_wrapper = true;
                }
                node.tag = Some(tag);
                Ok(El::Node(node))
            }
            El::Boolean(_) => Err(LiteflowError::Parse(
                "TAG caller must be Executable".to_string(),
            )),
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
