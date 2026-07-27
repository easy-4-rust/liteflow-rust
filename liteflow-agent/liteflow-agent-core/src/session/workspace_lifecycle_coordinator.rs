//! 跨 Agent 会话管理器的工作区生命周期协调器。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

/// 记录进程内所有 Agent 会话对共享 conversation 工作区的引用。
///
/// Java 的多个 `AgentSessionManager` 会按 `conversationId` 共享工作区。本对象把同一
/// 目录在不同 `agentKey` 管理器中的生命周期合并，防止某个 Agent 过期或关闭时删除
/// 仍被其他 Agent 使用的目录。对应 Java:
/// `com.yomahub.liteflow.agent.session.AgentSessionManager` 的共享工作区清理语义。
pub(crate) struct WorkspaceLifecycleCoordinator {
    leases: Mutex<HashMap<PathBuf, usize>>,
}

impl WorkspaceLifecycleCoordinator {
    /// 登记一个会话对工作区的占用。
    ///
    /// `workspace_dir` 为已经规范化到 conversation 维度的目录。
    pub(crate) fn acquire(&self, workspace_dir: &Path) {
        let mut leases = self
            .leases
            .lock()
            .expect("agent workspace lifecycle lock poisoned");
        *leases.entry(workspace_dir.to_path_buf()).or_default() += 1;
    }

    /// 释放一个会话对工作区的占用，并在最后一个占用者要求清理时删除目录。
    ///
    /// 删除动作在同一把锁内完成，避免“最后一个旧会话释放”与“新会话登记”交错时
    /// 误删新会话刚开始使用的目录。
    pub(crate) fn release(&self, workspace_dir: &Path, clean_workspace: bool) {
        let mut leases = self
            .leases
            .lock()
            .expect("agent workspace lifecycle lock poisoned");
        let Some(lease_count) = leases.get_mut(workspace_dir) else {
            return;
        };
        if *lease_count > 1 {
            *lease_count -= 1;
            return;
        }
        leases.remove(workspace_dir);
        if clean_workspace {
            delete_recursively(workspace_dir);
        }
    }
}

/// 返回进程级共享工作区生命周期协调器。
pub(crate) fn workspace_lifecycle_coordinator() -> &'static WorkspaceLifecycleCoordinator {
    static COORDINATOR: LazyLock<WorkspaceLifecycleCoordinator> =
        LazyLock::new(|| WorkspaceLifecycleCoordinator {
            leases: Mutex::new(HashMap::new()),
        });
    &COORDINATOR
}

fn delete_recursively(path: &Path) {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            tracing::warn!(
                workspace = %path.display(),
                error = %error,
                "failed to delete expired agent workspace"
            );
        }
        _ => {}
    }
}
