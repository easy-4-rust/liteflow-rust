//! 对应 core.ExecuteOption（2.16 新增）：一次执行的选项。
//! requestId / conversationId / contextBeans / eventListener。

use crate::flow::flow_event_listener::FlowEventListener;
use std::any::Any;
use std::sync::Arc;

/// 会话 ID 生成（对应 ConversationIdGenerator 的 NanoId 语义：短随机串）
pub fn gen_conversation_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let r: u32 = rand_u32();
    format!("{ts:x}{r:08x}")
}

fn rand_u32() -> u32 {
    // 轻量随机：时间 + 栈地址扰动（避免引入 rand 依赖；语义等价于 NanoId 的唯一性保证）
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x9e3779b97f4a7c15);
    let mut x = SEED.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed) ^ {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        t
    };
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x as u32
}

#[derive(Default, Clone)]
pub struct ExecuteOption {
    /// 指定本次执行的 requestId（None 时框架自动生成）
    pub request_id: Option<String>,
    /// 业务会话标识（ReAct Agent 连续对话场景）
    pub conversation_id: Option<String>,
    /// 声明需要 conversationId 但由框架生成
    pub auto_conversation_id: bool,
    /// contextBean 列表
    pub context_beans: Vec<(String, Arc<dyn Any + Send + Sync>)>,
    /// 本次执行的事件监听器
    pub event_listener: Option<Arc<dyn FlowEventListener>>,
}

impl ExecuteOption {
    pub fn of() -> Self {
        Self::default()
    }
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    /// 显式设置 conversationId（取消 auto_conversation_id 语义）
    pub fn conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self.auto_conversation_id = false;
        self
    }
    pub fn auto_conversation_id(mut self) -> Self {
        self.auto_conversation_id = true;
        self
    }
    pub fn context_bean(mut self, name: impl Into<String>, bean: Arc<dyn Any + Send + Sync>) -> Self {
        self.context_beans.push((name.into(), bean));
        self
    }
    pub fn event_listener(mut self, listener: Arc<dyn FlowEventListener>) -> Self {
        self.event_listener = Some(listener);
        self
    }
    /// 解析最终 conversationId（对应 FlowExecutor 内 resolve 逻辑）
    pub(crate) fn resolve_conversation_id(&self) -> Option<String> {
        if let Some(cid) = &self.conversation_id {
            Some(cid.clone())
        } else if self.auto_conversation_id {
            Some(gen_conversation_id())
        } else {
            None
        }
    }
}
