//! 绝对路径 Ant 风格通配符展开工具。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 绝对路径通配符匹配工具。
///
/// 对应 Java: `com.yomahub.liteflow.util.PathMatchUtil`。支持 `*`、`?` 与跨目录
/// `**`；相对路径和不含通配符的绝对路径保持原样，多个输入按首次出现顺序去重。
pub struct PathMatchUtil;

impl PathMatchUtil {
    /// 展开绝对路径中的 Ant 风格通配符。
    ///
    /// `path_list` 中的相对路径不会访问文件系统；绝对通配路径从首个通配符之前
    /// 的最大固定目录开始递归。对应 Java: `PathMatchUtil#searchAbsolutePath`。
    #[must_use]
    pub fn search_absolute_path(path_list: &[String]) -> Vec<String> {
        let mut paths = Vec::new();
        for path in path_list {
            let normalized = normalize(path);
            if !Path::new(&normalized).is_absolute() || !contains_wildcard(&normalized) {
                paths.push(normalized);
                continue;
            }

            let base = fixed_base_dir(&normalized);
            collect_matches(&base, &normalized, &mut paths);
        }

        let mut seen = HashSet::new();
        paths
            .into_iter()
            .filter(|path| seen.insert(path.clone()))
            .collect()
    }
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/")
}

fn contains_wildcard(path: &str) -> bool {
    path.contains('*') || path.contains('?')
}

fn fixed_base_dir(pattern: &str) -> PathBuf {
    let wildcard_index = pattern
        .find(['*', '?'])
        .expect("caller guarantees wildcard");
    let slash_index = pattern[..wildcard_index].rfind('/').unwrap_or(0);
    if slash_index == 0 {
        PathBuf::from("/")
    } else {
        PathBuf::from(&pattern[..slash_index])
    }
}

fn collect_matches(directory: &Path, pattern: &str, matches: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries = entries.flatten().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_matches(&path, pattern, matches);
        } else if file_type.is_file() {
            let absolute = normalize(&path.to_string_lossy());
            if ant_match(pattern, &absolute) {
                matches.push(absolute);
            }
        }
    }
}

fn ant_match(pattern: &str, path: &str) -> bool {
    let pattern_segments = pattern.split('/').collect::<Vec<_>>();
    let path_segments = path.split('/').collect::<Vec<_>>();
    let mut memo = std::collections::HashMap::new();
    match_segments(&pattern_segments, &path_segments, 0, 0, &mut memo)
}

fn match_segments(
    pattern: &[&str],
    path: &[&str],
    pattern_index: usize,
    path_index: usize,
    memo: &mut std::collections::HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(result) = memo.get(&(pattern_index, path_index)) {
        return *result;
    }
    let result = if pattern_index == pattern.len() {
        path_index == path.len()
    } else if pattern[pattern_index] == "**" {
        match_segments(pattern, path, pattern_index + 1, path_index, memo)
            || (path_index < path.len()
                && match_segments(pattern, path, pattern_index, path_index + 1, memo))
    } else {
        path_index < path.len()
            && match_segment(pattern[pattern_index], path[path_index])
            && match_segments(pattern, path, pattern_index + 1, path_index + 1, memo)
    };
    memo.insert((pattern_index, path_index), result);
    result
}

fn match_segment(pattern: &str, value: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let value = value.chars().collect::<Vec<_>>();
    let mut table = vec![vec![false; value.len() + 1]; pattern.len() + 1];
    table[0][0] = true;
    for index in 1..=pattern.len() {
        if pattern[index - 1] == '*' {
            table[index][0] = table[index - 1][0];
        }
    }
    for pattern_index in 1..=pattern.len() {
        for value_index in 1..=value.len() {
            table[pattern_index][value_index] = match pattern[pattern_index - 1] {
                '*' => {
                    table[pattern_index - 1][value_index] || table[pattern_index][value_index - 1]
                }
                '?' => table[pattern_index - 1][value_index - 1],
                character => {
                    character == value[value_index - 1] && table[pattern_index - 1][value_index - 1]
                }
            };
        }
    }
    table[pattern.len()][value.len()]
}
