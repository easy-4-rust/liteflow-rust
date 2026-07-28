use super::base::BaseOperator;
use super::max_wait_time_operator::MaxWaitTimeOperator;
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 maxWaitSeconds 操作符。
///
/// 将秒转换为毫秒后交给公共超时操作符处理。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.MaxWaitSecondsOperator`。
pub struct MaxWaitSecondsOperator;

impl BaseOperator for MaxWaitSecondsOperator {
    fn operator_name(&self) -> &'static str {
        "MAX_WAIT_SECONDS"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        MaxWaitTimeOperator::build(caller, objects, 1000.0, self.operator_name())
    }
}
