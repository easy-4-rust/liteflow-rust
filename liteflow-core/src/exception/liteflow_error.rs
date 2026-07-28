//! LiteFlow 运行时的统一 Rust 错误枚举。
//!
//! 该文件是 Rust `Result` 传播所需的实现对象；Java 基类
//! `LiteFlowException` 位于同包的 `lite_flow_exception.rs`。

use thiserror::Error;

use super::lite_flow_exception::LiteFlowException;

#[derive(Error, Debug, Clone)]
pub enum LiteflowError {
    /// Java LiteFlowException 基类实例，保留业务错误码、消息与 cause。
    #[error(transparent)]
    LiteFlow(#[from] LiteFlowException),
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
    NodeExec {
        node: String,
        msg: String,
        /// 被节点边界包装前的 LiteflowError 变体名，用于 RETRY 异常过滤。
        kind: String,
        /// 被节点边界包装前的 Java LiteFlowException 业务错误码。
        code: Option<String>,
    },
    /// ChainEndException（正常终止，不算失败）
    #[error("{0}")]
    ChainEnd(String),
    /// WhenExecuteException
    #[error("when execute error: {0}")]
    WhenExecute(String),
    /// WhenTimeoutException
    #[error("{0}")]
    WhenTimeout(String),
    /// NoSwitchTargetNodeException
    #[error("no switch target node found, target str is [{0}]")]
    NoSwitchTarget(String),
    /// NoIfTrueNodeException
    #[error("no if-true node found for the component[{0}]")]
    NoIfTrueNode(String),
    /// NoForNodeException
    #[error("{0}")]
    NoForNode(String),
    /// NoWhileNodeException
    #[error("{0}")]
    NoWhileNode(String),
    /// NoIteratorNodeException
    #[error("{0}")]
    NoIteratorNode(String),
    /// IfTargetCannotBePreOrFinallyException / SwitchTargetCannotBePreOrFinallyException
    #[error("target node cannot be pre or finally: {0}")]
    TargetCannotBePreOrFinally(String),
    /// IfTypeErrorException / SwitchTypeErrorException
    #[error("node[{node}] should return {expect}, but got {actual}")]
    NodeTypeError {
        node: String,
        expect: String,
        actual: String,
    },
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
    #[error("{0}")]
    NoMatchedRouteChain(String),
    /// ScriptLoadException / 脚本执行错误
    #[error("script error in node[{node}]: {msg}")]
    Script { node: String, msg: String },
    /// NodeIdUnIllegalException（2.16：node id 必须符合变量命名规则，
    /// 不能以数字开头，只能由字母/数字/下划线/$ 组成）
    #[error(
        "invalid node id: [{0}]. node id must follow variable naming rules: cannot start with a digit, must consist of letters, digits, underscores (_), or dollar signs ($)"
    )]
    NodeIdUnIllegal(String),
    /// AndOrConditionException
    #[error("{0}")]
    AndOrCondition(String),
    /// CatchErrorException
    #[error("{0}")]
    CatchError(String),
    /// ChainDuplicateException
    #[error("{0}")]
    ChainDuplicate(String),
    /// ChainNotImplementedException
    #[error("{0}")]
    ChainNotImplemented(String),
    /// CmpDefinitionException
    #[error("{0}")]
    CmpDefinition(String),
    /// ComponentCannotRegisterException
    #[error("{0}")]
    ComponentCannotRegister(String),
    /// ComponentNotAccessException
    #[error("{0}")]
    ComponentNotAccess(String),
    /// ComponentProxyErrorException
    #[error("{0}")]
    ComponentProxyError(String),
    /// CyclicDependencyException
    #[error("{0}")]
    CyclicDependency(String),
    /// DataNotFoundException
    #[error("{0}")]
    DataNotFound(String),
    /// EmptyConditionValueException
    #[error("{0}")]
    EmptyConditionValue(String),
    /// ErrorSupportPathException
    #[error("{0}")]
    ErrorSupportPath(String),
    /// FallbackCmpNotFoundException
    #[error("{0}")]
    FallbackCmpNotFound(String),
    /// FlowExecutorNotInitException
    #[error("{0}")]
    FlowExecutorNotInit(String),
    /// MissMavenDependencyException
    #[error("{0}")]
    MissMavenDependency(String),
    /// MonitorFileInitErrorException
    #[error("{0}")]
    MonitorFileInitError(String),
    /// MultipleParsersException
    #[error("{0}")]
    MultipleParsers(String),
    /// NoAvailableSlotException
    #[error("{0}")]
    NoAvailableSlot(String),
    /// NoSuchContextBeanException
    #[error("{0}")]
    NoSuchContextBean(String),
    /// NodeClassNotFoundException
    #[error("{0}")]
    NodeClassNotFound(String),
    /// NodeTypeCanNotGuessException
    #[error("{0}")]
    NodeTypeCanNotGuess(String),
    /// NodeTypeNotSupportException
    #[error("{0}")]
    NodeTypeNotSupport(String),
    /// NotSupportConditionException
    #[error("{0}")]
    NotSupportCondition(String),
    /// NotSupportDeclException
    #[error("{0}")]
    NotSupportDecl(String),
    /// NullNodeTypeException
    #[error("{0}")]
    NullNodeType(String),
    /// NullParamException
    #[error("{0}")]
    NullParam(String),
    /// ObjectConvertException
    #[error("{0}")]
    ObjectConvert(String),
    /// ParallelExecutorCreateException
    #[error("{0}")]
    ParallelExecutorCreate(String),
    /// ParameterFactException
    #[error("{0}")]
    ParameterFact(String),
    /// ParserCannotFindException
    #[error("{0}")]
    ParserCannotFind(String),
    /// ProxyException
    #[error("{0}")]
    Proxy(String),
    /// RequestIdGeneratorException
    #[error("{0}")]
    RequestIdGenerator(String),
    /// RouteELInvalidException
    #[error("{0}")]
    RouteELInvalid(String),
    /// ThreadExecutorServiceCreateException
    #[error("{0}")]
    ThreadExecutorServiceCreate(String),
    /// FlowSystemException
    #[error("{0}")]
    Custom(String),
}

impl LiteflowError {
    /// 返回 Java `LiteFlowException` 基类携带的业务错误码。
    ///
    /// 具体无状态码错误返回 `None`。对应 Java:
    /// `LiteFlowException#getCode`。
    #[must_use]
    pub fn get_code(&self) -> Option<&str> {
        match self {
            Self::LiteFlow(error) => error.get_code(),
            Self::NodeExec { code, .. } => code.as_deref(),
            _ => None,
        }
    }
}
