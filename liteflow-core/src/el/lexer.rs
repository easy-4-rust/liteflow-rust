//! LiteFlow EL 词法分析。

use crate::exception::{LFResult, LiteflowError};

use super::Tok;

/// 带源码字符偏移的词法 token。
pub(crate) type SpannedTok = (Tok, usize);

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

/// 把 EL 文本转换为携带源码字符偏移的递归下降解析器 token。
pub(crate) fn lex(source: &str) -> LFResult<Vec<SpannedTok>> {
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < chars.len() {
        let character = chars[index];
        match character {
            character if character.is_whitespace() => index += 1,
            '(' => {
                tokens.push((Tok::LP, index));
                index += 1;
            }
            ')' => {
                tokens.push((Tok::RP, index));
                index += 1;
            }
            ',' => {
                tokens.push((Tok::Comma, index));
                index += 1;
            }
            '.' => {
                tokens.push((Tok::Dot, index));
                index += 1;
            }
            '"' | '\'' => {
                let start = index;
                let quote = character;
                index += 1;
                let mut value = String::new();
                while index < chars.len() && chars[index] != quote {
                    if chars[index] == '\\' && index + 1 < chars.len() {
                        index += 1;
                        match chars[index] {
                            'n' => value.push('\n'),
                            't' => value.push('\t'),
                            other => value.push(other),
                        }
                    } else {
                        value.push(chars[index]);
                    }
                    index += 1;
                }
                if index >= chars.len() {
                    return Err(format_el_parse_error(
                        source,
                        start,
                        "unclosed string literal",
                    ));
                }
                index += 1;
                tokens.push((Tok::Str(value), start));
            }
            character
                if character.is_ascii_digit()
                    || (character == '-'
                        && chars.get(index + 1).is_some_and(char::is_ascii_digit)) =>
            {
                let mut end = index;
                if chars[end] == '-' {
                    end += 1;
                }
                while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.') {
                    end += 1;
                }
                let text: String = chars[index..end].iter().collect();
                let value = text.parse().map_err(|_| {
                    format_el_parse_error(source, index, format!("invalid number: {text}"))
                })?;
                tokens.push((Tok::Num(value), index));
                index = end;
            }
            character if character.is_alphabetic() || character == '_' || character == '$' => {
                let mut end = index;
                while end < chars.len()
                    && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '$')
                {
                    end += 1;
                }
                let word: String = chars[index..end].iter().collect();
                match word.as_str() {
                    "true" => tokens.push((Tok::Bool(true), index)),
                    "false" => tokens.push((Tok::Bool(false), index)),
                    _ => tokens.push((Tok::Ident(word), index)),
                }
                index = end;
            }
            ':' => {
                // `SWITCH(x).TO(a:tag1)` 的冒号由 parser 的目标处理吸收。
                index += 1;
            }
            other => {
                return Err(format_el_parse_error(
                    source,
                    index,
                    format!("unexpected character: {other}"),
                ));
            }
        }
    }
    Ok(tokens)
}
