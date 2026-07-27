//! Apollo namespace 读取与 LiteFlow XML 聚合辅助对象。

use std::collections::BTreeMap;
use std::time::Duration;

use liteflow_core::parser::helper::NodeConvertHelper;
use liteflow_core::rule_plugin::RuleSourceWatcher;
use liteflow_core::util::RuleParsePluginUtil;

use crate::parser::apollo::exception::ApolloException;
use crate::parser::apollo::vo::ApolloParserConfigVO;

/// 通过 Apollo Config Service 读取 namespace 并生成完整规则 XML。
///
/// Java 版使用 Apollo Client SDK 的 `Config`；Rust 使用等价的
/// `/configfiles/json/{appId}/{cluster}/{namespace}` 协议。
/// 对应 Java: `com.yomahub.liteflow.parser.apollo.util.ApolloParseHelper`。
#[derive(Debug, Clone)]
pub struct ApolloParseHelper {
    config: ApolloParserConfigVO,
    config_service_url: String,
    app_id: String,
    cluster: String,
    ip: String,
}

impl ApolloParseHelper {
    /// 创建 Apollo 读取辅助对象。
    ///
    /// 参数与 Apollo SDK 所需的应用、集群和 Config Service 地址一一对应。
    /// 对应 Java `ApolloParseHelper#ApolloParseHelper`。
    pub fn new(
        config: ApolloParserConfigVO,
        config_service_url: impl Into<String>,
        app_id: impl Into<String>,
        cluster: impl Into<String>,
    ) -> Result<Self, ApolloException> {
        config.validate()?;
        let config_service_url = config_service_url.into();
        let app_id = app_id.into();
        let cluster = cluster.into();
        if config_service_url.trim().is_empty() {
            return Err(ApolloException::new("configServiceUrl is empty"));
        }
        if app_id.trim().is_empty() {
            return Err(ApolloException::new("appId is empty"));
        }
        if cluster.trim().is_empty() {
            return Err(ApolloException::new("cluster is empty"));
        }
        Ok(Self {
            config,
            config_service_url: normalize_base_url(&config_service_url),
            app_id,
            cluster,
            ip: "rust".to_string(),
        })
    }

    /// 设置 Config Service 请求中的客户端 IP。
    #[must_use]
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = ip.into();
        self
    }

    /// 读取全部 Chain 配置和可选 Script 配置并聚合为 XML。
    ///
    /// Chain key 按 `chainId:enable` 解析，Script key 按
    /// `id:type:name:language:enable` 解析。
    /// 对应 Java `ApolloParseHelper#getContent`。
    pub fn get_content(&self) -> Result<String, ApolloException> {
        let chain_config = self.fetch_namespace(self.config.chain_namespace())?;
        let chain_xml = chain_config
            .into_iter()
            .map(|(key, value)| RuleParsePluginUtil::parse_chain_key(&key).to_el_xml(&value))
            .collect::<String>();

        let script_xml = match self
            .config
            .script_namespace()
            .filter(|namespace| !namespace.trim().is_empty())
        {
            Some(namespace) => {
                let nodes = self
                    .fetch_namespace(namespace)?
                    .into_iter()
                    .map(|(key, script)| {
                        let mut node = NodeConvertHelper::convert(&key).ok_or_else(|| {
                            ApolloException::new(format!("apollo script key[{key}] is invalid"))
                        })?;
                        node.set_script(script);
                        Ok(RuleParsePluginUtil::to_script_xml(&node))
                    })
                    .collect::<Result<String, ApolloException>>()?;
                if nodes.is_empty() {
                    String::new()
                } else {
                    format!("<nodes>{nodes}</nodes>")
                }
            }
            None => String::new(),
        };

        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><flow>{script_xml}{chain_xml}</flow>"
        ))
    }

    /// 安装 Apollo 配置变化监听。
    ///
    /// Java SDK 通过长轮询触发 `ConfigChangeListener`；Rust 的 Config Service
    /// 协议由通用 watcher 周期拉取指纹，并在变化后执行相同的重装载与删除对账。
    /// 返回句柄可通过 `abort` 停止监听。
    /// 对应 Java `ApolloParseHelper#listenApollo`。
    #[must_use]
    pub fn listen_apollo(
        &self,
        watcher: RuleSourceWatcher,
        interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        watcher.watch(interval)
    }

    /// 读取一个 namespace 的全部配置项。
    ///
    /// Config Service 的 `configfiles/json` 响应体本身就是配置 Map，
    /// 不存在 Portal API 的 `configurations` 外层。
    pub fn fetch_namespace(
        &self,
        namespace: &str,
    ) -> Result<BTreeMap<String, String>, ApolloException> {
        let url = format!(
            "{}/configfiles/json/{}/{}/{}?ip={}",
            self.config_service_url, self.app_id, self.cluster, namespace, self.ip
        );
        let mut response = ureq::get(&url)
            .call()
            .map_err(|error| ApolloException::new(format!("apollo fetch error: {error}")))?;
        response
            .body_mut()
            .read_json::<BTreeMap<String, String>>()
            .map_err(|error| ApolloException::new(format!("apollo parse error: {error}")))
    }

    /// 返回 Config Service 根地址。
    #[must_use]
    pub fn config_service_url(&self) -> &str {
        &self.config_service_url
    }

    /// 返回 Apollo 应用 id。
    #[must_use]
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// 返回 Apollo 集群。
    #[must_use]
    pub fn cluster(&self) -> &str {
        &self.cluster
    }

    /// 返回客户端 IP。
    #[must_use]
    pub fn ip(&self) -> &str {
        &self.ip
    }
}

fn normalize_base_url(value: &str) -> String {
    let value = value.trim().trim_end_matches('/');
    if value.starts_with("http://") || value.starts_with("https://") {
        value.to_string()
    } else {
        format!("http://{value}")
    }
}
