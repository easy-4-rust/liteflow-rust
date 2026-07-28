use std::collections::HashSet;
use std::sync::RwLock;

/// 保存 Spring/Vernal 扫描阶段发现的节点 ID。
///
/// Java 使用静态并发集合；Rust 将集合限制在一个 Vernal 应用上下文内，并用
/// `RwLock<HashSet<_>>` 提供等价并发语义。对应 Java:
/// `com.yomahub.liteflow.spring.process.holder.SpringNodeIdHolder`。
#[derive(Default)]
pub struct SpringNodeIdHolder {
    node_ids: RwLock<HashSet<String>>,
}

impl SpringNodeIdHolder {
    /// 创建空节点 ID 持有器。
    ///
    /// # 返回
    /// 当前上下文独享的节点集合。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 加入一个节点 ID。
    ///
    /// # 参数
    /// - `node_id`：EL 中引用的节点标识。
    ///
    /// 对应 Java: `SpringNodeIdHolder#addNodeId`。
    pub fn add_node_id(&self, node_id: impl Into<String>) {
        self.node_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(node_id.into());
    }

    /// 返回真实 Bean 名称。
    ///
    /// # 参数
    /// - `clazz`：Bean 类型诊断名，对应 Java `Class<?>`；
    /// - `bean_name`：容器提供的原始 Bean 名称。
    ///
    /// # 返回
    /// 此入口保持 Java 方法形状；Rust 无法从类型名反射 `@RefreshScope`，因此
    /// 未显式标记时原样返回。对应 Java:
    /// `SpringNodeIdHolder#getRealBeanName`。
    #[must_use]
    pub fn get_real_bean_name(&self, clazz: &str, bean_name: &str) -> String {
        let _ = clazz;
        bean_name.to_string()
    }

    /// 按显式刷新作用域元数据返回真实 Bean 名称。
    ///
    /// # 参数
    /// - `clazz`：Bean 类型诊断名；
    /// - `bean_name`：容器 Bean 名称；
    /// - `refresh_scoped`：注册定义是否等价于 Java `@RefreshScope`。
    ///
    /// # 返回
    /// 刷新作用域对象去除 `scopedTarget.` 前缀，否则原样返回。
    #[must_use]
    pub fn get_real_bean_name_with_scope(
        &self,
        clazz: &str,
        bean_name: &str,
        refresh_scoped: bool,
    ) -> String {
        if refresh_scoped {
            bean_name
                .strip_prefix("scopedTarget.")
                .unwrap_or(bean_name)
                .to_string()
        } else {
            self.get_real_bean_name(clazz, bean_name)
        }
    }

    /// 返回节点 ID 的确定性快照。
    ///
    /// # 返回
    /// 按字典序排序且去重的节点 ID。对应 Java:
    /// `SpringNodeIdHolder#getNodeIdSet`。
    #[must_use]
    pub fn get_node_id_set(&self) -> Vec<String> {
        let mut node_ids: Vec<_> = self
            .node_ids
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .cloned()
            .collect();
        node_ids.sort();
        node_ids
    }

    /// 清理当前上下文节点 ID。
    pub fn clean(&self) {
        self.node_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}
