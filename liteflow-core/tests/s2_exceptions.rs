//! S2-C 挂接测试：exception 包中此前缺少 LiteflowError 变体的异常 struct
//! 均可通过 From 转换为 LiteflowError，并保留各自 Java 构造与消息语义。

use liteflow_core::LiteflowError;
use liteflow_core::exception::and_or_condition_exception::AndOrConditionException;
use liteflow_core::exception::catch_error_exception::CatchErrorException;
use liteflow_core::exception::chain_duplicate_exception::ChainDuplicateException;
use liteflow_core::exception::chain_not_found_exception::ChainNotFoundException;
use liteflow_core::exception::chain_not_implemented_exception::ChainNotImplementedException;
use liteflow_core::exception::cmp_definition_exception::CmpDefinitionException;
use liteflow_core::exception::component_cannot_register_exception::ComponentCannotRegisterException;
use liteflow_core::exception::component_not_access_exception::ComponentNotAccessException;
use liteflow_core::exception::component_proxy_error_exception::ComponentProxyErrorException;
use liteflow_core::exception::cyclic_dependency_exception::CyclicDependencyException;
use liteflow_core::exception::data_not_found_exception::DataNotFoundException;
use liteflow_core::exception::empty_condition_value_exception::EmptyConditionValueException;
use liteflow_core::exception::error_support_path_exception::ErrorSupportPathException;
use liteflow_core::exception::fallback_cmp_not_found_exception::FallbackCmpNotFoundException;
use liteflow_core::exception::flow_executor_not_init_exception::FlowExecutorNotInitException;
use liteflow_core::exception::flow_system_exception::FlowSystemException;
use liteflow_core::exception::miss_maven_dependency_exception::MissMavenDependencyException;
use liteflow_core::exception::monitor_file_init_error_exception::MonitorFileInitErrorException;
use liteflow_core::exception::multiple_parsers_exception::MultipleParsersException;
use liteflow_core::exception::no_available_slot_exception::NoAvailableSlotException;
use liteflow_core::exception::no_such_context_bean_exception::NoSuchContextBeanException;
use liteflow_core::exception::node_class_not_found_exception::NodeClassNotFoundException;
use liteflow_core::exception::node_id_un_illegal_exception::NodeIdUnIllegalException;
use liteflow_core::exception::node_type_can_not_guess_exception::NodeTypeCanNotGuessException;
use liteflow_core::exception::node_type_not_support_exception::NodeTypeNotSupportException;
use liteflow_core::exception::not_support_condition_exception::NotSupportConditionException;
use liteflow_core::exception::not_support_decl_exception::NotSupportDeclException;
use liteflow_core::exception::null_node_type_exception::NullNodeTypeException;
use liteflow_core::exception::null_param_exception::NullParamException;
use liteflow_core::exception::object_convert_exception::ObjectConvertException;
use liteflow_core::exception::parallel_executor_create_exception::ParallelExecutorCreateException;
use liteflow_core::exception::parameter_fact_exception::ParameterFactException;
use liteflow_core::exception::parser_cannot_find_exception::ParserCannotFindException;
use liteflow_core::exception::proxy_exception::ProxyException;
use liteflow_core::exception::request_id_generator_exception::RequestIdGeneratorException;
use liteflow_core::exception::route_el_invalid_exception::RouteELInvalidException;
use liteflow_core::exception::thread_executor_service_create_exception::ThreadExecutorServiceCreateException;

/// 断言 message-only 异常 struct 能 From 转换为 LiteflowError，
/// 且转换后 Display 非空、与 struct 自身 Display（即原始 message）一致。
macro_rules! assert_from {
    ($ty:ty) => {{
        let msg = concat!("probe message for ", stringify!($ty));
        let e = <$ty>::new(msg);
        let err = LiteflowError::from(e);
        let display = err.to_string();
        assert!(!display.is_empty(), "Display must be non-empty");
        assert_eq!(display, msg, "Display must carry the original message");
    }};
}

#[test]
fn s2_newly_wired_exceptions_convert_to_liteflow_error() {
    assert_from!(AndOrConditionException);
    assert_from!(CatchErrorException);
    assert_from!(ChainDuplicateException);
    assert_from!(ChainNotImplementedException);
    assert_from!(CmpDefinitionException);
    assert_from!(ComponentCannotRegisterException);
    assert_from!(ComponentNotAccessException);
    assert_from!(ComponentProxyErrorException);
    assert_from!(CyclicDependencyException);
    assert_from!(DataNotFoundException);
    assert_from!(EmptyConditionValueException);
    assert_from!(ErrorSupportPathException);
    assert_from!(FallbackCmpNotFoundException);
    assert_from!(FlowExecutorNotInitException);
    assert_from!(MonitorFileInitErrorException);
    assert_from!(MultipleParsersException);
    assert_from!(NoAvailableSlotException);
    assert_from!(NoSuchContextBeanException);
    assert_from!(NodeClassNotFoundException);
    assert_from!(NodeTypeCanNotGuessException);
    assert_from!(NodeTypeNotSupportException);
    assert_from!(NotSupportConditionException);
    assert_from!(NotSupportDeclException);
    assert_from!(NullNodeTypeException);
    assert_from!(NullParamException);
    assert_from!(ObjectConvertException);
    assert_from!(ParallelExecutorCreateException);
    assert_from!(ParameterFactException);
    assert_from!(ParserCannotFindException);
    assert_from!(ProxyException);
    assert_from!(RequestIdGeneratorException);
    assert_from!(RouteELInvalidException);
    assert_from!(ThreadExecutorServiceCreateException);

    // 该对象在 Java 中不是 message-only 构造器，而是接收 Maven 坐标后生成提示。
    let exception = MissMavenDependencyException::new("com.example", "demo");
    let expected_message = exception.to_string();
    let error = LiteflowError::from(exception);
    assert_eq!(error.to_string(), expected_message);
}

/// 回归抽查：已有 From 实现的 3 个异常 struct 转换行为不受新增变体影响。
#[test]
fn existing_from_impls_regression() {
    let err = LiteflowError::from(ChainNotFoundException::new("chain-x"));
    assert!(matches!(err, LiteflowError::ChainNotFound(_)));
    assert_eq!(err.to_string(), "chain not found: chain-x");

    let err = LiteflowError::from(NodeIdUnIllegalException::new("1bad"));
    assert!(matches!(err, LiteflowError::NodeIdUnIllegal(_)));
    assert!(err.to_string().contains("1bad"));

    let err = LiteflowError::from(FlowSystemException::new("boom"));
    assert!(matches!(err, LiteflowError::Custom(_)));
    assert_eq!(err.to_string(), "boom");
}
