//! Java 异常对象可变 message 语义测试。

use liteflow_core::exception::{
    AndOrConditionException, CatchErrorException, ChainDuplicateException, ChainEndException,
    ChainNotFoundException, ChainNotImplementedException, CmpDefinitionException,
    ComponentCannotRegisterException, ComponentMethodDefineErrorException,
    ComponentNotAccessException, ComponentProxyErrorException, ConfigErrorException,
    CyclicDependencyException, DataNotFoundException, ELParseException,
    EmptyConditionValueException, ErrorSupportPathException, ExecutableItemNotFoundException,
    FallbackCmpNotFoundException, FlowExecutorNotInitException, FlowSystemException,
    IfTargetCannotBePreOrFinallyException, IfTypeErrorException, JsonProcessException,
    LiteflowError, MissMavenDependencyException, MonitorFileInitErrorException,
    MultipleParsersException, NoAvailableSlotException, NoForNodeException, NoIfTrueNodeException,
    NoIteratorNodeException, NoMatchedRouteChainException, NoSuchContextBeanException,
    NoSwitchTargetNodeException, NoWhileNodeException, NodeBuildException,
    NodeClassNotFoundException, NodeIdUnIllegalException, NodeTypeCanNotGuessException,
    NodeTypeNotSupportException, NotSupportConditionException, NotSupportDeclException,
    NullNodeTypeException, NullParamException, ObjectConvertException,
    ParallelExecutorCreateException, ParameterFactException, ParseException,
    ParserCannotFindException, ProxyException, RequestIdGeneratorException,
    RouteChainNotFoundException, RouteELInvalidException, ScriptBeanMethodInvokeException,
    SwitchTargetCannotBePreOrFinallyException, SwitchTypeErrorException,
    ThreadExecutorServiceCreateException, WhenExecuteException, WhenTimeoutException,
};
use liteflow_core::script::exception::{ScriptLoadException, ScriptSpiException};

macro_rules! assert_message_round_trip {
    ($exception_type:ty) => {{
        let mut exception = <$exception_type>::new("original-message");
        assert_eq!(exception.get_message(), "original-message");
        assert_eq!(exception.to_string(), "original-message");

        // setter、Display 与统一错误转换必须读取同一字段，避免出现兼容空壳。
        exception.set_message("updated-message");
        assert_eq!(exception.get_message(), "updated-message");
        assert_eq!(exception.to_string(), "updated-message");
        let unified_error: LiteflowError = exception.into();
        assert!(
            unified_error.to_string().contains("updated-message"),
            "统一错误转换必须保留修改后的消息，实际为: {unified_error}"
        );
    }};
}

#[test]
fn core_exception_messages_share_one_real_mutable_state_with_display_and_conversion() {
    assert_message_round_trip!(AndOrConditionException);
    assert_message_round_trip!(CatchErrorException);
    assert_message_round_trip!(ChainDuplicateException);
    assert_message_round_trip!(ChainEndException);
    assert_message_round_trip!(ChainNotFoundException);
    assert_message_round_trip!(ChainNotImplementedException);
    assert_message_round_trip!(CmpDefinitionException);
    assert_message_round_trip!(ComponentCannotRegisterException);
    assert_message_round_trip!(ComponentMethodDefineErrorException);
    assert_message_round_trip!(ComponentNotAccessException);
    assert_message_round_trip!(ComponentProxyErrorException);
    assert_message_round_trip!(ConfigErrorException);
    assert_message_round_trip!(CyclicDependencyException);
    assert_message_round_trip!(DataNotFoundException);
    assert_message_round_trip!(ELParseException);
    assert_message_round_trip!(EmptyConditionValueException);
    assert_message_round_trip!(ErrorSupportPathException);
    assert_message_round_trip!(ExecutableItemNotFoundException);
    assert_message_round_trip!(FallbackCmpNotFoundException);
    assert_message_round_trip!(FlowExecutorNotInitException);
    assert_message_round_trip!(FlowSystemException);
    assert_message_round_trip!(IfTargetCannotBePreOrFinallyException);
    assert_message_round_trip!(IfTypeErrorException);
    assert_message_round_trip!(JsonProcessException);
    assert_message_round_trip!(MonitorFileInitErrorException);
    assert_message_round_trip!(MultipleParsersException);
    assert_message_round_trip!(NoAvailableSlotException);
    assert_message_round_trip!(NodeBuildException);
    assert_message_round_trip!(NodeClassNotFoundException);
    assert_message_round_trip!(NodeIdUnIllegalException);
    assert_message_round_trip!(NodeTypeCanNotGuessException);
    assert_message_round_trip!(NodeTypeNotSupportException);
    assert_message_round_trip!(NoForNodeException);
    assert_message_round_trip!(NoIfTrueNodeException);
    assert_message_round_trip!(NoIteratorNodeException);
    assert_message_round_trip!(NoMatchedRouteChainException);
    assert_message_round_trip!(NoSuchContextBeanException);
    assert_message_round_trip!(NoSwitchTargetNodeException);
    assert_message_round_trip!(NotSupportConditionException);
    assert_message_round_trip!(NotSupportDeclException);
    assert_message_round_trip!(NoWhileNodeException);
    assert_message_round_trip!(NullNodeTypeException);
    assert_message_round_trip!(NullParamException);
    assert_message_round_trip!(ObjectConvertException);
    assert_message_round_trip!(ParallelExecutorCreateException);
    assert_message_round_trip!(ParameterFactException);
    assert_message_round_trip!(ParseException);
    assert_message_round_trip!(ParserCannotFindException);
    assert_message_round_trip!(ProxyException);
    assert_message_round_trip!(RequestIdGeneratorException);
    assert_message_round_trip!(RouteChainNotFoundException);
    assert_message_round_trip!(RouteELInvalidException);
    assert_message_round_trip!(ScriptBeanMethodInvokeException);
    assert_message_round_trip!(SwitchTargetCannotBePreOrFinallyException);
    assert_message_round_trip!(SwitchTypeErrorException);
    assert_message_round_trip!(ThreadExecutorServiceCreateException);
    assert_message_round_trip!(WhenExecuteException);
    assert_message_round_trip!(WhenTimeoutException);
    assert_message_round_trip!(ScriptLoadException);
    assert_message_round_trip!(ScriptSpiException);
}

#[test]
fn data_not_found_default_constructor_preserves_java_msg_constant() {
    let exception = DataNotFoundException::default();

    assert_eq!(
        exception.get_message(),
        liteflow_core::exception::data_not_found_exception::MSG
    );
    assert_eq!(exception.to_string(), "DataNotFoundException");
}

#[test]
fn executable_item_not_found_default_constructor_preserves_java_empty_message() {
    let exception = ExecutableItemNotFoundException::default();

    assert_eq!(exception.get_message(), "");
    assert_eq!(exception.to_string(), "");
}

#[test]
fn miss_maven_dependency_preserves_java_constructor_and_actual_hutool_output() {
    let mut exception = MissMavenDependencyException::new("com.example", "demo");

    // Java 的 StrUtil.format(Object...) 不识别命名占位符，实际消息会原样保留模板。
    assert_eq!(
        exception.get_message(),
        liteflow_core::exception::miss_maven_dependency_exception::TEMPLATE
    );
    assert!(exception.get_message().contains("{groupId}"));
    assert!(exception.get_message().contains("{artifactId}"));
    assert!(exception.get_message().contains("${version}"));

    exception.set_message("updated-message");
    assert_eq!(exception.get_message(), "updated-message");
    let unified_error: LiteflowError = exception.into();
    assert_eq!(unified_error.to_string(), "updated-message");
}
