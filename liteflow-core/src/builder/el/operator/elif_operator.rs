use super::base::{BaseOperator, OperatorHelper};
use crate::el::{Arg, El};
use crate::exception::{LFResult, LiteflowError};

/// EL 规则中的 ELIF 操作符。
///
/// 接收判断表达式和 true 分支，并追加到已有 IF 条件。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.ElifOperator`。
pub struct ElifOperator;

impl BaseOperator for ElifOperator {
    fn operator_name(&self) -> &'static str {
        "ELIF"
    }

    fn build(&self, caller: Option<El>, objects: Vec<Arg>) -> LFResult<El> {
        // Java 的 Object[] 还包含第一个 caller；Rust 已把 caller 拆成独立参数，
        // 因此这里的两个显式参数与 Java 总数三个语义等价。
        OperatorHelper::check_object_size_eq_two(&objects)?;
        let mut expressions = OperatorHelper::expressions(objects, self.operator_name(), 2)?;
        if expressions.len() != 2 {
            return Err(LiteflowError::Parse(
                "ELIF requires condition and true branch".to_string(),
            ));
        }
        let true_branch = expressions.pop().expect("长度已经校验");
        let condition = expressions.pop().expect("长度已经校验");
        match OperatorHelper::require_caller(caller, self.operator_name())? {
            El::If {
                cond,
                then,
                mut elifs,
                els,
            } => {
                elifs.push((condition, true_branch));
                Ok(El::If {
                    cond,
                    then,
                    elifs,
                    els,
                })
            }
            _ => Err(LiteflowError::Parse("ELIF must follow IF".to_string())),
        }
    }
}
