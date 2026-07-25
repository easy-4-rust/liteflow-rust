//! 对应 core.FlowExecutor：执行入口。

use crate::exception::LiteflowError;
use crate::flow::flow_bus::FlowBus;
use crate::flow::liteflow_response::LiteflowResponse;
use crate::slot::{Ctx, Slot, databus::gen_request_id};
use md5::Digest as _;
use serde::Serialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

pub struct FlowExecutor {
    bus: FlowBus,
}

impl FlowExecutor {
    pub fn new(bus: FlowBus) -> Self {
        Self { bus }
    }

    /// execute2Resp(chainId)
    pub async fn execute(&self, chain_id: &str) -> LiteflowResponse {
        self.execute_with(chain_id, Value::Null, Vec::<(String, Arc<dyn Any + Send + Sync>)>::new())
            .await
    }

    /// execute2Resp(chainId, requestData)
    pub async fn execute_with_data(&self, chain_id: &str, input: impl Serialize) -> LiteflowResponse {
        let v = serde_json::to_value(input).unwrap_or(Value::Null);
        self.execute_with(chain_id, v, Vec::<(String, Arc<dyn Any + Send + Sync>)>::new())
            .await
    }

    /// execute2Resp(chainId, requestData, contextBeanArray)
    pub async fn execute_with(
        &self,
        chain_id: &str,
        input: Value,
        beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LiteflowResponse {
        self.execute_with_option(chain_id, input, crate::core::execute_option::ExecuteOption {
            context_beans: beans,
            ..Default::default()
        })
        .await
    }

    /// execute2RespWithRid(chainId, requestData, requestId, contextBeans)（2.16：
    /// 组件内 invoke2Resp 语义——以同一 requestId 执行子链）
    pub async fn execute_with_rid(
        &self,
        chain_id: &str,
        input: Value,
        request_id: impl Into<String>,
        beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    ) -> LiteflowResponse {
        self.execute_with_option(chain_id, input, crate::core::execute_option::ExecuteOption {
            request_id: Some(request_id.into()),
            context_beans: beans,
            ..Default::default()
        })
        .await
    }

    /// execute2Resp(chainId, requestData, ExecuteOption)（2.16 新增入口：
    /// requestId / conversationId / eventListener / contextBeans）
    pub async fn execute_with_option(
        &self,
        chain_id: &str,
        input: Value,
        option: crate::core::execute_option::ExecuteOption,
    ) -> LiteflowResponse {
        let request_id = option
            .request_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(gen_request_id);
        let mut slot = Slot::new(request_id, chain_id, input);
        slot.conversation_id = option.resolve_conversation_id();
        let slot = Arc::new(slot);
        for (name, bean) in option.context_beans {
            slot.beans.insert(name, bean);
        }
        let ctx = Ctx::new(slot.clone());
        if let Some(l) = &option.event_listener {
            crate::flow::flow_event_publisher::FlowEventPublisher::set_listener(&ctx, l.clone());
        }

        {
            let hooks = self.bus.lifecycle.read().unwrap();
            for h in &hooks.flow_execute {
                h.post_process_before_flow_execute(chain_id).await;
            }
            for h in &hooks.chain_execute {
                h.post_process_before_chain_execute(chain_id).await;
            }
        }

        let chain = match self.bus.get_chain(chain_id) {
            Some(c) => c,
            None => {
                return LiteflowResponse::new(
                    slot,
                    false,
                    format!("chain not found: {chain_id}"),
                    Some(LiteflowError::ChainNotFound(chain_id.to_string()).to_string()),
                )
            }
        };

        let result = match chain.execute(&ctx).await {
            Ok(_) | Err(LiteflowError::ChainEnd) => {
                LiteflowResponse::new(slot, true, "success".into(), None)
            }
            Err(e) => LiteflowResponse::new(slot, false, e.to_string(), Some(e.to_string())),
        };
        {
            let hooks = self.bus.lifecycle.read().unwrap();
            for h in &hooks.chain_execute {
                h.post_process_after_chain_execute(chain_id).await;
            }
            for h in &hooks.flow_execute {
                h.post_process_after_flow_execute(chain_id).await;
            }
        }
        result
    }

    /// execute2RespWithEL(elStr)（2.16 新增：直接执行 EL 表达式）
    pub async fn execute_with_el(&self, el_str: &str) -> LiteflowResponse {
        self.execute_with_el_full(el_str, Value::Null, None).await
    }

    /// execute2RespWithEL(elStr, param)
    pub async fn execute_with_el_data(&self, el_str: &str, input: impl Serialize) -> LiteflowResponse {
        let v = serde_json::to_value(input).unwrap_or(Value::Null);
        self.execute_with_el_full(el_str, v, None).await
    }

    /// execute2RespWithEL(elStr, param, requestId)
    pub async fn execute_with_el_full(
        &self,
        el_str: &str,
        input: Value,
        request_id: Option<String>,
    ) -> LiteflowResponse {
        // 规范化 EL（对应 ElRegexUtil.normalize：单引号→双引号、去空白、末尾保留一个分号）
        let normalized = crate::util::el_regex::normalize_el(el_str);
        let el_md5 = format!("{:x}", md5::Md5::digest(normalized.as_bytes()));

        let chain_id = match self.bus.get_chain_id_by_el_md5(&el_md5) {
            Some(id) => id,
            None => {
                // 匿名链路：UUID 语义的唯一 chainId
                let id = format!("anon_{:x}", md5::Md5::digest(
                    format!("{}-{}", normalized, gen_request_id()).as_bytes()
                ));
                if let Err(e) = self.bus.add_chain_anonymous(&id, &normalized, el_md5.clone()) {
                    let slot = Arc::new(Slot::new(gen_request_id(), &id, input));
                    return LiteflowResponse::new(slot, false, e.to_string(), Some(e.to_string()));
                }
                id
            }
        };
        match request_id {
            Some(rid) => {
                self.execute_with_rid(&chain_id, input, rid, Vec::<(String, Arc<dyn Any + Send + Sync>)>::new()).await
            }
            None => self.execute_with(&chain_id, input, Vec::<(String, Arc<dyn Any + Send + Sync>)>::new()).await,
        }
    }

    /// executeRouteChain(namespace, param, contextBeans)：
    /// 并行求值 namespace 下所有决策表链路的 route EL，
    /// 命中的链路并行执行 body，返回各链路的响应（对应 doExecuteWithRoute）。
    pub async fn execute_route_chain(
        &self,
        namespace: Option<&str>,
        input: impl Serialize,
    ) -> crate::exception::LFResult<Vec<LiteflowResponse>> {
        let namespace = namespace.unwrap_or(crate::flow::element::chain::DEFAULT_NAMESPACE);
        let v = serde_json::to_value(input).unwrap_or(Value::Null);

        let route_chains: Vec<Arc<crate::flow::element::chain::Chain>> = self
            .bus
            .chain_ids()
            .into_iter()
            .filter_map(|id| self.bus.get_chain(&id))
            .filter(|c| c.namespace == namespace && c.route_item().is_some())
            .collect();
        if route_chains.is_empty() {
            return Err(LiteflowError::RouteChainNotFound(namespace.to_string()));
        }

        // 并行求 route EL（每条链路独立 slot，同 requestId 语义）
        let mut set: tokio::task::JoinSet<(Arc<crate::flow::element::chain::Chain>, bool)> =
            tokio::task::JoinSet::new();
        for chain in route_chains {
            let v = v.clone();
            set.spawn(async move {
                let slot = Arc::new(Slot::new(gen_request_id(), chain.id.clone(), v));
                let ctx = Ctx::new(slot);
                let matched = chain.execute_route(&ctx).await.unwrap_or(false);
                (chain, matched)
            });
        }
        let mut matched = Vec::new();
        while let Some(Ok((chain, ok))) = set.join_next().await {
            if ok {
                matched.push(chain);
            }
        }
        if matched.is_empty() {
            return Err(LiteflowError::NoMatchedRouteChain);
        }

        // 命中的链路并行执行 body
        let mut set: tokio::task::JoinSet<LiteflowResponse> = tokio::task::JoinSet::new();
        for chain in matched {
            let v = v.clone();
            set.spawn(async move {
                let slot = Arc::new(Slot::new(gen_request_id(), chain.id.clone(), v));
                let ctx = Ctx::new(slot.clone());
                match chain.execute(&ctx).await {
                    Ok(_) | Err(LiteflowError::ChainEnd) => {
                        LiteflowResponse::new(slot, true, "success".into(), None)
                    }
                    Err(e) => {
                        LiteflowResponse::new(slot, false, e.to_string(), Some(e.to_string()))
                    }
                }
            });
        }
        let mut responses = Vec::new();
        while let Some(Ok(resp)) = set.join_next().await {
            responses.push(resp);
        }
        Ok(responses)
    }

    /// execute2Resp(chainId, requestData, timeout, unit)
    pub async fn execute_timeout(
        &self,
        chain_id: &str,
        input: impl Serialize,
        timeout: Duration,
    ) -> LiteflowResponse {
        match tokio::time::timeout(timeout, self.execute_with_data(chain_id, input)).await {
            Ok(resp) => resp,
            Err(_) => {
                let slot = Arc::new(Slot::new(gen_request_id(), chain_id, Value::Null));
                LiteflowResponse::new(
                    slot,
                    false,
                    "chain execute timeout".into(),
                    Some("chain execute timeout".into()),
                )
            }
        }
    }
}
