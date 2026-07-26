//! 对应各 RulePlugin 的 listen/refresh 轮询热刷新机制。

use std::sync::Arc;
use std::time::Duration;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::rule_plugin::{RuleFormat, RuleSource};

/// 规则源监听器：先全量装载，再按间隔轮询并在指纹变化时热刷新。
#[derive(Clone)]
pub struct RuleSourceWatcher {
    bus: FlowBus,
    source: Arc<dyn RuleSource>,
}

impl RuleSourceWatcher {
    /// 拉取规则源当前内容、解析并完成初始装载。
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

    /// 启动后台轮询；调用返回句柄的 `abort` 可停止监听。
    #[must_use]
    pub fn watch(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut last_fp: Option<String> = None;
            loop {
                tokio::time::sleep(interval).await;
                match self.source.fetch().await {
                    Ok((text, fingerprint)) => {
                        if last_fp.as_deref() == Some(fingerprint.as_str()) {
                            continue;
                        }
                        match load_by_format(&self.bus, self.source.format(), &text) {
                            Ok(ids) => {
                                println!(
                                    "[liteflow] rule source {} reloaded, {} chains",
                                    self.source.name(),
                                    ids.len()
                                );
                                last_fp = Some(fingerprint);
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
}

fn load_by_format(bus: &FlowBus, format: RuleFormat, text: &str) -> LFResult<Vec<String>> {
    match format {
        RuleFormat::Json => crate::parser::el::load_json_str(bus, text),
        RuleFormat::Xml => crate::parser::el::load_xml_str(bus, text),
        RuleFormat::Yml => crate::parser::el::load_yml_str(bus, text),
    }
}
