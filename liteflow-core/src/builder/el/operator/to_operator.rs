use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, NodeRef};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 SWITCH.TO 候选目标操作符。
///
/// 字符串目标支持 `id:tag`，表达式目标保持原 AST。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ToOperator`。
pub struct ToOperator;

impl BaseOperator for ToOperator {
    fn operator_name(&self) -> &'static str {
        "TO"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let mut targets = Vec::new();
        for object in objects {
            match object {
                Arg::Expr(target) => targets.push(target),
                Arg::Str(value) => {
                    let mut parts = value.splitn(2, ':');
                    let mut node = NodeRef::new(parts.next().unwrap_or_default());
                    node.tag = parts.next().map(str::to_string);
                    targets.push(El::Node(node));
                }
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "invalid TO target: {other:?}"
                    )));
                }
            }
        }
        let caller = OperatorHelper::require_caller(caller, self.operator_name())?;
        OperatorHelper::map_through_property_mods(caller, |caller| match caller {
            El::Switch { node, default, .. } => Ok(El::Switch {
                node,
                targets,
                default,
            }),
            _ => Err(LiteflowError::Parse("TO must follow SWITCH".to_string())),
        })
    }
}
