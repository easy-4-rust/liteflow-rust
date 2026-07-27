// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/mod.rs 中的错误脱敏工具。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! Provider 错误脱敏工具：从 API 错误响应中抹除密钥/令牌等敏感信息。

const MAX_API_ERROR_CHARS: usize = 200;

/// 抹除输入中的密钥/令牌模式，并截断到 `MAX_API_ERROR_CHARS` 字符。
pub fn sanitize_api_error(input: &str) -> String {
    let scrubbed = scrub_secret_patterns(input);

    if scrubbed.chars().count() <= MAX_API_ERROR_CHARS {
        return scrubbed;
    }

    let mut end = MAX_API_ERROR_CHARS;
    while end > 0 && !scrubbed.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...", &scrubbed[..end])
}

/// 抹除常见的密钥/令牌前缀模式（sk-/xoxb-/ghp_/Bearer 等）。
pub fn scrub_secret_patterns(input: &str) -> String {
    const PREFIXES: [(&str, usize); 26] = [
        ("sk-", 1),
        ("xoxb-", 1),
        ("xoxp-", 1),
        ("ghp_", 1),
        ("gho_", 1),
        ("ghu_", 1),
        ("github_pat_", 1),
        ("AIza", 1),
        ("AKIA", 1),
        ("\"access_token\":\"", 8),
        ("\"refresh_token\":\"", 8),
        ("\"id_token\":\"", 8),
        ("\"token\":\"", 8),
        ("\"api_key\":\"", 8),
        ("\"client_secret\":\"", 8),
        ("\"app_secret\":\"", 8),
        ("\"verify_token\":\"", 8),
        ("access_token=", 8),
        ("refresh_token=", 8),
        ("id_token=", 8),
        ("token=", 8),
        ("api_key=", 8),
        ("client_secret=", 8),
        ("app_secret=", 8),
        ("Bearer ", 16),
        ("bearer ", 16),
    ];

    let mut scrubbed = input.to_string();

    for (prefix, min_len) in PREFIXES {
        let mut search_from = 0;
        loop {
            let Some(rel) = scrubbed[search_from..].find(prefix) else {
                break;
            };

            let start = search_from + rel;
            let content_start = start + prefix.len();
            let end = token_end(&scrubbed, content_start);
            let token_len = end.saturating_sub(content_start);

            // Bare prefixes like "sk-" should not stop future scans.
            if token_len < min_len {
                search_from = content_start;
                continue;
            }

            scrubbed.replace_range(start..end, "[REDACTED]");
            search_from = start + "[REDACTED]".len();
        }
    }

    scrubbed
}

fn token_end(input: &str, from: usize) -> usize {
    let mut end = from;
    for (i, c) in input[from..].char_indices() {
        if is_secret_char(c) {
            end = from + i + c.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn is_secret_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')
}

/// 构造带状态码与脱敏正文的 provider API 错误。
pub async fn api_error(provider: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read provider error body>".to_string());
    let sanitized = sanitize_api_error(&body);
    anyhow::anyhow!("{provider} API error ({status}): {sanitized}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_openai_key() {
        let input = "error: invalid key sk-proj-abcdef123456";
        let scrubbed = scrub_secret_patterns(input);
        assert!(scrubbed.contains("[REDACTED]"));
        assert!(!scrubbed.contains("sk-proj-abcdef123456"));
    }

    #[test]
    fn scrubs_bearer_token() {
        let input = "Authorization: Bearer ya29.example-token-here";
        let scrubbed = scrub_secret_patterns(input);
        assert!(scrubbed.contains("[REDACTED]"));
    }

    #[test]
    fn truncates_long_errors() {
        let long = "x".repeat(500);
        let sanitized = sanitize_api_error(&long);
        assert!(sanitized.ends_with("..."));
        assert!(sanitized.chars().count() <= MAX_API_ERROR_CHARS + 3);
    }

    #[test]
    fn preserves_short_errors() {
        let short = "bad request";
        assert_eq!(sanitize_api_error(short), short);
    }
}
