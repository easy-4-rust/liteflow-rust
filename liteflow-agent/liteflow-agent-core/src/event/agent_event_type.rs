//! 对应 Java `ReActAgentComponent` 的 FlowEvent 类型常量。

/// Agent 对外发布的稳定事件类型。
pub struct AgentEventType;

impl AgentEventType {
    /// 开始 reasoning/acting。
    pub const REASONING: &'static str = "agent.reasoning";
    /// 工具结果。
    pub const TOOL_RESULT: &'static str = "agent.tool_result";
    /// 最大轮次后的摘要。
    pub const SUMMARY: &'static str = "agent.summary";
    /// 最终结果。
    pub const RESULT: &'static str = "agent.result";
}
