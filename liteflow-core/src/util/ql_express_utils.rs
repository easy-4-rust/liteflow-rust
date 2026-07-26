//! 对应 Java: com.yomahub.liteflow.util.QlExpressUtils

use crate::el::{El, parse_el};
use crate::exception::LFResult;

/// LiteFlow EL 解析与变量名校验工具。
///
/// Java 的两个 QLExpress Runner 在 Rust 中由无共享可变状态的原生解析器替代。
pub struct QlExpressUtils;

impl QlExpressUtils {
    /// 使用注册了全部 LiteFlow 操作符的 Rust EL 解析器解析表达式。
    pub fn parse_el(expression: &str) -> LFResult<El> {
        parse_el(expression)
    }

    /// 检查变量名是否符合 Java 标识符语义。
    ///
    /// 首字符必须是字母、下划线或美元符号；后续字符还可包含数字。
    #[must_use]
    pub fn check_variable_name(variable_name: &str) -> bool {
        let mut characters = variable_name.chars();
        matches!(
            characters.next(),
            Some(character) if character.is_alphabetic() || character == '_' || character == '$'
        ) && characters
            .all(|character| character.is_alphanumeric() || character == '_' || character == '$')
    }
}
