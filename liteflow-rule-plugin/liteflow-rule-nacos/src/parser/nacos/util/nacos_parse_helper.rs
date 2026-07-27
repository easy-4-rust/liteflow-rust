//! Nacos 配置读取与监听辅助对象。

use std::sync::Arc;

use liteflow_core::rule_plugin::RuleSourceWatcher;
use nacos_sdk::api::config::{
    ConfigChangeListener, ConfigResponse, ConfigService, ConfigServiceBuilder,
};
use nacos_sdk::api::props::ClientProps;
use tokio::sync::OnceCell;

use crate::parser::nacos::exception::NacosException;
use crate::parser::nacos::vo::NacosParserVO;

/// 创建并复用 Nacos 客户端，读取配置并安装原生变更监听。
///
/// 对应 Java: `com.yomahub.liteflow.parser.nacos.util.NacosParserHelper`。
#[derive(Debug, Clone)]
pub struct NacosParseHelper {
    config: NacosParserVO,
    service: Arc<OnceCell<ConfigService>>,
}

impl NacosParseHelper {
    /// 校验配置并创建惰性连接辅助对象。
    ///
    /// 实际网络连接在首次读取或监听时建立。
    /// 对应 Java `NacosParserHelper#NacosParserHelper`。
    pub fn new(config: NacosParserVO) -> Result<Self, NacosException> {
        config.validate()?;
        Ok(Self {
            config,
            service: Arc::new(OnceCell::new()),
        })
    }

    /// 读取 dataId/group 对应的配置文本。
    ///
    /// 对应 Java `NacosParserHelper#getContent`。
    pub async fn get_content(&self) -> Result<String, NacosException> {
        let response = self
            .service()
            .await?
            .get_config(
                self.config.data_id().to_string(),
                self.config.group().to_string(),
            )
            .await?;
        Ok(response.content().clone())
    }

    /// 校验配置内容非空白。
    ///
    /// 对应 Java `NacosParserHelper#checkContent`。
    pub fn check_content(&self, content: &str) -> Result<(), NacosException> {
        if content.trim().is_empty() {
            return Err(NacosException::new(format!(
                "the node[{}] value is empty",
                self.config.data_id()
            )));
        }
        Ok(())
    }

    /// 为 dataId/group 安装 Nacos 原生配置变化监听。
    ///
    /// SDK 通知到达后触发统一规则重载与删除对账。
    /// 对应 Java `NacosParserHelper#listener`。
    pub async fn listener(&self, watcher: RuleSourceWatcher) -> Result<(), NacosException> {
        let listener = Arc::new(ReloadConfigListener {
            watcher,
            runtime: tokio::runtime::Handle::current(),
        });
        self.service()
            .await?
            .add_listener(
                self.config.data_id().to_string(),
                self.config.group().to_string(),
                listener,
            )
            .await?;
        Ok(())
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &NacosParserVO {
        &self.config
    }

    async fn service(&self) -> Result<ConfigService, NacosException> {
        let service = self
            .service
            .get_or_try_init(|| async { self.build_service().await })
            .await?;
        Ok(service.clone())
    }

    async fn build_service(&self) -> Result<ConfigService, NacosException> {
        let mut props = ClientProps::new()
            .server_addr(self.config.server_addr())
            .namespace(self.config.namespace())
            .app_name("liteflow-rust")
            .env_first(false);

        let username_auth =
            !self.config.username().trim().is_empty() && !self.config.password().trim().is_empty();
        let aliyun_auth = !self.config.access_key().trim().is_empty()
            && !self.config.secret_key().trim().is_empty();

        if username_auth {
            props = props
                .auth_username(self.config.username())
                .auth_password(self.config.password());
            return ConfigServiceBuilder::new(props)
                .enable_auth_plugin_http()
                .build()
                .await
                .map_err(NacosException::from);
        }
        if aliyun_auth {
            props = props
                .auth_access_key(self.config.access_key())
                .auth_access_secret(self.config.secret_key());
            return ConfigServiceBuilder::new(props)
                .enable_auth_plugin_aliyun()
                .build()
                .await
                .map_err(NacosException::from);
        }

        ConfigServiceBuilder::new(props)
            .build()
            .await
            .map_err(NacosException::from)
    }
}

/// Nacos SDK 回调适配器；作为辅助对象的内部实现与主对象同文件。
struct ReloadConfigListener {
    watcher: RuleSourceWatcher,
    runtime: tokio::runtime::Handle,
}

impl ConfigChangeListener for ReloadConfigListener {
    fn notify(&self, _config_response: ConfigResponse) {
        let watcher = self.watcher.clone();
        self.runtime.spawn(async move {
            if let Err(error) = watcher.reload().await {
                eprintln!("[liteflow] nacos listener reload failed: {error}");
            }
        });
    }
}
