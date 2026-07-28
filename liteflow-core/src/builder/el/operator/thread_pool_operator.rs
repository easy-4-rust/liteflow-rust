use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 THREAD_POOL 操作符。
///
/// 支持 WHEN、FOR、WHILE、ITERATOR 四类并发条件。线程池构建器名称会进入
/// `WhenOpts` 或循环 `Mods`，构建后由 `ExecutorHelper` 按
/// Condition > Chain > 全局优先级选择并缓存真实有界 Tokio 执行器。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ThreadPoolOperator`。
pub struct ThreadPoolOperator;

impl BaseOperator for ThreadPoolOperator {
    fn operator_name(&self) -> &'static str {
        "THREAD_POOL"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        let thread_pool = OperatorHelper::one_string(objects, self.operator_name())?;
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::When { items, mut opts } => {
                opts.thread_pool = Some(thread_pool);
                Ok(El::When { items, opts })
            }
            loop_expression @ (El::For { .. }
            | El::ForCount { .. }
            | El::While { .. }
            | El::Iter { .. }) => Ok(OperatorHelper::add_mods(
                loop_expression,
                Mods {
                    thread_pool: Some(thread_pool),
                    ..Default::default()
                },
            )),
            _ => Err(LiteflowError::Parse(
                "THREAD_POOL must follow WHEN/FOR/WHILE/ITERATOR".to_string(),
            )),
        }
    }
}
