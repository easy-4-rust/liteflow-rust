//! 对应各 RulePlugin 的 listen/refresh 轮询热刷新机制。

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::rule_plugin::{RuleFormat, RuleSource};

/// 规则源监听器：先全量装载，再按间隔轮询并在指纹变化时热刷新。
#[derive(Clone)]
pub struct RuleSourceWatcher {
    bus: FlowBus,
    source: Arc<dyn RuleSource>,
    managed_chain_ids: Arc<RwLock<HashSet<String>>>,
    last_fingerprint: Arc<RwLock<String>>,
}

impl RuleSourceWatcher {
    /// 拉取规则源当前内容、解析并完成初始装载。
    pub async fn new(bus: FlowBus, source: Arc<dyn RuleSource>) -> LFResult<Self> {
        let (text, fingerprint) = source.fetch().await?;
        let ids = load_by_format(&bus, source.format(), &text)?;
        println!(
            "[liteflow] rule source {} loaded, {} chains",
            source.name(),
            ids.len()
        );
        Ok(Self {
            bus,
            source,
            managed_chain_ids: Arc::new(RwLock::new(ids.into_iter().collect())),
            last_fingerprint: Arc::new(RwLock::new(fingerprint)),
        })
    }

    /// 立即重新拉取规则，并对账删除本规则源不再提供的 Chain。
    ///
    /// 返回重新装载后的 Chain id。对应 Java 各规则插件监听器收到变更事件后的
    /// `changeChain`、`removeChain` 组合语义。
    pub async fn reload(&self) -> LFResult<Vec<String>> {
        let (text, fingerprint) = self.source.fetch().await?;
        let ids = load_by_format(&self.bus, self.source.format(), &text)?;
        self.reconcile_managed_chains(&ids);
        *self.last_fingerprint.write().expect("规则源指纹写锁中毒") = fingerprint;
        println!(
            "[liteflow] rule source {} reloaded, {} chains",
            self.source.name(),
            ids.len()
        );
        Ok(ids)
    }

    /// 卸载规则源已经删除或禁用的脚本节点。
    ///
    /// 对应 Java `ScriptPollingTask#run` 与订阅删除监听器调用
    /// `FlowBus#unloadScriptNode` 的语义。
    pub fn unload_script_node(&self, node_id: &str) {
        self.bus.unregister(node_id);
    }

    /// 启动后台轮询；调用返回句柄的 `abort` 可停止监听。
    #[must_use]
    pub fn watch(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                match self.source.fetch().await {
                    Ok((text, fingerprint)) => {
                        if *self.last_fingerprint.read().expect("规则源指纹读锁中毒") == fingerprint
                        {
                            continue;
                        }
                        match load_by_format(&self.bus, self.source.format(), &text) {
                            Ok(ids) => {
                                self.reconcile_managed_chains(&ids);
                                println!(
                                    "[liteflow] rule source {} reloaded, {} chains",
                                    self.source.name(),
                                    ids.len()
                                );
                                *self.last_fingerprint.write().expect("规则源指纹写锁中毒") =
                                    fingerprint;
                            }
                            Err(error) => eprintln!(
                                "[liteflow] reload from {} failed: {error}",
                                self.source.name()
                            ),
                        }
                    }
                    Err(error) => eprintln!(
                        "[liteflow] fetch from {} failed: {error}",
                        self.source.name()
                    ),
                }
            }
        })
    }

    fn reconcile_managed_chains(&self, current_ids: &[String]) {
        let current_ids: HashSet<String> = current_ids.iter().cloned().collect();
        let mut managed_chain_ids = self
            .managed_chain_ids
            .write()
            .expect("规则源 Chain 集合写锁中毒");
        for removed_id in managed_chain_ids.difference(&current_ids) {
            self.bus.remove_chain(removed_id);
        }
        *managed_chain_ids = current_ids;
    }
}

fn load_by_format(bus: &FlowBus, format: RuleFormat, text: &str) -> LFResult<Vec<String>> {
    match format {
        RuleFormat::Json => crate::parser::el::load_json_str(bus, text),
        RuleFormat::Xml => crate::parser::el::load_xml_str(bus, text),
        RuleFormat::Yml => crate::parser::el::load_yml_str(bus, text),
    }
}
