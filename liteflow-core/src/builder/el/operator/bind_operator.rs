use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 BIND 操作符。
///
/// Node 的 bind 数据直接保存在 NodeRef；Condition/Chain 通过 Mods 在构建期
/// 生成绑定包装。override=true 时构建器会清除子节点同名 bind。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.BindOperator`。
pub struct BindOperator;

impl BaseOperator for BindOperator {
    fn operator_name(&self) -> &'static str {
        "BIND"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let (key, value, override_flag) = match objects.as_slice() {
            [Arg::Str(key), Arg::Str(value)] => (key.clone(), value.clone(), false),
            [Arg::Str(key), Arg::Str(value), Arg::Bool(override_flag)] => {
                (key.clone(), value.clone(), *override_flag)
            }
            _ => {
                return Err(LiteflowError::Parse(
                    "BIND requires key, value and optional override bool".to_string(),
                ));
            }
        };
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::Node(mut node) => {
                node.bind.retain(|(existing, _)| *existing != key);
                node.bind.push((key, value));
                // Java 的 Node 分支只执行 putBindData；第四个 override 参数仅在
                // Condition 分支清理子节点绑定，不改变 Node 自身状态。
                node.bind_override = false;
                Ok(El::Node(node))
            }
            El::Boolean(_) => Err(LiteflowError::Parse(
                "BIND caller must be Executable".to_string(),
            )),
            other => Ok(OperatorHelper::add_mods(
                other,
                Mods {
                    bind: vec![(key.clone(), value)],
                    bind_override_keys: override_flag.then_some(key).into_iter().collect(),
                    ..Default::default()
                },
            )),
        }
    }
}
