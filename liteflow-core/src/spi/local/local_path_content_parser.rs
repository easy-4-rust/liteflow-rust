//! 对应 Java 类：com.yomahub.liteflow.spi.local.LocalPathContentParser
//!
//! 非 Spring 环境路径内容解析实现。支持：
//! - 绝对路径的本地文件（自动补 `file:` 前缀语义）
//! - `file:` / `file://` 前缀路径
//! - `classpath:` 与裸相对资源路径
//!
//! Java 侧由 ClassLoader 搜索运行时 classpath；Rust 侧依次搜索显式
//! `CLASSPATH`、应用工作目录、Cargo 约定资源目录、当前 crate 资源目录和
//! 可执行文件相邻资源目录。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::exception::LFResult;
use crate::exception::config_error_exception::ConfigErrorException;
use crate::spi::path_content_parser::PathContentParser;
use crate::spi::spi_priority::SpiPriority;
use crate::util::PathMatchUtil;

/// 对应 FILE_URL_PREFIX = "file:"
const FILE_URL_PREFIX: &str = "file:";

/// 对应 CLASSPATH_URL_PREFIX = "classpath:"
const CLASSPATH_URL_PREFIX: &str = "classpath:";

/// 非容器环境的规则资源路径解析器。
///
/// 负责把绝对文件、裸 classpath 名称和显式 `classpath:` 名称解析为真实 UTF-8
/// 内容或绝对路径。对应 Java:
/// `com.yomahub.liteflow.spi.local.LocalPathContentParser`。
#[derive(Default)]
pub struct LocalPathContentParser;

impl LocalPathContentParser {
    /// 创建本地路径内容解析器。
    ///
    /// 对应 Java: `LocalPathContentParser#LocalPathContentParser`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 剥离 `file:` / `file://` 前缀并规范化为本地路径。
    fn resolve_file_path(path: &str) -> String {
        if let Some(rest) = path.strip_prefix(FILE_URL_PREFIX) {
            // 兼容 file:// 形式
            let rest = rest.strip_prefix("//").unwrap_or(rest);
            rest.to_string()
        } else {
            path.to_string()
        }
    }

    /// 构造 Rust 运行时 classpath 根目录。
    ///
    /// 显式 `CLASSPATH` 根优先；随后加入应用工作目录、Cargo 常用资源目录和
    /// 可执行文件附近目录。返回顺序稳定，并按路径去重。
    fn classpath_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(classpath) = std::env::var_os("CLASSPATH") {
            roots.extend(std::env::split_paths(&classpath));
        }

        if let Ok(current_dir) = std::env::current_dir() {
            Self::append_conventional_roots(&mut roots, &current_dir);
        }
        Self::append_conventional_roots(&mut roots, Path::new(env!("CARGO_MANIFEST_DIR")));
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(executable_dir) = current_exe.parent()
        {
            Self::append_conventional_roots(&mut roots, executable_dir);
            if let Some(parent) = executable_dir.parent() {
                roots.push(parent.join("Resources"));
            }
        }

