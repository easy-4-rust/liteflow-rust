//! 对应 Java: com.yomahub.liteflow.meta.LiteflowMetaOperator

use std::sync::Arc;

use crate::el::{El, NodeRef, parse_el};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::FlowBus;
use crate::flow::element::chain::Chain;

/// 以显式 FlowBus 作用域统一查询和更新 LiteFlow 元数据。
///
/// Java 通过全局静态 FlowBus 访问；Rust 显式持有总线，避免多个运行时相互污染。
#[derive(Clone)]
pub struct LiteflowMetaOperator {
    flow_bus: FlowBus,
    reload_all: Option<Arc<dyn Fn() -> LFResult<()> + Send + Sync>>,
}

impl LiteflowMetaOperator {
    /// 创建元数据操作器。
    #[must_use]
    pub fn new(flow_bus: FlowBus) -> Self {
        Self {
            flow_bus,
            reload_all: None,
        }
    }

    /// 绑定规则源全量刷新函数。
    #[must_use]
    pub fn with_reload_all(
        mut self,
        reload_all: Arc<dyn Fn() -> LFResult<()> + Send + Sync>,
    ) -> Self {
        self.reload_all = Some(reload_all);
        self
    }

    /// 通过 chain id 获取 Chain。
    #[must_use]
    pub fn get_chain(&self, chain_id: &str) -> Option<Arc<Chain>> {
        self.flow_bus.get_chain(chain_id)
    }

    /// 查找包含指定 node id 的全部 Chain。
    #[must_use]
    pub fn get_chains_contains_node_id(&self, node_id: &str) -> Vec<Arc<Chain>> {
        self.flow_bus
            .chain_ids()
            .into_iter()
            .filter_map(|chain_id| self.get_chain(&chain_id))
            .filter(|chain| {
                Self::get_nodes_from_chain(chain)
                    .iter()
                    .any(|node| node.id == node_id)
            })
            .collect()
    }

    /// 从配置的规则源刷新全部 Chain。
    pub fn reload_all_chain(&self) -> LFResult<()> {
        self.reload_all.as_ref().ok_or_else(|| {
            LiteflowError::FlowExecutorNotInit(
                "reload-all rule source is not configured".to_string(),
            )
        })?()
    }

    /// 热刷新单个 Chain。
    pub fn reload_one_chain(&self, chain_id: &str, el: &str) -> LFResult<()> {
        self.flow_bus.reload_chain(chain_id, el)
    }

    /// 卸载一个 Chain。
    pub fn remove_chain(&self, chain_id: &str) {
        self.flow_bus.remove_chain(chain_id);
    }

    /// 卸载多个 Chain。
    pub fn remove_chains<'a>(&self, chain_ids: impl IntoIterator<Item = &'a str>) {
        for chain_id in chain_ids {
            self.remove_chain(chain_id);
        }
    }

    /// 从 Chain 的 EL 元数据递归提取 Node 引用。
    #[must_use]
    pub fn get_nodes_from_chain(chain: &Chain) -> Vec<NodeRef> {
        chain
            .el()
            .and_then(|el| parse_el(el).ok())
            .map(|el| {
                let mut nodes = Vec::new();
                collect_nodes(&el, &mut nodes);
                nodes
            })
            .unwrap_or_default()
    }

    /// 返回指定 Chain 的全部 Node 引用。
    #[must_use]
    pub fn get_nodes(&self, chain_id: &str) -> Vec<NodeRef> {
        self.get_chain(chain_id)
            .map(|chain| Self::get_nodes_from_chain(&chain))
            .unwrap_or_default()
    }

    /// 返回 Chain 中指定 id 的全部 Node 引用。
    #[must_use]
    pub fn get_nodes_by_id(&self, chain_id: &str, node_id: &str) -> Vec<NodeRef> {
        self.get_nodes(chain_id)
            .into_iter()
            .filter(|node| node.id == node_id)
            .collect()
    }

    /// 按同名节点出现下标返回 Node。
    #[must_use]
    pub fn get_node_by_index(
        &self,
        chain_id: &str,
        node_id: &str,
        index: usize,
    ) -> Option<NodeRef> {
        self.get_nodes_by_id(chain_id, node_id)
            .into_iter()
            .nth(index)
    }

    /// 返回 node id 在所有 Chain 中的引用。
    #[must_use]
    pub fn get_nodes_in_all_chain(&self, node_id: &str) -> Vec<NodeRef> {
        self.flow_bus
            .chain_ids()
            .into_iter()
            .flat_map(|chain_id| self.get_nodes_by_id(&chain_id, node_id))
            .collect()
    }

    /// 热刷新脚本，保留原语言和节点类别。
    pub fn reload_script(&self, node_id: &str, script: &str) -> LFResult<()> {
        self.flow_bus.reload_script(node_id, script)
    }
}

fn collect_nodes(el: &El, nodes: &mut Vec<NodeRef>) {
    match el {
        El::Node(node) => nodes.push(node.clone()),
        El::Boolean(_) => {}
        El::Then(items) | El::And(items) | El::Or(items) => {
            for item in items {
                collect_nodes(item, nodes);
            }
        }
        El::When { items, .. } => {
            for item in items {
                collect_nodes(item, nodes);
            }
        }
        El::If {
            cond,
            then,
            elifs,
            els,
        } => {
            collect_nodes(cond, nodes);
            collect_nodes(then, nodes);
            for (condition, body) in elifs {
                collect_nodes(condition, nodes);
                collect_nodes(body, nodes);
            }
            if let Some(els) = els {
                collect_nodes(els, nodes);
            }
        }
        El::Switch {
            node,
            targets,
            default,
        } => {
            collect_nodes(node, nodes);
            for target in targets {
                collect_nodes(target, nodes);
            }
            if let Some(default) = default {
                collect_nodes(default, nodes);
            }
        }
        El::For {
            node, body, brk, ..
        }
        | El::While {
            node, body, brk, ..
        }
        | El::Iter {
            node, body, brk, ..
        } => {
            collect_nodes(node, nodes);
            collect_nodes(body, nodes);
            if let Some(brk) = brk {
                collect_nodes(brk, nodes);
            }
        }
        El::ForCount { body, brk, .. } => {
            collect_nodes(body, nodes);
            if let Some(brk) = brk {
                collect_nodes(brk, nodes);
            }
        }
        El::Catch { body, do_ } => {
            collect_nodes(body, nodes);
            if let Some(do_) = do_ {
                collect_nodes(do_, nodes);
            }
        }
        El::Not(item) | El::Pre(item) | El::Fin(item) | El::Mods(item, _) => {
            collect_nodes(item, nodes);
        }
    }
}
