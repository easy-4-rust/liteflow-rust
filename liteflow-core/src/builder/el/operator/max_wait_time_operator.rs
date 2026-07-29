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
        let caller = OperatorHelper::require_caller(caller, operator_name)?;
        Self::apply_to_expression(caller, max_wait_ms, operator_name)
    }

    /// 按 Java 运行时具体类型处理 maxWait，并保留先前属性所在对象。
    fn apply_to_expression(expression: El, max_wait_ms: u64, operator_name: &str) -> LFResult<El> {
        match expression {
            El::Boolean(_) => Err(LiteflowError::Parse(format!(
                "{operator_name} caller must be Executable"
            ))),
            El::Mods(inner, mods) if !mods.creates_wrapper_condition() => match *inner {
                El::When { items, mut opts } => {
                    opts.max_wait_ms = Some(max_wait_ms);
                    Ok(OperatorHelper::add_mods(El::When { items, opts }, mods))
                }
                El::Fin(_) => Err(LiteflowError::Parse(format!(
                    "FINALLY cannot use {operator_name}"
                ))),
                El::Then(items) if items.iter().any(|item| matches!(item, El::Fin(_))) => {
                    // Java handleFinally 把原 ThenCondition（含已有 id/tag/bind）
                    // 放进 TimeoutCondition，再创建只承载 FINALLY 的外层 THEN。
                    Self::wrap_then_without_finally(items, Some(mods), max_wait_ms)
                }
                inner => Ok(OperatorHelper::add_mods(
                    El::Mods(Box::new(inner), mods),
                    Mods {
                        max_wait_ms: Some(max_wait_ms),
                        ..Default::default()
                    },
                )),
            },
            El::When { items, mut opts } => {
                opts.max_wait_ms = Some(max_wait_ms);
                Ok(El::When { items, opts })
            }
            El::Fin(_) => Err(LiteflowError::Parse(format!(
                "FINALLY cannot use {operator_name}"
            ))),
            El::Then(items) if items.iter().any(|item| matches!(item, El::Fin(_))) => {
                Self::wrap_then_without_finally(items, None, max_wait_ms)
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

    /// 把 THEN 中的 FINALLY 提升到 timeout 外层。
    fn wrap_then_without_finally(
        items: Vec<El>,
        properties: Option<Mods>,
        max_wait_ms: u64,
    ) -> LFResult<El> {
        let mut timed_items = Vec::new();
        let mut finally_items = Vec::new();
        for item in items {
            if matches!(item, El::Fin(_)) {
                finally_items.push(item);
            } else {
                timed_items.push(item);
            }
        }
        let timed_body = match properties {
            Some(mods) => OperatorHelper::add_mods(El::Then(timed_items), mods),
            None => El::Then(timed_items),
        };
        let timed = OperatorHelper::add_mods(
            timed_body,
            Mods {
                max_wait_ms: Some(max_wait_ms),
                ..Default::default()
            },
        );
        let mut outer = vec![timed];
        outer.extend(finally_items);
        Ok(El::Then(outer))
    }
}
