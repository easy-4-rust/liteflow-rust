use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 RETRY 操作符。
///
/// 第一个参数为重试次数，后续可选参数为 Java 全限定异常名或 Rust 错误变体名。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.RetryOperator`。
pub struct RetryOperator;

impl BaseOperator for RetryOperator {
    fn operator_name(&self) -> &'static str {
        "RETRY"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let mut objects = objects.into_iter();
        let retry = match objects.next() {
            Some(Arg::Num(value)) if value >= 0.0 && value.fract() == 0.0 => value as u32,
            _ => {
                return Err(LiteflowError::Parse(
                    "RETRY requires a non-negative integer".to_string(),
                ));
            }
        };
        let mut retry_for = Vec::new();
        for object in objects {
            match object {
                Arg::Str(exception) => retry_for.push(exception),
                _ => {
                    return Err(LiteflowError::Parse(
                        "RETRY exception filters must be strings".to_string(),
                    ));
                }
            }
        }
        let caller = OperatorHelper::require_caller(caller, self.operator_name())?;
        Ok(OperatorHelper::add_mods(
            caller,
            Mods {
                retry: Some(retry),
                retry_for,
                ..Default::default()
            },
        ))
    }
}
