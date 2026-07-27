//! Nacos 解析器扩展配置。

use serde::{Deserialize, Serialize};

use crate::parser::nacos::exception::NacosException;

/// 保存 Nacos 服务、命名空间、配置坐标与两种认证方式。
///
/// serde 驼峰字段对齐 Jackson 配置。
/// 对应 Java: `com.yomahub.liteflow.parser.nacos.vo.NacosParserVO`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NacosParserVO {
    server_addr: String,
    namespace: String,
    data_id: String,
    group: String,
    access_key: String,
    secret_key: String,
    username: String,
    password: String,
}

impl Default for NacosParserVO {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:8848".to_string(),
            namespace: String::new(),
            data_id: "LiteFlow".to_string(),
            group: "LITE_FLOW_GROUP".to_string(),
            access_key: String::new(),
            secret_key: String::new(),
            username: String::new(),
            password: String::new(),
        }
    }
}

impl NacosParserVO {
    /// 返回 Nacos 服务地址。对应 Java `getServerAddr`。
    #[must_use]
    pub fn server_addr(&self) -> &str {
        &self.server_addr
    }

    /// 设置 Nacos 服务地址。对应 Java `setServerAddr`。
    pub fn set_server_addr(&mut self, server_addr: impl Into<String>) {
        self.server_addr = server_addr.into();
    }

    /// 返回 namespace。对应 Java `getNamespace`。
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// 设置 namespace。对应 Java `setNamespace`。
    pub fn set_namespace(&mut self, namespace: impl Into<String>) {
        self.namespace = namespace.into();
    }

    /// 返回 dataId。对应 Java `getDataId`。
    #[must_use]
    pub fn data_id(&self) -> &str {
        &self.data_id
    }

    /// 设置 dataId。对应 Java `setDataId`。
    pub fn set_data_id(&mut self, data_id: impl Into<String>) {
        self.data_id = data_id.into();
    }

    /// 返回 group。对应 Java `getGroup`。
    #[must_use]
    pub fn group(&self) -> &str {
        &self.group
    }

    /// 设置 group。对应 Java `setGroup`。
    pub fn set_group(&mut self, group: impl Into<String>) {
        self.group = group.into();
    }

    /// 返回 AccessKey。对应 Java `getAccessKey`。
    #[must_use]
    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    /// 设置 AccessKey。对应 Java `setAccessKey`。
    pub fn set_access_key(&mut self, access_key: impl Into<String>) {
        self.access_key = access_key.into();
    }

    /// 返回 SecretKey。对应 Java `getSecretKey`。
    #[must_use]
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }

    /// 设置 SecretKey。对应 Java `setSecretKey`。
    pub fn set_secret_key(&mut self, secret_key: impl Into<String>) {
        self.secret_key = secret_key.into();
    }

    /// 返回用户名。对应 Java `getUsername`。
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// 设置用户名。对应 Java `setUsername`。
    pub fn set_username(&mut self, username: impl Into<String>) {
        self.username = username.into();
    }

    /// 返回密码。对应 Java `getPassword`。
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }

    /// 设置密码。对应 Java `setPassword`。
    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = password.into();
    }

    /// 校验 SDK 建连和配置查询所需字段。
    ///
    /// Java 构造函数会为空字段填默认值；serde 默认和 `Default` 已保留该语义。
    pub fn validate(&self) -> Result<(), NacosException> {
        if self.server_addr.trim().is_empty() {
            return Err(NacosException::new("serverAddr is empty"));
        }
        if self.data_id.trim().is_empty() {
            return Err(NacosException::new("dataId is empty"));
        }
        if self.group.trim().is_empty() {
            return Err(NacosException::new("group is empty"));
        }
        if self.username.trim().is_empty() != self.password.trim().is_empty() {
            return Err(NacosException::new(
                "username and password must be configured together",
            ));
        }
        if self.access_key.trim().is_empty() != self.secret_key.trim().is_empty() {
            return Err(NacosException::new(
                "accessKey and secretKey must be configured together",
            ));
        }
        Ok(())
    }
}
