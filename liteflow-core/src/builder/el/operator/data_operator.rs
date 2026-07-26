use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 DATA 操作符。
///
/// Java 使用 LiteflowMetaOperator 获取调用表达式中的全部 Node 并设置
/// cmpData；Rust 端在 AST 上递归执行相同操作。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.DataOperator`。
pub(crate) struct DataOperator;

impl BaseOperator for DataOperator {
    fn operator_name(&self) -> &'static str {
        "DATA"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let data = OperatorHelper::one_string(objects, self.operator_name())?;
        let mut expression = OperatorHelper::require_caller(caller, self.operator_name())?;
        set_node_data(&mut expression, &data);
        Ok(expression)
    }
}

/// 递归设置表达式中的所有节点数据。
/// 对应 Java: `LiteflowMetaOperator#getNodes` + `Node#setCmpData`。
fn set_node_data(expression: &mut El, data: &str) {
    match expression {
        El::Node(node) => node.data = Some(data.to_string()),
        El::Boolean(_) => {}
        El::Then(items) | El::And(items) | El::Or(items) => {
            items.iter_mut().for_each(|item| set_node_data(item, data));
        }
        El::When { items, .. } => {
            items.iter_mut().for_each(|item| set_node_data(item, data));
        }
        El::If {
            cond,
            then,
            elifs,
            els,
        } => {
            set_node_data(cond, data);
            set_node_data(then, data);
            elifs.iter_mut().for_each(|(condition, executable)| {
                set_node_data(condition, data);
                set_node_data(executable, data);
            });
            if let Some(executable) = els {
                set_node_data(executable, data);
            }
        }
        El::Switch {
            node,
            targets,
            default,
        } => {
            set_node_data(node, data);
            targets
                .iter_mut()
                .for_each(|target| set_node_data(target, data));
            if let Some(executable) = default {
                set_node_data(executable, data);
            }
        }
        El::For {
            node, body, brk, ..
        }
        | El::While {
            node, body, brk, ..
        }
        | El::Iter {
            node, body, brk, ..
        } => {
            set_node_data(node, data);
            set_node_data(body, data);
            if let Some(executable) = brk {
                set_node_data(executable, data);
            }
        }
        El::ForCount { body, brk, .. } => {
            set_node_data(body, data);
            if let Some(executable) = brk {
                set_node_data(executable, data);
            }
        }
        El::Catch { body, do_ } => {
            set_node_data(body, data);
            if let Some(executable) = do_ {
                set_node_data(executable, data);
            }
        }
        El::Not(inner) | El::Pre(inner) | El::Fin(inner) | El::Mods(inner, _) => {
            set_node_data(inner, data);
        }
    }
}
