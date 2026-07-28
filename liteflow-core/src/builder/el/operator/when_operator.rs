use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El, WhenOpts};
use crate::exception::LFResult;

/// EL 规则中的 WHEN 并行操作符。
///
/// 参数必须是一个或多个普通可执行表达式，私有并行策略由后缀操作符设置。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.WhenOperator`。
pub struct WhenOperator;

impl BaseOperator for WhenOperator {
    fn operator_name(&self) -> &'static str {
        "WHEN"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        OperatorHelper::require_primary(caller, self.operator_name())?;
        let items = OperatorHelper::expressions(objects, self.operator_name(), 1)?;
        for item in &items {
            OperatorHelper::check_obj_must_be_common_type_item(item)?;
        }
        Ok(El::When {
            items,
            opts: WhenOpts::default(),
        })
    }
}
