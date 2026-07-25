//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalPathContentParser
//!
//! 非 Spring 环境路径内容解析实现。支持：
//! - 绝对路径的本地文件（自动补 `file:` 前缀语义）
//! - `file:` / `file://` 前缀路径
//!
//! `classpath:` 前缀：Java 侧由 ClassLoader 运行期加载；Rust 无运行期
//! classpath，对应语义为 include_str!/构建期嵌入，运行期解析 classpath
//! 路径会返回 ConfigErrorException。

use std::path::Path;

use crate::exception::config_error_exception::ConfigErrorException;
use crate::exception::LFResult;
use crate::spi::path_content_parser::PathContentParser;
use crate::spi::spi_priority::SpiPriority;

/// 对应 FILE_URL_PREFIX = "file:"
const FILE_URL_PREFIX: &str = "file:";

/// 对应 CLASSPATH_URL_PREFIX = "classpath:"
const CLASSPATH_URL_PREFIX: &str = "classpath:";

/// 对应 LocalPathContentParser
#[derive(Default)]
pub struct LocalPathContentParser;

impl LocalPathContentParser {
    pub fn new() -> Self {
        Self
    }

    /// 剥离 file:/file:// 前缀并规范化为本地路径。
    /// classpath: 路径返回 None（调用方按不支持处理）。
    fn resolve_local_path(path: &str) -> Option<String> {
        if path.starts_with(CLASSPATH_URL_PREFIX) {
            return None;
        }
        if let Some(rest) = path.strip_prefix(FILE_URL_PREFIX) {
            // 兼容 file:// 形式
            let rest = rest.strip_prefix("//").unwrap_or(rest);
            Some(rest.to_string())
        } else {
            Some(path.to_string())
        }
    }
}

impl PathContentParser for LocalPathContentParser {
    /// 对应 parseContent(List<String> pathList)
    fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        if path_list.is_empty() {
            return Err(ConfigErrorException::new("rule source must not be null").into());
        }

        let mut content_list = Vec::new();
        for path in path_list {
            let local = Self::resolve_local_path(path).ok_or_else(|| {
                ConfigErrorException::new(format!(
                    "classpath: 路径运行期解析不受支持（Rust 侧对应 include_str!/构建期嵌入）: {path}"
                ))
            })?;
            let p = Path::new(&local);
            if !p.is_file() {
                return Err(ConfigErrorException::new(format!(
                    "rule source file not found: {path}"
                ))
                .into());
            }
            let content = std::fs::read_to_string(p).map_err(|e| {
                ConfigErrorException::new(format!("read rule source failed: {path}: {e}"))
            })?;
            // 对应 StrUtil.isNotBlank：空白内容不加入
            if !content.trim().is_empty() {
                content_list.push(content);
            }
        }
        Ok(content_list)
    }

    /// 对应 getFileAbsolutePath(List<String> pathList)
    fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        if path_list.is_empty() {
            return Err(ConfigErrorException::new("rule source must not be null").into());
        }

        let mut result = Vec::new();
        for path in path_list {
            // Java 侧 classpath 资源仅当 ClassLoaderUtil.isPresent 时才加入结果，
            // 否则静默跳过；Rust 侧 classpath 无运行期语义，同样跳过。
            let Some(local) = Self::resolve_local_path(path) else {
                continue;
            };
            let p = Path::new(&local);
            if p.is_file() {
                let abs = p.canonicalize().map_err(|e| {
                    ConfigErrorException::new(format!("resolve absolute path failed: {path}: {e}"))
                })?;
                result.push(abs.to_string_lossy().into_owned());
            }
        }
        Ok(result)
    }
}

impl SpiPriority for LocalPathContentParser {
    /// 对应 priority()
    fn priority(&self) -> i32 {
        2
    }
}
