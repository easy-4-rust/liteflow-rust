use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use liteflow_core::spi::{Bean, ContextAware};

const ATTACHMENT_NAME: &str = "liteflow.solon.node-id-holder";

/// Solon 应用上下文中的节点 ID 持有器。
///
/// Java 通过 `AppContext#attachOf` 为每个应用上下文保存独立集合；Rust 通过
/// 当前 `ContextAware` 的命名 Bean 附件实现同样的上下文隔离，避免静态集合在
/// 并行应用间污染。对应 Java:
/// `com.yomahub.liteflow.process.holder.SolonNodeIdHolder`。
#[derive(Default)]
pub struct SolonNodeIdHolder {
    node_id_set: RwLock<HashSet<String>>,
}

impl SolonNodeIdHolder {
    /// 从上下文附件获取或创建节点持有器。
    ///
    /// # 参数
    /// - `context`：当前 Solon/Vernal 环境的 LiteFlow 容器 SPI。
    ///
    /// # 返回
    /// 同一上下文中稳定复用的共享持有器。对应 Java:
    /// `SolonNodeIdHolder#of(AppContext)`。
    #[must_use]
    pub fn of(context: &dyn ContextAware) -> Arc<Self> {
        if let Some(bean) = context.get_bean(ATTACHMENT_NAME)
            && let Ok(holder) = Arc::downcast::<Self>(bean)
        {
            return holder;
        }

        let holder = Arc::new(Self::default());
        let bean: Bean = Arc::clone(&holder) as Bean;
        let registered = context.register_or_get(ATTACHMENT_NAME, &|| Arc::clone(&bean));
        Arc::downcast::<Self>(registered).unwrap_or(holder)
    }

    /// 添加一个待统一注册的节点 ID。
    ///
    /// # 参数
    /// - `node_id`：Solon Bean 名或注解显式节点 ID。空白 ID 不进入集合。
    ///
    /// 对应 Java: `SolonNodeIdHolder#add`。
    pub fn add(&self, node_id: impl Into<String>) {
        let node_id = node_id.into();
        if node_id.trim().is_empty() {
            return;
        }
        self.node_id_set
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(node_id);
    }

    /// 返回当前节点 ID 集合快照。
    ///
    /// # 返回
    /// 与内部集合解耦的快照，调用方无法绕过锁修改上下文状态。对应 Java:
    /// `SolonNodeIdHolder#getNodeIdSet`。
    #[must_use]
    pub fn get_node_id_set(&self) -> HashSet<String> {
        self.node_id_set
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}
