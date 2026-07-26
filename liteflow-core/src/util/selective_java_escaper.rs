//! 选择性 Java 字符串转义。
//!
//! 对应 Java: `com.yomahub.liteflow.util.SelectiveJavaEscaper`。

/// 只转义 Java 字符串字面量所需字符，保留中文等非 ASCII 字符。
pub struct SelectiveJavaEscaper;

impl SelectiveJavaEscaper {
    /// 对输入应用选择性 Java 转义规则。
    ///
    /// `None` 原样返回 `None`；双引号、反斜杠以及 `\n\t\r\f\b`
    /// 被转义，其他 Unicode 字符保持不变。对应 Java:
    /// `SelectiveJavaEscaper#escape`。
    #[must_use]
    pub fn escape(input: Option<&str>) -> Option<String> {
        input.map(|input| {
            let mut escaped = String::with_capacity(input.len() + input.len() / 2);
            for character in input.chars() {
                match character {
                    '"' => escaped.push_str("\\\""),
                    '\\' => escaped.push_str("\\\\"),
                    '\n' => escaped.push_str("\\n"),
                    '\t' => escaped.push_str("\\t"),
                    '\r' => escaped.push_str("\\r"),
                    '\u{000c}' => escaped.push_str("\\f"),
                    '\u{0008}' => escaped.push_str("\\b"),
                    other => escaped.push(other),
                }
            }
            escaped
        })
    }
}
