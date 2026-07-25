//! 对应各 RulePlugin 的公共模式：fetch 规则文本 → 解析装载 → 轮询热刷新。

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// 规则文本格式（对应 JSON/XML/YML 三种 parser）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFormat {
    Json,
    Xml,
    Yml,
}

/// 变更检测指纹（FNV-1a）
pub fn fnv_fp(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:x}")
}

/// 规则源接口（对应 Nacos/Zk/Apollo/Redis/Etcd/SQL 插件的统一形态）
#[async_trait]
pub trait RuleSource: Send + Sync + 'static {
    /// 拉取规则文本与版本指纹（用于变更检测）
    async fn fetch(&self) -> LFResult<(String, String)>;
    /// 规则格式
    fn format(&self) -> RuleFormat;
    /// 源名称（日志用）
    fn name(&self) -> &str;
}

fn load_by_format(bus: &FlowBus, format: RuleFormat, text: &str) -> LFResult<Vec<String>> {
    match format {
        RuleFormat::Json => crate::parser::local_json_flow_el_parser::load_json_str(bus, text),
        RuleFormat::Xml => crate::parser::local_xml_flow_el_parser::load_xml_str(bus, text),
        RuleFormat::Yml => crate::parser::local_yml_flow_el_parser::load_yml_str(bus, text),
    }
}

/// 规则源监听器：先全量装载，再按间隔轮询，指纹变化时平滑热刷新
/// （对应各插件的 listen/refresh 机制）
#[derive(Clone)]
pub struct RuleSourceWatcher {
    bus: FlowBus,
    source: Arc<dyn RuleSource>,
}

impl RuleSourceWatcher {
    /// 初始装载
    pub async fn new(bus: FlowBus, source: Arc<dyn RuleSource>) -> LFResult<Self> {
        let (text, _) = source.fetch().await?;
        let ids = load_by_format(&bus, source.format(), &text)?;
        println!(
            "[liteflow] rule source {} loaded, {} chains",
            source.name(),
            ids.len()
        );
        Ok(Self { bus, source })
    }

    /// 启动后台轮询（abort 返回的 JoinHandle 即停止）
    pub fn watch(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut last_fp: Option<String> = None;
            loop {
                tokio::time::sleep(interval).await;
                match self.source.fetch().await {
                    Ok((text, fp)) => {
                        if last_fp.as_deref() == Some(fp.as_str()) {
                            continue;
                        }
                        match load_by_format(&self.bus, self.source.format(), &text) {
                            Ok(ids) => {
                                println!(
                                    "[liteflow] rule source {} reloaded, {} chains",
                                    self.source.name(),
                                    ids.len()
                                );
                                last_fp = Some(fp);
                            }
                            Err(e) => eprintln!(
                                "[liteflow] reload from {} failed: {e}",
                                self.source.name()
                            ),
                        }
                    }
                    Err(e) => eprintln!(
                        "[liteflow] fetch from {} failed: {e}",
                        self.source.name()
                    ),
                }
            }
        })
    }
}
