//! Etcd 客户端封装。

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use etcd_client::{
    ConnectOptions, EventType, GetOptions, PutOptions, SortOrder, SortTarget, WatchOptions,
};
use tokio::sync::Mutex;

use super::exception::EtcdException;

/// 封装 Etcd KV、前缀查询和 Watch 生命周期。
///
/// 对应 Java: `com.yomahub.liteflow.parser.etcd.EtcdClient`。
#[derive(Clone)]
pub struct EtcdClient {
    endpoints: Vec<String>,
    user: Option<String>,
    password: Option<String>,
    namespace: Option<String>,
    client: Arc<Mutex<Option<etcd_client::Client>>>,
    watch_cache: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl Debug for EtcdClient {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EtcdClient")
            .field("endpoints", &self.endpoints)
            .field("user", &self.user)
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
}

impl EtcdClient {
    /// 保存连接参数并创建惰性客户端。
    ///
    /// 真实连接在第一次 KV 或 Watch 操作时建立。
    /// 对应 Java `EtcdClient#EtcdClient`。
    #[must_use]
    pub fn new(
        endpoints: Vec<String>,
        namespace: Option<String>,
        user: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            endpoints,
            user,
            password,
            namespace,
            client: Arc::new(Mutex::new(None)),
            watch_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 关闭全部 Watch，并丢弃当前连接。
    ///
    /// 后续操作可按保存的连接参数重新连接。对应 Java `EtcdClient#close`。
    pub async fn close(&self) {
        let mut watches = self.watch_cache.lock().await;
        for (_, handle) in watches.drain() {
            handle.abort();
        }
        self.client.lock().await.take();
    }

    /// 获取节点值；节点不存在时返回 `None`。
    ///
    /// 参数 `key` 对应 Java `key`。对应 Java `EtcdClient#get`。
    pub async fn get(&self, key: &str) -> Result<Option<String>, EtcdException> {
        let mut client = self.client().await?;
        let response = client
            .get(self.physical_key(key), None)
            .await
            .map_err(EtcdException::from)?;
        match response.kvs().first() {
            Some(item) => item
                .value_str()
                .map(|value| Some(value.to_string()))
                .map_err(EtcdException::from),
            None => Ok(None),
        }
    }

    /// 写入节点并返回旧值；此前不存在时返回 `None`。
    ///
    /// 参数与 Java `key`、`value` 一一对应。对应 Java `EtcdClient#put`。
    pub async fn put(&self, key: &str, value: &str) -> Result<Option<String>, EtcdException> {
        let mut client = self.client().await?;
        let response = client
            .put(
                self.physical_key(key),
                value,
                Some(PutOptions::new().with_prev_key()),
            )
            .await
            .map_err(EtcdException::from)?;
        match response.prev_key() {
            Some(item) => item
                .value_str()
                .map(|value| Some(value.to_string()))
                .map_err(EtcdException::from),
            None => Ok(None),
        }
    }

    /// 返回指定前缀下去重且按 key 升序排列的子节点名称。
    ///
    /// 参数 `prefix`、`separator` 对应 Java 同名参数。
    /// 对应 Java `EtcdClient#getChildrenKeys`。
    pub async fn get_children_keys(
        &self,
        prefix: &str,
        separator: &str,
    ) -> Result<Vec<String>, EtcdException> {
        let mut client = self.client().await?;
        let physical_prefix = self.physical_key(prefix);
        let response = client
            .get(
                physical_prefix,
                Some(
                    GetOptions::new()
                        .with_prefix()
                        .with_sort(SortTarget::Key, SortOrder::Ascend),
                ),
            )
            .await
            .map_err(EtcdException::from)?;
        let mut children = Vec::new();
        for item in response.kvs() {
            let full_path = self.logical_key(item.key_str().map_err(EtcdException::from)?);
            if let Some(name) = get_sub_node_key_name(prefix, full_path, separator)
                && !children.contains(&name)
            {
                children.push(name);
            }
        }
        Ok(children)
    }

    /// 监听单节点增删改。
    ///
    /// 更新回调参数依次为逻辑路径和值；删除回调参数为逻辑路径。
    /// 对应 Java `EtcdClient#watchDataChange`。
    pub async fn watch_data_change<U, D>(
        &self,
        key: &str,
        update_handler: U,
        delete_handler: D,
    ) -> Result<(), EtcdException>
    where
        U: Fn(String, String) + Send + Sync + 'static,
        D: Fn(String) + Send + Sync + 'static,
    {
        self.watch(key, false, update_handler, delete_handler).await
    }

    /// 监听前缀下全部子节点增删改。
    ///
    /// 对应 Java `EtcdClient#watchChildChange`。
    pub async fn watch_child_change<U, D>(
        &self,
        key: &str,
        update_handler: U,
        delete_handler: D,
    ) -> Result<(), EtcdException>
    where
        U: Fn(String, String) + Send + Sync + 'static,
        D: Fn(String) + Send + Sync + 'static,
    {
        self.watch(key, true, update_handler, delete_handler).await
    }

    /// 取消指定逻辑 key 的 Watch。对应 Java `EtcdClient#watchClose`。
    pub async fn watch_close(&self, key: &str) {
        if let Some(handle) = self.watch_cache.lock().await.remove(key) {
            handle.abort();
        }
    }

    async fn watch<U, D>(
        &self,
        key: &str,
        prefix: bool,
        update_handler: U,
        delete_handler: D,
    ) -> Result<(), EtcdException>
    where
        U: Fn(String, String) + Send + Sync + 'static,
        D: Fn(String) + Send + Sync + 'static,
    {
        self.watch_close(key).await;
        let mut client = self.client().await?;
        let options = prefix.then(|| WatchOptions::new().with_prefix());
        let mut stream = client
            .watch(self.physical_key(key), options)
            .await
            .map_err(EtcdException::from)?;
        let namespace = self.namespace.clone();
        let handle = tokio::spawn(async move {
            while let Ok(Some(response)) = stream.message().await {
                for event in response.events() {
                    let Some(kv) = event.kv() else {
                        continue;
                    };
                    let Ok(path) = kv.key_str() else {
                        continue;
                    };
                    let path = strip_namespace(namespace.as_deref(), path).to_string();
                    match event.event_type() {
                        EventType::Put => {
                            if let Ok(value) = kv.value_str() {
                                update_handler(path, value.to_string());
                            }
                        }
                        EventType::Delete => delete_handler(path),
                    }
                }
            }
            // etcd-client 0.19 将请求端与响应端合并为 WatchStream。
            // 任务结束或被 abort 时直接丢弃 stream，即关闭对应 gRPC Watch。
        });
        self.watch_cache
            .lock()
            .await
            .insert(key.to_string(), handle);
        Ok(())
    }

    async fn client(&self) -> Result<etcd_client::Client, EtcdException> {
        let mut client = self.client.lock().await;
        if let Some(client) = client.as_ref() {
            return Ok(client.clone());
        }
        let options = match (&self.user, &self.password) {
            (Some(user), Some(password))
                if !user.trim().is_empty() && !password.trim().is_empty() =>
            {
                Some(ConnectOptions::new().with_user(user, password))
            }
            _ => None,
        };
        let connected = etcd_client::Client::connect(self.endpoints.clone(), options)
            .await
            .map_err(EtcdException::from)?;
        *client = Some(connected.clone());
        Ok(connected)
    }

    fn physical_key(&self, key: &str) -> String {
        match self.namespace.as_deref().filter(|value| !value.is_empty()) {
            Some(namespace) => format!("{namespace}{key}"),
            None => key.to_string(),
        }
    }

    fn logical_key<'a>(&self, key: &'a str) -> &'a str {
        strip_namespace(self.namespace.as_deref(), key)
    }
}

fn strip_namespace<'a>(namespace: Option<&str>, key: &'a str) -> &'a str {
    namespace
        .filter(|value| !value.is_empty())
        .and_then(|namespace| key.strip_prefix(namespace))
        .unwrap_or(key)
}

fn get_sub_node_key_name(prefix: &str, full_path: &str, separator: &str) -> Option<String> {
    if prefix.len() > full_path.len() {
        return None;
    }
    let path_without_prefix = &full_path[prefix.len()..];
    Some(
        if path_without_prefix.contains(separator) {
            path_without_prefix.get(1..).unwrap_or_default()
        } else {
            path_without_prefix
        }
        .to_string(),
    )
}
