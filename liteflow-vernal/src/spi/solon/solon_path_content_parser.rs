use std::collections::HashSet;
use std::path::{Path, PathBuf};

use liteflow_core::LFResult;
use liteflow_core::exception::config_error_exception::ConfigErrorException;
use liteflow_core::spi::{PathContentParser, SpiPriority};
use liteflow_core::util::PathMatchUtil;

use super::ResourceUtils;

/// Solon 环境的规则路径内容解析器。
///
/// Java 使用 `ResourceUtil.getResource` 读取 classpath 或裸相对资源，绝对文件
/// 则直接转换为 URL；Rust 从运行目录、Cargo 资源目录、可执行文件资源目录和
/// `CLASSPATH` 构造等价资源根，并保留空路径校验、UTF-8 非空过滤及扩展名一致性
/// 约束。对应 Java:
/// `com.yomahub.liteflow.spi.solon.SolonPathContentParser`。
#[derive(Debug, Default)]
pub struct SolonPathContentParser;

impl SolonPathContentParser {
    /// 创建无状态的 Solon 路径解析器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 读取全部非空规则内容。
    ///
    /// # 参数
    /// - `path_list`：绝对文件、`file:`、`classpath:`、`classpath*:` 或裸相对
    ///   资源路径。
    ///
    /// # 返回
    /// UTF-8 非空内容列表；资源扩展名不一致时返回配置错误。对应 Java:
    /// `SolonPathContentParser#parseContent`。
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
            // Java StrUtil.isNotBlank：空白资源不交给后续规则解析器。
            if !content.trim().is_empty() {
                content_list.push(content);
            }
        }
        Ok(content_list)
    }

    /// 返回资源对应的绝对文件路径。
    ///
    /// # 参数
    /// - `path_list`：与 `parse_content` 相同的资源列表。
    ///
    /// # 返回
    /// 规范化、去重后的真实文件路径。对应 Java:
    /// `SolonPathContentParser#getFileAbsolutePath`。
    pub fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        Self::get_resources(path_list).map(|resources| {
            resources
                .into_iter()
                .map(|resource| resource.to_string_lossy().into_owned())
                .collect()
        })
    }

    /// 返回 Solon SPI 优先级。
    #[must_use]
    pub fn priority(&self) -> i32 {
        1
    }

    /// 把配置路径解析成真实文件资源。
    fn get_resources(path_list: &[String]) -> LFResult<Vec<PathBuf>> {
        if path_list.is_empty() {
            return Err(ConfigErrorException::new("rule source must not be null").into());
        }

        let mut resources = Vec::new();
        for path in PathMatchUtil::search_absolute_path(path_list) {
            if let Some(pattern) = path.strip_prefix(ResourceUtils::CLASSPATH_ALL_URL_PREFIX) {
                resources.extend(Self::resolve_all_classpath_resources(pattern));
                continue;
            }
            if let Some(pattern) = path.strip_prefix(ResourceUtils::CLASSPATH_URL_PREFIX) {
                resources.extend(Self::resolve_first_classpath_resources(pattern));
                continue;
            }

            let local_path = Self::strip_file_prefix(&path);
            let local_path = PathBuf::from(&local_path);
            if local_path.is_absolute() && local_path.is_file() {
                if let Ok(canonical) = local_path.canonicalize() {
                    resources.push(canonical);
                }
            } else if local_path.is_absolute() && Self::contains_wildcard(&path) {
                resources.extend(Self::expand_absolute_pattern(&path));
            } else {
                // Java ResourceUtil.getResource 对裸相对路径执行 classpath 查询。
                resources.extend(Self::resolve_first_classpath_resources(&path));
            }
        }

        let mut seen = HashSet::new();
        Ok(resources
            .into_iter()
            .filter(|resource| seen.insert(resource.clone()))
            .collect())
    }

    /// 返回当前 Solon/Rust 应用可见的资源根。
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

    /// 添加工程与发行包的惯用资源目录。
    fn append_conventional_roots(roots: &mut Vec<PathBuf>, base: &Path) {
        roots.push(base.to_path_buf());
        roots.push(base.join("resources"));
        roots.push(base.join("tests/resources"));
        roots.push(base.join("src/main/resources"));
        roots.push(base.join("src/test/resources"));
    }

    /// 从首个存在匹配资源的根返回结果。
    fn resolve_first_classpath_resources(pattern: &str) -> Vec<PathBuf> {
        for root in Self::classpath_roots() {
            let resources = Self::resolve_pattern_under_root(&root, pattern);
            if !resources.is_empty() {
                return resources;
            }
        }
        Vec::new()
    }

    /// 跨全部资源根解析 classpath* 模式。
    fn resolve_all_classpath_resources(pattern: &str) -> Vec<PathBuf> {
        Self::classpath_roots()
            .into_iter()
            .flat_map(|root| Self::resolve_pattern_under_root(&root, pattern))
            .collect()
    }

    /// 在单个资源根下解析路径或 Ant 模式。
    fn resolve_pattern_under_root(root: &Path, pattern: &str) -> Vec<PathBuf> {
        let pattern = pattern.trim_start_matches(['/', '\\']);
        if pattern.is_empty() {
            return Vec::new();
        }
        let candidate = root.join(pattern);
        if Self::contains_wildcard(pattern) {
            return Self::expand_absolute_pattern(&candidate.to_string_lossy());
        }
        candidate
            .is_file()
            .then(|| candidate.canonicalize().ok())
            .flatten()
            .into_iter()
            .collect()
    }

    /// 展开绝对 Ant 风格模式。
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

    /// 去除 file URL 前缀。
    fn strip_file_prefix(path: &str) -> String {
        path.strip_prefix(ResourceUtils::FILE_URL_PREFIX)
            .map(|path| path.strip_prefix("//").unwrap_or(path))
            .unwrap_or(path)
            .to_string()
    }

    /// 判断是否含路径通配符。
    fn contains_wildcard(path: &str) -> bool {
        path.contains('*') || path.contains('?')
    }
}

impl PathContentParser for SolonPathContentParser {
    fn parse_content(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        SolonPathContentParser::parse_content(self, path_list)
    }

    fn get_file_absolute_path(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        SolonPathContentParser::get_file_absolute_path(self, path_list)
    }
}

impl SpiPriority for SolonPathContentParser {
    fn priority(&self) -> i32 {
        SolonPathContentParser::priority(self)
    }
}
