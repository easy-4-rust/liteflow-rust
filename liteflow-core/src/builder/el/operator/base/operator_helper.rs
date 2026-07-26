use crate::el::{Arg, El, Mods, NodeRef};
use crate::exception::{LFResult, LiteflowError};

/// EL 操作符参数校验与转换助手。
///
/// Java 版负责 `Object[]` 数量检查、Class 转换和布尔/普通表达式校验；
/// Rust 版利用 `Arg` 枚举消除 Class 强转，并集中生成一致的错误信息。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.base.OperatorHelper`。
pub(crate) struct OperatorHelper;

impl OperatorHelper {
    /// 校验调用者必须为空，即当前操作符只能作为主表达式使用。
    pub(crate) fn require_primary(caller: Option<El>, operator: &str) -> LFResult<()> {
        if caller.is_some() {
            return Err(LiteflowError::Parse(format!(
                "{operator} must be used as a primary expression"
            )));
        }
        Ok(())
    }

    /// 读取后缀操作符的左侧调用表达式。
    pub(crate) fn require_caller(caller: Option<El>, operator: &str) -> LFResult<El> {
        caller.ok_or_else(|| {
            LiteflowError::Parse(format!("{operator} must follow an executable expression"))
        })
    }

    /// 把全部参数转换为表达式，并校验最小数量。
    ///
    /// 字符串参数按节点引用转换，与 Java QLExpress 的 Executable 参数一致。
    pub(crate) fn expressions(
        objects: Vec<Arg>,
        operator: &str,
        minimum: usize,
    ) -> LFResult<Vec<El>> {
        let mut expressions = Vec::with_capacity(objects.len());
        for object in objects {
            match object {
                Arg::Expr(expression) => expressions.push(expression),
                Arg::Str(node_id) => expressions.push(El::Node(NodeRef::new(node_id))),
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "{operator} requires expression arguments, got {other:?}"
                    )));
                }
            }
        }
        if expressions.len() < minimum {
            return Err(LiteflowError::Parse(format!(
                "{operator} requires at least {minimum} expression argument(s)"
            )));
        }
        Ok(expressions)
    }

    /// 读取唯一表达式参数。
    pub(crate) fn one_expression(objects: Vec<Arg>, operator: &str) -> LFResult<El> {
        let mut expressions = Self::expressions(objects, operator, 1)?;
        if expressions.len() != 1 {
            return Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one expression"
            )));
        }
        Ok(expressions.remove(0))
    }

    /// 读取唯一字符串参数。
    pub(crate) fn one_string(objects: Vec<Arg>, operator: &str) -> LFResult<String> {
        match objects.as_slice() {
            [Arg::Str(value)] => Ok(value.clone()),
            _ => Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one string"
            ))),
        }
    }

    /// 读取唯一布尔参数。
    pub(crate) fn one_bool(objects: Vec<Arg>, operator: &str) -> LFResult<bool> {
        match objects.as_slice() {
            [Arg::Bool(value)] => Ok(*value),
            _ => Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one bool"
            ))),
        }
    }

    /// 读取唯一数字参数。
    pub(crate) fn one_number(objects: Vec<Arg>, operator: &str) -> LFResult<f64> {
        match objects.as_slice() {
            [Arg::Num(value)] => Ok(*value),
            _ => Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one number"
            ))),
        }
    }

    /// 合并通用修饰，避免多次后缀调用形成无意义的嵌套 Mods。
    pub(crate) fn add_mods(expression: El, mods: Mods) -> El {
        match expression {
            El::Mods(inner, mut old) => {
                if mods.id.is_some() {
                    old.id = mods.id;
                }
                if mods.tag.is_some() {
                    old.tag = mods.tag;
                }
                if mods.thread_pool.is_some() {
                    old.thread_pool = mods.thread_pool;
                }
                if mods.retry.is_some() {
                    old.retry = mods.retry;
                }
                if !mods.retry_for.is_empty() {
                    old.retry_for = mods.retry_for;
                }
                if mods.max_wait_ms.is_some() {
                    old.max_wait_ms = mods.max_wait_ms;
                }
                old.ignore_error = old.ignore_error || mods.ignore_error;
                if !mods.bind.is_empty() {
                    for (key, value) in mods.bind {
                        old.bind.retain(|(existing, _)| *existing != key);
                        old.bind.push((key, value));
                    }
                }
                old.bind_override = old.bind_override || mods.bind_override;
                El::Mods(inner, old)
            }
            other => El::Mods(Box::new(other), mods),
        }
    }
}
