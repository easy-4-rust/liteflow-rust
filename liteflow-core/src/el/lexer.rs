//! LiteFlow EL 解析错误格式化。
//!
//! 词法分析已经由发布版 QlExpress Rust 承担；这里只保留
//! `LiteFlowChainELBuilder` 生成对象级校验诊断所需的源码定位逻辑。

use crate::exception::LiteflowError;

/// 构造包含行、列、EL 原文和 `^` 指示符的解析错误。
///
/// `character_index` 是 Unicode 字符偏移而不是字节偏移，避免中文节点名之前的
/// 多字节字符导致列号错位。对应 Java:
/// `LiteFlowChainELBuilder#buildDataNotFoundExceptionMsg`。
pub(crate) fn format_el_parse_error(
    source: &str,
    character_index: usize,
    detail: impl AsRef<str>,
) -> LiteflowError {
    let characters: Vec<char> = source.chars().collect();
    let bounded_index = character_index.min(characters.len());
    let line_start = characters[..bounded_index]
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |index| index + 1);
    let line_end = characters[bounded_index..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(characters.len(), |offset| bounded_index + offset);
    let line = characters[..bounded_index]
        .iter()
        .filter(|character| **character == '\n')
        .count()
        + 1;
    let column = bounded_index - line_start + 1;
    let source_line: String = characters[line_start..line_end].iter().collect();
    LiteflowError::Parse(format!(
        "{} at line {line}, column {column}\n EL: {source_line}\n{}^",
        detail.as_ref(),
        " ".repeat(column + 4)
    ))
}
