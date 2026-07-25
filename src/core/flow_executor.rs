//! 对应 core.FlowExecutor：执行入口。

use crate::exception::LiteflowError;
use crate::flow::flow_bus::FlowBus;
use crate::flow::liteflow_response::LiteflowResponse;
use crate::slot::{databus::gen_request_id, Ctx, Slot};
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
        let slot = Arc::new(Slot::new(gen_request_id(), chain_id, input));
        for (name, bean) in beans {
            slot.beans.insert(name, bean);
        }
        let ctx = Ctx::new(slot.clone());

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
