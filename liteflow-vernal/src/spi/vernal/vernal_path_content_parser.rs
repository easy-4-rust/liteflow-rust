//! 对应 Java 类：com.yomahub.liteflow.spi.spring.SpringPathContentParser

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use liteflow_core::LFResult;
use liteflow_core::exception::config_error_exception::ConfigErrorException;
use liteflow_core::spi::{PathContentParser, SpiPriority};
use liteflow_core::util::PathMatchUtil;

const FILE_URL_PREFIX: &str = "file:";
const CLASSPATH_URL_PREFIX: &str = "classpath:";
const CLASSPATH_ALL_URL_PREFIX: &str = "classpath*:";

/// Vernal 容器环境的规则资源路径解析器。
///
/// 使用 Rust 应用资源根映射 Spring `PathMatchingResourcePatternResolver`：
/// `classpath:` 与裸相对路径从首个匹配资源根读取，`classpath*:` 跨全部资源根
/// 展开 Ant 风格通配符，绝对路径仍作为本地文件处理。对应 Java:
/// `com.yomahub.liteflow.spi.spring.SpringPathContentParser`。
#[derive(Default)]
pub struct VernalPathContentParser;

impl VernalPathContentParser {
    /// 创建 Vernal 路径内容解析器。
    ///
    /// # 返回
    /// 无状态的路径解析器。对应 Java:
    /// `SpringPathContentParser#SpringPathContentParser`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 解析路径下的全部非空规则内容。
    ///
    /// # 参数
    /// - `path_list`：绝对文件、裸 classpath 路径、`classpath:` 路径或
    ///   `classpath*:` 多资源模式。
    ///
    /// # 返回
    /// 按资源根及文件名稳定顺序排列的 UTF-8 非空内容；不同扩展名混用时返回
    /// 配置错误。对应 Java: `SpringPathContentParser#parseContent`。
    pub fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        let resources = Self::get_resources(path_list)?;
        Self::verify_file_ext_name(&resources)?;

