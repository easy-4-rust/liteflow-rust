use std::collections::HashSet;
use std::path::{Path, PathBuf};

use liteflow_core::util::PathMatchUtil;

/// Solon 规则资源通配路径解析工具。
///
/// Java 为兼容旧版 Solon 自行组合 `ScanUtil` 与 `PathMatchUtil`；Rust 对绝对
/// 路径沿用核心 Ant 风格匹配，对相对路径依次扫描运行目录与常用资源目录。
/// 对应 Java: `com.yomahub.liteflow.solon.config.PathsUtils`。
#[derive(Debug, Clone, Copy, Default)]
pub struct PathsUtils;

impl PathsUtils {
    /// 解析单个 Solon 路径表达式。
    ///
    /// # 参数
    /// - `path_expr`：普通相对资源、绝对路径或包含 `*`/`**` 的表达式。
    ///
    /// # 返回
    /// 去重并按字典序排列的匹配路径；不含通配符的相对资源原样返回。对应 Java:
    /// `PathsUtils#resolvePaths`。
    #[must_use]
    pub fn resolve_paths(path_expr: &str) -> Vec<String> {
        if Path::new(path_expr).is_absolute() {
            return Self::resolve_absolute_paths(path_expr);
        }
        if !path_expr.contains("/*") {
            return vec![path_expr.to_string()];
        }

        let mut paths = Vec::new();
        for root in Self::resource_roots() {
            let pattern = root.join(path_expr);
            paths.extend(
                PathMatchUtil::search_absolute_path(&[pattern.to_string_lossy().into_owned()])
                    .into_iter()
                    .filter_map(|path| PathBuf::from(path).canonicalize().ok())
                    .map(|path| path.to_string_lossy().into_owned()),
            );
        }
        paths.sort();
        paths.dedup();
        paths
    }

    /// 解析一个或多个逗号分隔的绝对路径模式。
    fn resolve_absolute_paths(path_expr: &str) -> Vec<String> {
        let expressions = path_expr
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut paths = PathMatchUtil::search_absolute_path(&expressions);
        paths.sort();
        paths.dedup();
        paths
    }

    /// 返回 Solon/Rust 宿主常用的资源扫描根。
    fn resource_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(current_dir) = std::env::current_dir() {
            Self::append_roots(&mut roots, &current_dir);
        }
        Self::append_roots(&mut roots, Path::new(env!("CARGO_MANIFEST_DIR")));
        let mut seen = HashSet::new();
        roots
            .into_iter()
            .filter(|root| seen.insert(root.clone()))
            .collect()
    }

    /// 添加工程根及惯用资源子目录。
    fn append_roots(roots: &mut Vec<PathBuf>, base: &Path) {
        roots.push(base.to_path_buf());
        roots.push(base.join("resources"));
        roots.push(base.join("tests/resources"));
        roots.push(base.join("src/main/resources"));
        roots.push(base.join("src/test/resources"));
    }
}
