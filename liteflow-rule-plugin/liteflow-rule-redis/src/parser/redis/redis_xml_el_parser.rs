//! Redis Hash XML EL 规则解析源。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::parser::helper::NodeConvertHelper;
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};
use liteflow_core::util::RuleParsePluginUtil;

use super::mode::{RClient, RedisParserHelper};
use super::vo::RedisParserVO;

/// 从 Chain/Script Hash 聚合 LiteFlow XML 的 Redis 解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.redis.RedisXmlELParser`。
#[derive(Debug, Clone)]
pub struct RedisXmlELParser {
    config: RedisParserVO,
    chain_client: RClient,
    script_client: Option<RClient>,
}

impl RedisXmlELParser {
    /// 校验扩展配置并创建解析器。对应 Java `RedisXmlELParser#RedisXmlELParser`。
    pub fn new(config: RedisParserVO) -> LFResult<Self> {
        config.validate().map_err(LiteflowError::Rule)?;
        let chain_client =
            <Self as RedisParserHelper>::get_redis_client(&config, config.chain_data_base)?
                .ok_or_else(|| {
                    LiteflowError::Rule("ruleSourceExtData chainDataBase is blank".to_string())
                })?;
        let script_client = if config.script_key.is_some() {
            <Self as RedisParserHelper>::get_redis_client(&config, config.script_data_base)?
        } else {
            None
        };
        Ok(Self {
            config,
            chain_client,
            script_client,
        })
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &RedisParserVO {
        &self.config
    }
}

impl RedisParserHelper for RedisXmlELParser {
    /// 聚合 Chain Hash 与可选 Script Hash。
    ///
    /// 对应 Java 两种模式的 `getContent` 共同初始装载语义。
    fn get_content(&self) -> LFResult<String> {
        let chain_key = self.config.chain_key.as_deref().ok_or_else(|| {
            LiteflowError::Rule("ruleSourceExtData chainKey is blank".to_string())
        })?;
        let mut chains: Vec<_> = self.chain_client.get_map(chain_key)?.into_iter().collect();
        chains.sort_by(|left, right| left.0.cmp(&right.0));
        let chain_xml = chains
            .into_iter()
            .filter(|(_, value)| !value.trim().is_empty())
            .map(|(field, value)| RuleParsePluginUtil::parse_chain_key(&field).to_el_xml(&value))
            .collect::<String>();

        let script_xml = match (&self.script_client, self.config.script_key.as_deref()) {
            (Some(client), Some(script_key)) => {
                let mut scripts: Vec<_> = client.get_map(script_key)?.into_iter().collect();
                scripts.sort_by(|left, right| left.0.cmp(&right.0));
                let nodes = scripts
                    .into_iter()
                    .map(|(field, script)| {
                        let mut node = NodeConvertHelper::convert(&field).ok_or_else(|| {
                            LiteflowError::Rule(format!(
                                "The name of the redis field [{field}] in scriptKey [{script_key}] is invalid"
                            ))
                        })?;
                        node.set_script(script);
                        Ok(RuleParsePluginUtil::to_script_xml(&node))
                    })
                    .collect::<LFResult<String>>()?;
                (!nodes.is_empty())
                    .then(|| format!("<nodes>{nodes}</nodes>"))
                    .unwrap_or_default()
            }
            _ => String::new(),
        };
        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><flow>{script_xml}{chain_xml}</flow>"
        ))
    }
}

#[async_trait]
impl RuleSource for RedisXmlELParser {
    /// 在线程池中读取同步 Redis Hash 并返回 XML 指纹。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let parser = self.clone();
        let text = tokio::task::spawn_blocking(move || parser.get_content())
            .await
            .map_err(|error| LiteflowError::Rule(format!("redis task error: {error}")))??;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "redis"
    }
}
