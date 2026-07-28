use std::collections::BTreeSet;

use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 MUST 指定任务并行策略操作符。
///
/// 参数可为节点 id 字符串或节点表达式；Rust 使用有序集合去重后写入 WhenOpts。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.MustOperator`。
pub struct MustOperator;

impl BaseOperator for MustOperator {
    fn operator_name(&self) -> &'static str {
        "MUST"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        if objects.is_empty() {
            return Err(LiteflowError::Parse(
                "MUST requires at least one node id".to_string(),
            ));
        }
        let mut node_ids = BTreeSet::new();
        for object in objects {
            match object {
                Arg::Str(node_id) => {
                    node_ids.insert(node_id);
                }
                Arg::Expr(El::Node(node)) => {
                    node_ids.insert(node.id);
                }
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "invalid MUST argument: {other:?}"
                    )));
                }
            }
        }
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::When { items, mut opts } => {
                opts.must = node_ids.into_iter().collect();
                Ok(El::When { items, opts })
            }
            _ => Err(LiteflowError::Parse(
                "MUST must follow WHEN/PAR".to_string(),
            )),
        }
    }
}
