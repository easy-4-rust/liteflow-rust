use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 ID 操作符。
///
/// 仅允许 Condition 设置 id；Node、Chain 引用及布尔字面量均拒绝该操作。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.IdOperator`。
pub(crate) struct IdOperator;

impl BaseOperator for IdOperator {
    fn operator_name(&self) -> &'static str {
        "ID"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let id = OperatorHelper::one_string(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            // Java IdOperator 会把调用方强制转换为 Condition；Node/Chain 在这里
            // 都表现为 NodeRef，因此必须统一拒绝，不能再把 id 当作节点别名。
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
