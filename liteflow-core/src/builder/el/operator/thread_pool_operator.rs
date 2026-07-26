use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, Mods};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 THREAD_POOL 操作符。
///
/// 支持 WHEN、FOR、WHILE、ITERATOR 四类并发条件。Rust 运行时统一由 tokio
/// 调度，线程池类名作为调度元数据保留。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ThreadPoolOperator`。
pub(crate) struct ThreadPoolOperator;

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
