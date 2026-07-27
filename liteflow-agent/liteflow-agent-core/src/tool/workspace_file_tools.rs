use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentscope_core::tool::AgentTool;

use super::{DeleteFileTool, ListFilesTool, ReadFileTool, WriteFileTool};
use crate::{AgentError, WorkspaceConfig};

/// 在单个会话工作区内提供受限的文件读写能力。
///
/// 路径必须相对当前会话目录，规范化后的路径以及已有符号链接的真实目标都不得
/// 越过工作区根目录。读取内容和目录列表分别受配置的字节数与条目数限制。
///
/// 对应 Java: `com.yomahub.liteflow.agent.tool.WorkspaceFileTools`。
pub struct WorkspaceFileTools {
    workspace: PathBuf,
    max_bytes: u64,
    max_list: usize,
}

impl WorkspaceFileTools {
    /// 校验或创建总工作区根目录，并返回规范化后的真实路径。
    ///
    /// # 参数
    /// - `config`: `liteflow.agent.workspace.*` 配置。
    ///
    /// # 返回
    /// 可安全用于派生会话目录的绝对路径。
    ///
    /// 对应 Java: `AgentSessionManager#AgentSessionManager`。
    pub fn prepare_root(config: &WorkspaceConfig) -> Result<Option<PathBuf>, AgentError> {
        let Some(root) = config.root.as_deref() else {
            return Ok(None);
        };
        let root = PathBuf::from(root);
        if root.exists() {
            if !root.is_dir() {
                return Err(AgentError::WorkspaceIo {
                    operation: "initialize",
                    message: format!("{} is not a directory", root.display()),
                });
            }
        } else if config.auto_create {
            fs::create_dir_all(&root).map_err(|error| AgentError::WorkspaceIo {
                operation: "initialize",
                message: error.to_string(),
            })?;
        } else {
            return Err(AgentError::WorkspaceRootDoesNotExist(
                root.display().to_string(),
            ));
        }
        root.canonicalize()
            .map(Some)
            .map_err(|error| AgentError::WorkspaceIo {
                operation: "initialize",
                message: error.to_string(),
            })
    }

    /// 为指定会话创建文件工具集合。
    ///
    /// # 参数
    /// - `root`: 已通过 [`Self::prepare_root`] 校验的总工作区根目录。
    /// - `conversation_id`: 原始会话标识；目录名会按 Java `safeId` 语义转义。
    /// - `config`: 文件大小与列表数量限制。
    ///
    /// # 返回
    /// 绑定到独立会话目录的文件工具对象。
    ///
    /// 对应 Java: `AgentSessionManager#getOrCreate` 与
    /// `ReActAgentComponent#buildAgent`。
    pub fn for_conversation(
        root: &Path,
        conversation_id: &str,
        config: &WorkspaceConfig,
    ) -> Result<Self, AgentError> {
        let workspace = root.join(safe_id(conversation_id));
        fs::create_dir_all(&workspace).map_err(|error| AgentError::WorkspaceIo {
            operation: "initialize",
            message: error.to_string(),
        })?;
        let workspace = workspace
            .canonicalize()
            .map_err(|error| AgentError::WorkspaceIo {
                operation: "initialize",
                message: error.to_string(),
            })?;
        if !workspace.starts_with(root) {
            return Err(AgentError::WorkspacePathDenied(
                workspace.display().to_string(),
            ));
        }
        Ok(Self {
            workspace,
            max_bytes: config.max_file_bytes,
            max_list: config.max_list_size,
        })
    }

    /// 返回当前会话工作区的绝对路径。
    ///
    /// # 返回
    /// 规范化且已解析符号链接的会话目录。
    #[must_use]
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// 创建四个可注册到 AgentScope Toolkit 的独立工具对象。
    ///
    /// # 返回
    /// `read_file`、`write_file`、`list_files`、`delete_file` 工具。
    ///
    /// 对应 Java: `ReActAgentComponent#buildAgent`。
    #[must_use]
    pub fn tools(self) -> Vec<Arc<dyn AgentTool>> {
        let workspace = Arc::new(self);
        vec![
            Arc::new(ReadFileTool::new(Arc::clone(&workspace))),
            Arc::new(WriteFileTool::new(Arc::clone(&workspace))),
            Arc::new(ListFilesTool::new(Arc::clone(&workspace))),
            Arc::new(DeleteFileTool::new(workspace)),
        ]
    }

