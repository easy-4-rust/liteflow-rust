//! 对应 flow.element.Chain：按序执行 conditionList；
//! 决策表链路持有 routeItem（executeRoute 语义，2.12+）。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::element::condition::expect_bool;
use crate::slot::{Ctx, Frame};
use serde_json::Value;
use std::sync::Arc;

pub const DEFAULT_NAMESPACE: &str = "DEFAULT";

pub struct Chain {
    pub id: String,
    pub namespace: String,
    /// 构建该链的 EL 原文（2.16：getEl/getElMd5，execute2RespWithEL 缓存索引用）
    el: Option<String>,
    el_md5: Option<String>,
    /// 决策表链路的 route EL（对应 routeItem）
    route_item: Option<Arc<dyn Executable>>,
    condition_list: Vec<Arc<dyn Executable>>,
}

impl Chain {
    pub fn new(id: impl Into<String>, condition_list: Vec<Arc<dyn Executable>>) -> Self {
        Self {
            id: id.into(),
            namespace: DEFAULT_NAMESPACE.to_string(),
            el: None,
            el_md5: None,
            route_item: None,
            condition_list,
        }
    }

    /// setEL/getEl/getElMd5（2.16）
    pub fn set_el(&mut self, el: impl Into<String>, el_md5: impl Into<String>) {
        self.el = Some(el.into());
        self.el_md5 = Some(el_md5.into());
    }
    pub fn el(&self) -> Option<&str> {
        self.el.as_deref()
    }
    pub fn el_md5(&self) -> Option<&str> {
        self.el_md5.as_deref()
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn set_route_item(&mut self, route: Arc<dyn Executable>) {
        self.route_item = Some(route);
    }

    pub fn route_item(&self) -> Option<&Arc<dyn Executable>> {
        self.route_item.as_ref()
    }

    /// Chain.execute(slotIndex)：按序执行 conditionList
    pub async fn execute(&self, ctx: &Ctx) -> LFResult<Value> {
        self.execute_with_frame(ctx, &Frame::root()).await
    }

    /// 以指定执行路径帧执行（ChainBindWrapperCondition 下传 bind/loop 栈用，
    /// 对齐 Java 子链共享 Slot 的 conditionStack / loop 栈语义）
    pub async fn execute_with_frame(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        for cond in &self.condition_list {
            cond.execute(ctx, frame).await?;
        }
        Ok(Value::Null)
    }

    /// Chain.executeRoute(slotIndex)：求 route EL 的布尔结果
    pub async fn execute_route(&self, ctx: &Ctx) -> LFResult<bool> {
        let route = self
            .route_item
            .as_ref()
            .ok_or_else(|| LiteflowError::Custom(format!("chain[{}] has no route", self.id)))?;
        let v = route.execute(ctx, &Frame::root()).await?;
        expect_bool(route.id(), &v)
    }
}
