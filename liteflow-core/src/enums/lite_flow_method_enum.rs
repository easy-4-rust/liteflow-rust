//! 对应 com.yomahub.liteflow.enums.LiteFlowMethodEnum：
//! 声明式组件（@LiteflowMethod / @LiteflowCmpDefine 语义）可编排方法的元数据。
//! isMainMethod=true 的方法对应 EL 中的主逻辑（processXxx），
//! false 的为辅助方法（isAccess/isContinueOnError/beforeProcess 等）。

/// LiteFlow 可编排方法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteFlowMethodEnum {
    Process,
    ProcessSwitch,
    ProcessIf,
    ProcessFor,
    ProcessWhile,
    ProcessBreak,
    ProcessIterator,
    IsAccess,
    IsEnd,
    IsContinueOnError,
    GetNodeExecutorClass,
    OnSuccess,
    OnError,
    BeforeProcess,
    AfterProcess,
}

impl LiteFlowMethodEnum {
    /// getMethodName()：Java 方法名
    pub fn get_method_name(&self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::ProcessSwitch => "processSwitch",
            Self::ProcessIf => "processIf",
            Self::ProcessFor => "processFor",
            Self::ProcessWhile => "processWhile",
            Self::ProcessBreak => "processBreak",
            Self::ProcessIterator => "processIterator",
            Self::IsAccess => "isAccess",
            Self::IsEnd => "isEnd",
            Self::IsContinueOnError => "isContinueOnError",
            Self::GetNodeExecutorClass => "getNodeExecutorClass",
            Self::OnSuccess => "onSuccess",
            Self::OnError => "onError",
            Self::BeforeProcess => "beforeProcess",
            Self::AfterProcess => "afterProcess",
        }
    }
    /// isMainMethod()：是否为主逻辑方法（processXxx）
    pub fn is_main_method(&self) -> bool {
        matches!(
            self,
            Self::Process
                | Self::ProcessSwitch
                | Self::ProcessIf
                | Self::ProcessFor
                | Self::ProcessWhile
                | Self::ProcessBreak
                | Self::ProcessIterator
        )
    }
    /// 按 Java 方法名反查（对应按 methodName 匹配的声明式方法解析）
    pub fn get_enum_by_method_name(name: &str) -> Option<Self> {
        [
            Self::Process,
            Self::ProcessSwitch,
            Self::ProcessIf,
            Self::ProcessFor,
            Self::ProcessWhile,
            Self::ProcessBreak,
            Self::ProcessIterator,
            Self::IsAccess,
            Self::IsEnd,
            Self::IsContinueOnError,
            Self::GetNodeExecutorClass,
            Self::OnSuccess,
            Self::OnError,
            Self::BeforeProcess,
            Self::AfterProcess,
        ]
        .into_iter()
        .find(|e| e.get_method_name() == name)
    }
}
