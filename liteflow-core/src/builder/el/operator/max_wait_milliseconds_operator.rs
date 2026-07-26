use super::base::BaseOperator;
use super::max_wait_time_operator::MaxWaitTimeOperator;
use crate::el::{Arg, El};
use crate::exception::LFResult;

/// EL 规则中的 maxWaitMilliseconds 操作符。
///
/// 以毫秒为单位交给公共超时操作符处理。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.MaxWaitMillisecondsOperator`。
pub(crate) struct MaxWaitMillisecondsOperator;

impl BaseOperator for MaxWaitMillisecondsOperator {
    fn operator_name(&self) -> &'static str {
        "MAX_WAIT_MILLISECONDS"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        MaxWaitTimeOperator::build(caller, objects, 1.0, self.operator_name())
    }
}
