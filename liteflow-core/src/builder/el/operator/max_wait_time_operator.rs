use super::base::OperatorHelper;
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// maxWaitSeconds 与 maxWaitMilliseconds 的公共操作符逻辑。
///
/// 对 WHEN/PAR 直接设置等待时间；FINALLY 禁止设置；当 THEN 含 FINALLY 时，
/// 只给其普通/PRE 部分增加超时，FINALLY 被提升到外层，保持 Java 的清理保障。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.MaxWaitTimeOperator`。
pub struct MaxWaitTimeOperator;

impl MaxWaitTimeOperator {
    /// 按给定毫秒倍率构建超时 AST。
    pub(crate) fn build(
        caller: Option<El>,
        objects: Vec<Arg>,
        multiplier: f64,
        operator_name: &str,
    ) -> LFResult<El> {
        let value = OperatorHelper::one_number(objects, operator_name)?;
        if value < 0.0 {
            return Err(LiteflowError::Parse(format!(
                "{operator_name} cannot be negative"
            )));
        }
        let max_wait_ms = (value * multiplier) as u64;
        match OperatorHelper::require_caller(caller, operator_name)? {
            El::When { items, mut opts } => {
                opts.max_wait_ms = Some(max_wait_ms);
                Ok(El::When { items, opts })
            }
            El::Fin(_) => Err(LiteflowError::Parse(format!(
                "FINALLY cannot use {operator_name}"
            ))),
            El::Then(items) if items.iter().any(|item| matches!(item, El::Fin(_))) => {
                let mut timed_items = Vec::new();
                let mut finally_items = Vec::new();
                for item in items {
                    if matches!(item, El::Fin(_)) {
                        finally_items.push(item);
                    } else {
                        timed_items.push(item);
                    }
                }
                let timed = OperatorHelper::add_mods(
                    El::Then(timed_items),
                    Mods {
                        max_wait_ms: Some(max_wait_ms),
                        ..Default::default()
                    },
                );
                let mut outer = vec![timed];
                outer.extend(finally_items);
                Ok(El::Then(outer))
            }
            other => Ok(OperatorHelper::add_mods(
                other,
                Mods {
                    max_wait_ms: Some(max_wait_ms),
                    ..Default::default()
                },
            )),
        }
    }
}
