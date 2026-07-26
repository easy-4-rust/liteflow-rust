//! LiteFlow EL 词法分析。

use crate::exception::{LFResult, LiteflowError};

use super::Tok;

/// 把 EL 文本转换为递归下降解析器 token。
pub(crate) fn lex(source: &str) -> LFResult<Vec<Tok>> {
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < chars.len() {
        let character = chars[index];
        match character {
            character if character.is_whitespace() => index += 1,
            '(' => {
                tokens.push(Tok::LP);
                index += 1;
            }
            ')' => {
                tokens.push(Tok::RP);
                index += 1;
            }
            ',' => {
                tokens.push(Tok::Comma);
                index += 1;
            }
            '.' => {
                tokens.push(Tok::Dot);
                index += 1;
            }
            '"' | '\'' => {
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
                    return Err(LiteflowError::Parse("unclosed string literal".into()));
                }
                index += 1;
                tokens.push(Tok::Str(value));
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
                let value = text
                    .parse()
                    .map_err(|_| LiteflowError::Parse(format!("invalid number: {text}")))?;
                tokens.push(Tok::Num(value));
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
                    "true" => tokens.push(Tok::Bool(true)),
                    "false" => tokens.push(Tok::Bool(false)),
                    _ => tokens.push(Tok::Ident(word)),
                }
                index = end;
            }
            ':' => {
                // `SWITCH(x).TO(a:tag1)` 的冒号由 parser 的目标处理吸收。
                index += 1;
            }
            other => {
                return Err(LiteflowError::Parse(format!(
                    "unexpected character: {other}"
                )));
            }
        }
    }
    Ok(tokens)
}
