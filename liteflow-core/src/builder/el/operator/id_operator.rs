use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 ID 操作符。
///
/// 仅允许 Condition 设置 id；Node、Chain 引用及布尔字面量均拒绝该操作。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IdOperator`。
pub struct IdOperator;

impl BaseOperator for IdOperator {
    fn operator_name(&self) -> &'static str {
        "ID"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let id = OperatorHelper::one_string(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            // tag/bind 会把 Chain 引用包装成 Condition。解析阶段 Node 与 Chain
            // 都是 NodeRef，先暂存 ID，Builder 解析注册表后对普通 Node 拒绝、
            // 对已包装 Chain 应用 Condition ID。
            El::Node(mut node) if node.tag.is_some() || !node.bind.is_empty() => {
                node.condition_id = Some(id);
                Ok(El::Node(node))
            }
            El::Node(_) | El::Boolean(_) => Err(LiteflowError::Parse(
                "The caller must be Condition item".to_string(),
            )),
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