    /// 读取相对路径文本；超过上限时仅返回前 `max_file_bytes` 个字节。
    ///
    /// # 参数
    /// - `path`: 相对当前会话工作区的文件路径。
    ///
    /// # 返回
    /// UTF-8 有损解码后的文本内容。
    ///
    /// 对应 Java: `WorkspaceFileTools#readFile`。
    pub fn read_file(&self, path: &str) -> Result<String, AgentError> {
        let path = self.resolve_safe(path)?;
        let file = File::open(path).map_err(|error| AgentError::WorkspaceIo {
            operation: "read_file",
            message: error.to_string(),
        })?;
        let mut bytes = Vec::new();
        file.take(self.max_bytes)
            .read_to_end(&mut bytes)
            .map_err(|error| AgentError::WorkspaceIo {
                operation: "read_file",
                message: error.to_string(),
            })?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// 覆盖写入相对路径文本，并按需创建父目录。
    ///
    /// # 参数
    /// - `path`: 相对当前会话工作区的文件路径。
    /// - `content`: 要以 UTF-8 写入的文本。
    ///
    /// # 返回
    /// 成功时返回 `"ok"`。
    ///
    /// 对应 Java: `WorkspaceFileTools#writeFile`。
    pub fn write_file(&self, path: &str, content: &str) -> Result<String, AgentError> {
        let path = self.resolve_safe(path)?;
        let parent = path
            .parent()
            .ok_or_else(|| AgentError::WorkspacePathDenied(path.display().to_string()))?;
        fs::create_dir_all(parent).map_err(|error| AgentError::WorkspaceIo {
            operation: "write_file",
            message: error.to_string(),
        })?;
        // 创建父目录后重新验证真实父路径，阻止并发替换或符号链接逃逸。
        self.ensure_existing_path_safe(parent)?;
        fs::write(path, content).map_err(|error| AgentError::WorkspaceIo {
            operation: "write_file",
            message: error.to_string(),
        })?;
        Ok("ok".to_string())
    }

    /// 列出一个工作区目录的直接子项。
    ///
    /// # 参数
    /// - `path`: 可选相对目录；空值与空字符串均表示当前目录。
    ///
    /// # 返回
    /// 相对会话工作区的路径，最多 `max_list_size` 项。
    ///
    /// 对应 Java: `WorkspaceFileTools#listFiles`。
    pub fn list_files(&self, path: Option<&str>) -> Result<Vec<String>, AgentError> {
        let relative = path.filter(|value| !value.is_empty()).unwrap_or(".");
        let directory = self.resolve_safe(relative)?;
        let entries = fs::read_dir(directory).map_err(|error| AgentError::WorkspaceIo {
            operation: "list_files",
            message: error.to_string(),
        })?;
        let mut output = Vec::new();
        for entry in entries.take(self.max_list) {
            let entry = entry.map_err(|error| AgentError::WorkspaceIo {
                operation: "list_files",
                message: error.to_string(),
            })?;
            let relative = entry
                .path()
                .strip_prefix(&self.workspace)
                .map_err(|_| AgentError::WorkspacePathDenied(entry.path().display().to_string()))?
                .to_string_lossy()
                .into_owned();
            output.push(relative);
        }
        Ok(output)
    }

    /// 删除相对路径文件或空目录；路径不存在时仍返回成功。
    ///
    /// # 参数
    /// - `path`: 相对当前会话工作区的路径。
    ///
    /// # 返回
    /// 成功时返回 `"ok"`。
    ///
    /// 对应 Java: `WorkspaceFileTools#deleteFile`。
    pub fn delete_file(&self, path: &str) -> Result<String, AgentError> {
        let path = self.resolve_safe(path)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_dir() => {
                fs::remove_dir(path).map_err(|error| AgentError::WorkspaceIo {
                    operation: "delete_file",
                    message: error.to_string(),
                })?;
            }
            Ok(_) => {
                fs::remove_file(path).map_err(|error| AgentError::WorkspaceIo {
                    operation: "delete_file",
                    message: error.to_string(),
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AgentError::WorkspaceIo {
                    operation: "delete_file",
                    message: error.to_string(),
                });
            }
        }
        Ok("ok".to_string())
    }

    fn resolve_safe(&self, relative: &str) -> Result<PathBuf, AgentError> {
        if relative.is_empty() {
            return Err(AgentError::WorkspacePathDenied("path is empty".to_string()));
        }
        let relative_path = Path::new(relative);
        if relative_path.is_absolute() {
            return Err(AgentError::WorkspacePathDenied(format!(
                "absolute path denied: {relative}"
            )));
        }
        let candidate = normalize_lexically(&self.workspace.join(relative_path));
        if !candidate.starts_with(&self.workspace) {
            return Err(AgentError::WorkspacePathDenied(format!(
                "path escapes workspace: {relative}"
            )));
        }

        // 已存在路径解析到真实目标；待创建路径则验证最近的已有祖先。
        let mut existing = candidate.as_path();
        while !existing.exists() {
            existing = existing
                .parent()
                .ok_or_else(|| AgentError::WorkspacePathDenied(relative.to_string()))?;
        }
        self.ensure_existing_path_safe(existing)?;
        Ok(candidate)
    }

    fn ensure_existing_path_safe(&self, path: &Path) -> Result<(), AgentError> {
        let canonical = path
            .canonicalize()
            .map_err(|error| AgentError::WorkspaceIo {
                operation: "resolve",
                message: error.to_string(),
            })?;
        if canonical.starts_with(&self.workspace) {
            Ok(())
        } else {
            Err(AgentError::WorkspacePathDenied(path.display().to_string()))
        }
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn safe_id(raw: &str) -> String {
    if raw.is_empty() {
        return "_".to_string();
    }
    if raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return raw.to_string();
    }
    let mut encoded = String::new();
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'*') {
            encoded.push(char::from(byte));
        } else if byte == b' ' {
            encoded.push('+');
        } else {
            encoded.push('_');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}