        let mut seen = HashSet::new();
        roots
            .into_iter()
            .filter(|root| seen.insert(root.clone()))
            .collect()
    }

    /// 添加一个应用根目录下的常见 Rust/Java 资源目录。
    fn append_conventional_roots(roots: &mut Vec<PathBuf>, base: &Path) {
        roots.push(base.to_path_buf());
        roots.push(base.join("resources"));
        roots.push(base.join("tests/resources"));
        roots.push(base.join("src/main/resources"));
        roots.push(base.join("src/test/resources"));
    }

    /// 在运行时 classpath 中查找单个资源。
    fn resolve_classpath_resource(path: &str) -> Option<PathBuf> {
        let resource_name = path
            .strip_prefix(CLASSPATH_URL_PREFIX)
            .unwrap_or(path)
            .trim_start_matches(['/', '\\']);
        if resource_name.is_empty() {
            return None;
        }

        Self::classpath_roots()
            .into_iter()
            .map(|root| root.join(resource_name))
            .find(|candidate| candidate.is_file())
            .and_then(|candidate| candidate.canonicalize().ok())
    }

    /// 按 Java LocalPathContentParser 的规则解析内容路径。
    fn resolve_content_path(path: &str) -> LFResult<PathBuf> {
        if path.starts_with(CLASSPATH_URL_PREFIX) {
            return Self::resolve_classpath_resource(path).ok_or_else(|| {
                ConfigErrorException::new(format!("classpath resource not found: {path}")).into()
            });
        }

        let file_path = Self::resolve_file_path(path);
        let local = PathBuf::from(&file_path);
        if local.is_absolute() {
            if local.is_file() {
                return Ok(local);
            }
            return Err(ConfigErrorException::new(format!(
                "rule source file not found: {file_path}"
            ))
            .into());
        }

        // Java 对裸相对路径自动补 classpath: 前缀。
        Self::resolve_classpath_resource(path).ok_or_else(|| {
            ConfigErrorException::new(format!("classpath resource not found: {path}")).into()
        })
    }

    /// 读取路径列表中的全部非空规则内容。
    ///
    /// 参数 `path_list` 支持 classpath/裸相对资源、`file:`、绝对文件和绝对路径
    /// 通配符；返回值按展开顺序保存非空 UTF-8 内容。空路径或缺失资源返回配置
    /// 错误。对应 Java: `LocalPathContentParser#parseContent`。
    pub fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        if path_list.is_empty() {
            return Err(ConfigErrorException::new("rule source must not be null").into());
        }

        let mut content_list = Vec::new();
        for path in PathMatchUtil::search_absolute_path(path_list) {
            let resolved_path = Self::resolve_content_path(&path)?;
            let content = std::fs::read_to_string(&resolved_path).map_err(|error| {
                ConfigErrorException::new(format!(
                    "read rule source failed: {}: {error}",
                    resolved_path.display()
                ))
            })?;
            // 对应 StrUtil.isNotBlank：空白内容不加入
            if !content.trim().is_empty() {
                content_list.push(content);
            }
        }
        Ok(content_list)
    }

    /// 展开并返回路径列表中现存文件的绝对路径。
    ///
    /// 参数 `path_list` 支持裸 classpath 资源、`file:`、绝对文件和绝对路径
    /// 通配符；不存在的路径被忽略。Java 本地实现对已经带 `classpath:` 前缀的
    /// 项不加入绝对路径列表，Rust 保留该可观察行为。对应 Java:
    /// `LocalPathContentParser#getFileAbsolutePath`。
    pub fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        if path_list.is_empty() {
            return Err(ConfigErrorException::new("rule source must not be null").into());
        }

        let mut result = Vec::new();
        for path in PathMatchUtil::search_absolute_path(path_list) {
            // Java 代码仅给“不带 classpath: 前缀”的相对路径补前缀并查询；
            // 已显式携带前缀的分支没有 add，保持这一基线行为。
            if path.starts_with(CLASSPATH_URL_PREFIX) {
                continue;
            }
            let file_path = Self::resolve_file_path(&path);
            let local = PathBuf::from(&file_path);
            let resolved = if local.is_absolute() && local.is_file() {
                local.canonicalize().ok()
            } else if !local.is_absolute() {
                Self::resolve_classpath_resource(&path)
            } else {
                None
            };
            if let Some(resolved) = resolved {
                result.push(resolved.to_string_lossy().into_owned());
            }
        }
        Ok(result)
    }

    /// 返回本地路径解析器的 SPI 优先级。
    ///
    /// 返回值为 `2`，用于在多实现环境中保持 Java 本地解析器的选择顺序。
    /// 对应 Java: `LocalPathContentParser#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        2
    }
}

impl PathContentParser for LocalPathContentParser {
    fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        LocalPathContentParser::parse_content(self, path_list)
    }

    fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        LocalPathContentParser::get_file_absolute_path(self, path_list)
    }
}

impl SpiPriority for LocalPathContentParser {
    fn priority(&self) -> i32 {
        LocalPathContentParser::priority(self)
    }
}