        let mut content_list = Vec::new();
        for resource in resources {
            let content = std::fs::read_to_string(&resource).map_err(|error| {
                ConfigErrorException::new(format!(
                    "read rule source failed: {}: {error}",
                    resource.display()
                ))
            })?;
            // 对应 Java StrUtil.isNotBlank：只向解析器提交实际含有规则的资源。
            if !content.trim().is_empty() {
                content_list.push(content);
            }
        }
        Ok(content_list)
    }

    /// 返回全部可解析文件资源的绝对路径。
    ///
    /// # 参数
    /// - `path_list`：与 `parse_content` 相同的资源路径或模式。
    ///
    /// # 返回
    /// 去重后的规范化绝对文件路径。Rust 当前仅暴露真实文件资源，因此等价于
    /// Java 对 `Resource::isFile` 的过滤结果。对应 Java:
    /// `SpringPathContentParser#getFileAbsolutePath`。
    pub fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        Self::get_resources(path_list).map(|resources| {
            resources
                .into_iter()
                .map(|resource| resource.to_string_lossy().into_owned())
                .collect()
        })
    }

    /// 返回 Vernal 路径解析器 SPI 优先级。
    ///
    /// # 返回
    /// 固定返回 `1`，高于非容器本地实现的 `2`。对应 Java:
    /// `SpringPathContentParser#priority`。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }

    /// 将路径模式解析为真实文件资源。
    fn get_resources(path_list: &[String]) -> LFResult<Vec<PathBuf>> {
        if path_list.is_empty() {
            return Err(ConfigErrorException::new("rule source must not be null").into());
        }

        let mut resources = Vec::new();
        for path in PathMatchUtil::search_absolute_path(path_list) {
            if path.starts_with(CLASSPATH_ALL_URL_PREFIX) {
                let pattern = path.strip_prefix(CLASSPATH_ALL_URL_PREFIX).unwrap_or(&path);
                resources.extend(Self::resolve_all_classpath_resources(pattern));
                continue;
            }

            if path.starts_with(CLASSPATH_URL_PREFIX) {
                let pattern = path.strip_prefix(CLASSPATH_URL_PREFIX).unwrap_or(&path);
                resources.extend(Self::resolve_first_classpath_resources(pattern));
                continue;
            }

            let file_path = Self::strip_file_prefix(&path);
            let local_path = PathBuf::from(&file_path);
            if local_path.is_absolute() && local_path.is_file() {
                if let Ok(canonical) = local_path.canonicalize() {
                    resources.push(canonical);
                }
            } else if local_path.is_absolute() && Self::contains_wildcard(&file_path) {
                resources.extend(Self::expand_absolute_pattern(&file_path));
            } else {
                // Java 对未带协议的相对路径自动补 classpath: 前缀。
                resources.extend(Self::resolve_first_classpath_resources(&path));
            }
        }

        let mut seen = HashSet::new();
        Ok(resources
            .into_iter()
            .filter(|resource| seen.insert(resource.clone()))
            .collect())
    }

    /// 构造容器运行时的 classpath 资源根。
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

    /// 添加 Rust 与 Java 工程常用资源目录。
    fn append_conventional_roots(roots: &mut Vec<PathBuf>, base: &Path) {
        roots.push(base.to_path_buf());
        roots.push(base.join("resources"));
        roots.push(base.join("tests/resources"));
        roots.push(base.join("src/main/resources"));
        roots.push(base.join("src/test/resources"));
    }

    /// 在第一个含匹配项的 classpath 根中解析资源。
    fn resolve_first_classpath_resources(pattern: &str) -> Vec<PathBuf> {
        for root in Self::classpath_roots() {
            let matches = Self::resolve_pattern_under_root(&root, pattern);
            if !matches.is_empty() {
                return matches;
            }
        }
        Vec::new()
    }

    /// 跨全部 classpath 根解析资源。
    fn resolve_all_classpath_resources(pattern: &str) -> Vec<PathBuf> {
        Self::classpath_roots()
            .into_iter()
            .flat_map(|root| Self::resolve_pattern_under_root(&root, pattern))
            .collect()
    }

    /// 在指定资源根下解析单个路径或 Ant 风格模式。
    fn resolve_pattern_under_root(root: &Path, pattern: &str) -> Vec<PathBuf> {
        let resource_pattern = pattern.trim_start_matches(['/', '\\']);
        if resource_pattern.is_empty() {
            return Vec::new();
        }
        let candidate = root.join(resource_pattern);
        if Self::contains_wildcard(resource_pattern) {
            return Self::expand_absolute_pattern(&candidate.to_string_lossy());
        }
        candidate
            .is_file()
            .then(|| candidate.canonicalize().ok())
            .flatten()
            .into_iter()
            .collect()
    }

    /// 使用核心 PathMatchUtil 展开绝对 Ant 风格路径。
    fn expand_absolute_pattern(pattern: &str) -> Vec<PathBuf> {
        PathMatchUtil::search_absolute_path(&[pattern.to_string()])
            .into_iter()
            .filter_map(|path| PathBuf::from(path).canonicalize().ok())
            .collect()
    }

    /// 校验所有资源扩展名一致。
    fn verify_file_ext_name(resources: &[PathBuf]) -> LFResult<()> {
        let file_types = resources
            .iter()
            .map(|resource| {
                resource
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect::<HashSet<_>>();
        if file_types.len() > 1 {
            return Err(ConfigErrorException::new(
                "config error,please use the same type of configuration",
            )
            .into());
        }
        Ok(())
    }

    /// 剥离 `file:` 或 `file://` 前缀。
    fn strip_file_prefix(path: &str) -> String {
        path.strip_prefix(FILE_URL_PREFIX)
            .map(|path| path.strip_prefix("//").unwrap_or(path))
            .unwrap_or(path)
            .to_string()
    }

    /// 判断路径是否包含 Ant 风格通配符。
    fn contains_wildcard(path: &str) -> bool {
        path.contains('*') || path.contains('?')
    }
}

impl PathContentParser for VernalPathContentParser {
    fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        VernalPathContentParser::parse_content(self, path_list)
    }

    fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        VernalPathContentParser::get_file_absolute_path(self, path_list)
    }
}

impl SpiPriority for VernalPathContentParser {
    fn priority(&self) -> i32 {
        VernalPathContentParser::priority(self)
    }
}
