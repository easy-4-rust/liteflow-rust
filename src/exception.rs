//! 对应 liteflow-core exception 包。

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LiteflowError {
    /// ELParseException / ParseException
    #[error("EL parse error: {0}")]
    Parse(String),
    /// ChainNotFoundException
    #[error("chain not found: {0}")]
    ChainNotFound(String),
    /// ExecutableItemNotFoundException
    #[error("node not found: {0}")]
    NodeNotFound(String),
    /// 组件执行异常包裹
    #[error("node[{node}] execute error: {msg}")]
    NodeExec { node: String, msg: String },
    /// ChainEndException（正常终止，不算失败）
    #[error("chain end")]
    ChainEnd,
    /// WhenExecuteException
    #[error("when execute error: {0}")]
    WhenExecute(String),
    /// WhenTimeoutException
    #[error("when timeout")]
    WhenTimeout,
    /// NoSwitchTargetNodeException
    #[error("no switch target node found, target str is [{0}]")]
    NoSwitchTarget(String),
    /// NoIfTrueNodeException
    #[error("no if-true node found for the component[{0}]")]
    NoIfTrueNode(String),
    /// NoForNodeException
    #[error("no for-node found")]
    NoForNode,
    /// NoWhileNodeException
    #[error("no while-node found")]
    NoWhileNode,
    /// NoIteratorNodeException
    #[error("no iterator-node found")]
    NoIteratorNode,
    /// IfTargetCannotBePreOrFinallyException / SwitchTargetCannotBePreOrFinallyException
    #[error("target node cannot be pre or finally: {0}")]
    TargetCannotBePreOrFinally(String),
    /// IfTypeErrorException / SwitchTypeErrorException
    #[error("node[{node}] should return {expect}, but got {actual}")]
    NodeTypeError { node: String, expect: String, actual: String },
    /// NodeBuildException（链构建期节点未注册等）
    #[error("node build error: {0}")]
    NodeBuild(String),
    /// ConfigErrorException / JsonProcessException
    #[error("rule error: {0}")]
    Rule(String),
    /// ComponentMethodDefineErrorException
    #[error("component define error: {0}")]
    CmpDefine(String),
    /// RouteChainNotFoundException
    #[error("no route found for namespace[{0}]")]
    RouteChainNotFound(String),
    /// NoMatchedRouteChainException
    #[error("there is no matched route chain")]
    NoMatchedRouteChain,
    /// ScriptLoadException / 脚本执行错误
    #[error("script error in node[{node}]: {msg}")]
    Script { node: String, msg: String },
    /// FlowSystemException
    #[error("{0}")]
    Custom(String),
}

pub type LFResult<T> = Result<T, LiteflowError>;
