use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use agentscope_core::Model;
use agentscope_core::message::{ChatUsage, Msg};
use agentscope_core::model::{
    ChatResponse, GenerateOptions, ModelCapabilities, ModelError, ToolSchema, ToolsPayload,
};
use async_trait::async_trait;
use futures::Stream;

use super::ChatUsageTrackingHook;

/// 为 AgentScope 模型流补充逐 reasoning step usage 采集。
///
/// 对应 Java 的 `ChatUsageTrackingHook` 由 PostReasoningEvent 累加；当前
/// AgentScope-Rust 主链未完整分发该事件，因此本适配器在每次模型流结束时提交
/// 最后一个 usage 快照，避免对流式累计值重复求和。
pub(crate) struct UsageTrackingModel {
    delegate: Arc<dyn Model>,
    tracking_hook: Arc<ChatUsageTrackingHook>,
}

impl UsageTrackingModel {
    /// 创建保留底层模型全部能力的 usage 跟踪适配器。
    #[must_use]
    pub(crate) fn new(delegate: Arc<dyn Model>, tracking_hook: Arc<ChatUsageTrackingHook>) -> Self {
        Self {
            delegate,
            tracking_hook,
        }
    }
}

#[async_trait]
impl Model for UsageTrackingModel {
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.delegate.capabilities()
    }

    fn stream(
        &self,
        messages: &[Msg],
        tools: &[ToolSchema],
        options: Option<&GenerateOptions>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse, ModelError>> + Send>> {
        Box::pin(UsageTrackingStream {
            inner: self.delegate.stream(messages, tools, options),
            tracking_hook: Arc::clone(&self.tracking_hook),
            last_usage: None,
            committed: false,
        })
    }

    fn get_context_window_size(&self) -> u32 {
        self.delegate.get_context_window_size()
    }

    fn supports_native_structured_output(&self) -> bool {
        self.delegate.supports_native_structured_output()
    }

    fn convert_tools(&self, tools: &[ToolSchema]) -> ToolsPayload {
        self.delegate.convert_tools(tools)
    }

    fn clean_schema(&self, schema: serde_json::Value) -> serde_json::Value {
        self.delegate.clean_schema(schema)
    }

    fn supports_native_tools(&self) -> bool {
        self.delegate.supports_native_tools()
    }

    fn supports_vision(&self) -> bool {
        self.delegate.supports_vision()
    }

    fn supports_streaming(&self) -> bool {
        self.delegate.supports_streaming()
    }

    async fn warmup(&self) -> Result<(), ModelError> {
        self.delegate.warmup().await
    }
}

struct UsageTrackingStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatResponse, ModelError>> + Send>>,
    tracking_hook: Arc<ChatUsageTrackingHook>,
    last_usage: Option<ChatUsage>,
    committed: bool,
}

impl UsageTrackingStream {
    fn commit(&mut self) {
        if self.committed {
            return;
        }
        if let Some(usage) = self.last_usage.take() {
            self.tracking_hook.add_usage(&usage);
        }
        self.committed = true;
    }
}

impl Stream for UsageTrackingStream {
    type Item = Result<ChatResponse, ModelError>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(context) {
            Poll::Ready(Some(Ok(response))) => {
                if let Some(usage) = &response.usage {
                    self.last_usage = Some(usage.clone());
                }
                Poll::Ready(Some(Ok(response)))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => {
                self.commit();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for UsageTrackingStream {
    fn drop(&mut self) {
        self.commit();
    }
}
