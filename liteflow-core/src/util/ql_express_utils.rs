//! LiteFlow EL 的 QLExpress4 运行器适配。
//!
//! 对应 Java: `com.yomahub.liteflow.util.QlExpressUtils`。

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::InterpolationMode;
use qlexpress::{
    DataValue, Express4Runner, InitOptions, NativeObject, QLException, QLOptions,
    QLSecurityStrategy,
};

use crate::builder::el::operator::base::BaseOperator;
use crate::builder::el::operator::{
    AndOperator, CatchOperator, FinallyOperator, ForOperator, IfOperator, IteratorOperator,
    NodeOperator, NotOperator, OrOperator, PreOperator, SwitchOperator, ThenOperator, WhenOperator,
    WhileOperator,
};
use crate::el::{Arg, El, NodeRef, apply_el_method, apply_el_method_ref};
use crate::exception::{LFResult, LiteflowError};

/// LiteFlow EL 解析与变量名校验工具。
///
/// Java 使用两个静态 `Express4Runner`：EL Runner 禁用字符串插值，上下文搜索
/// Runner 开放成员访问。Rust 的 QLExpress Runner 内含 `Rc/RefCell`，因此每次
/// 调用在当前线程创建局部 Runner；真正的 lexer、parser、compiler 和 QVM 均由
/// crates.io 发布的 `qlexpress` crate 提供。
///
/// 对应 Java: `com.yomahub.liteflow.util.QlExpressUtils`。
pub struct QlExpressUtils;

impl QlExpressUtils {
    /// 获取注册了全部 LiteFlow EL 操作符的 QLExpress Runner 适配器。
    ///
    /// 返回值可解析并执行一条 LiteFlow EL；每次执行创建线程局部的真实
    /// `Express4Runner`。对应 Java: `QlExpressUtils#getELExpressRunner`。
    #[must_use]
    pub fn get_el_express_runner() -> ElExpressRunner {
        ElExpressRunner
    }

    /// 获取用于上下文表达式搜索的开放安全策略 Runner 适配器。
    ///
    /// 返回值直接执行 QLExpress 表达式并返回 `DataValue`。对应 Java:
    /// `QlExpressUtils#getContextSearchExpressRunner`。
    #[must_use]
    pub fn get_context_search_express_runner() -> ContextSearchExpressRunner {
        ContextSearchExpressRunner
    }

    /// 使用真实 QLExpress lexer/parser/compiler/QVM 解析 LiteFlow EL。
    ///
    /// 参数 `expression` 对应 Java 传给 `Express4Runner#execute` 的 EL 文本；
    /// 返回现有强类型 `El` AST，供 `LiteFlowChainELBuilder` 构建 Condition。
    pub fn parse_el(expression: &str) -> LFResult<El> {
        Self::get_el_express_runner().execute(expression)
    }

    /// 检查变量名是否符合 Java 标识符语义。
    ///
    /// 参数 `variable_name` 为待检查名称；首字符必须是字母、下划线或美元符号，
    /// 后续字符还可包含数字。对应 Java: `QlExpressUtils#checkVariableName`。
    #[must_use]
    pub fn check_variable_name(variable_name: &str) -> bool {
        let mut characters = variable_name.chars();
        matches!(
            characters.next(),
            Some(character) if character.is_alphabetic() || character == '_' || character == '$'
        ) && characters
            .all(|character| character.is_alphanumeric() || character == '_' || character == '$')
    }
}

/// LiteFlow EL 专用的 QLExpress Runner 适配器。
///
/// Java 返回共享 `Express4Runner`；Rust 返回无状态句柄，在调用线程内创建 Runner，
/// 避免 QLExpress 的 `Rc/RefCell` 跨线程逃逸。
#[derive(Debug, Clone, Copy, Default)]
pub struct ElExpressRunner;

impl ElExpressRunner {
    /// 编译并执行 LiteFlow EL，返回强类型表达式树。
    ///
    /// 参数 `expression` 是完整 EL；QLExpress 语法错误、未知扩展或 LiteFlow
    /// Operator 校验错误均转换为 `LiteflowError::Parse`。
    pub fn execute(&self, expression: &str) -> LFResult<El> {
        if expression.trim().is_empty() {
            return Err(LiteflowError::Parse("empty EL".to_string()));
        }

        let runner = create_el_runner();
        let variable_names = runner
            .get_out_var_names(expression)
            .map_err(|error| qlexpress_parse_error(expression, &error.to_string()))?;
        let context = variable_names
            .into_iter()
            .map(|name| {
                let value = ElExpressValue::success(El::Node(NodeRef::new(name.clone())));
                (name, value.into_data_value())
            })
            .collect::<HashMap<_, _>>();

        // QLExpress 负责真实词法、语法、编译与 QVM 执行；注册函数只把动态值
        // 转换到已有的一对象一文件 Operator，实现层不复制业务规则。
        let result = runner
            .execute(expression, context, &QLOptions::default())
            .map_err(|error| qlexpress_parse_error(expression, &error.to_string()))?
            .into_result();
        extract_el_result(result)
    }
}

/// 上下文搜索专用的 QLExpress Runner 适配器。
///
/// 对应 Java `CONTEXT_SEARCH_EXPRESS_RUNNER`；使用开放安全策略执行宿主明确提供的
/// 上下文值，不持有进程级可变状态。
#[derive(Debug, Clone, Copy, Default)]
pub struct ContextSearchExpressRunner;

impl ContextSearchExpressRunner {
    /// 执行上下文搜索表达式。
    ///
    /// 参数 `expression` 为 QLExpress 表达式，`context` 为具名上下文值；返回
    /// QLExpress 原生结果。对应 Java:
    /// `QlExpressUtils#getContextSearchExpressRunner().execute(...)`。
    pub fn execute(
        &self,
        expression: &str,
        context: HashMap<String, DataValue>,
    ) -> LFResult<DataValue> {
        let options = InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build();
        Express4Runner::with_init_options(options)
            .execute(expression, context, &QLOptions::default())
            .map(|result| result.into_result())
            .map_err(|error| qlexpress_parse_error(expression, &error.to_string()))
    }
}

/// QLExpress QVM 中承载 LiteFlow `El` 的宿主对象。
///
/// Java Operator 直接返回 Condition/Node；Rust 将强类型 AST 包装为 NativeObject，
/// 使 `.tag(...)`、`.ELSE(...)` 等成员调用继续由 QVM 调度。
#[derive(Debug, Clone)]
struct ElExpressValue {
    result: LFResult<El>,
}

impl ElExpressValue {
    fn success(expression: El) -> Self {
        Self {
            result: Ok(expression),
        }
    }

    fn from_result(result: LFResult<El>) -> Self {
        Self { result }
    }

    fn into_data_value(self) -> DataValue {
        DataValue::Object(Rc::new(RefCell::new(self)))
    }
}

impl NativeObject for ElExpressValue {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        let result = self
            .result
            .clone()
            .and_then(|expression| apply_el_method_ref(expression, name));
        Some(Self::from_result(result).into_data_value())
    }

    fn call_method(
        &mut self,
        name: &str,
        arguments: &[DataValue],
    ) -> Result<DataValue, QLException> {
        let result = self.result.clone().and_then(|expression| {
            data_values_to_args(arguments)
                .map_err(LiteflowError::Parse)
                .and_then(|arguments| apply_el_method(expression, name, arguments))
        });
        Ok(Self::from_result(result).into_data_value())
    }

    fn native_type_name(&self) -> &str {
        "com.yomahub.liteflow.flow.element.Executable"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_el_runner() -> Express4Runner {
    let options = InitOptions::builder()
        .interpolation_mode(InterpolationMode::Disable)
        // Java 通过 addExtendFunction 放行指定成员；Rust 的上下文只包含
        // ElExpressValue，开放策略不会暴露任意业务对象。
        .security_strategy(QLSecurityStrategy::open())
        .build();
    let runner = Express4Runner::with_init_options(options);

    register_primary(&runner, "THEN", ThenOperator);
    register_primary(&runner, "SER", ThenOperator);
    register_primary(&runner, "WHEN", WhenOperator);
    register_primary(&runner, "PAR", WhenOperator);
    register_primary(&runner, "SWITCH", SwitchOperator);
    register_primary(&runner, "PRE", PreOperator);
    register_primary(&runner, "FINALLY", FinallyOperator);
    register_primary(&runner, "IF", IfOperator);
    register_primary(&runner, "NODE", NodeOperator);
    register_primary(&runner, "node", NodeOperator);
    register_primary(&runner, "FOR", ForOperator);
    register_primary(&runner, "WHILE", WhileOperator);
    register_primary(&runner, "ITERATOR", IteratorOperator);
    register_primary(&runner, "CATCH", CatchOperator);
    register_primary(&runner, "AND", AndOperator);
    register_primary(&runner, "OR", OrOperator);
    register_primary(&runner, "NOT", NotOperator);
    runner
}

fn register_primary<O>(runner: &Express4Runner, name: &'static str, operator: O)
where
    O: BaseOperator + 'static,
{
    let registered = runner.add_varargs_function(name, move |values: &[DataValue]| {
        let result = data_values_to_args(values)
            .map_err(LiteflowError::Parse)
            .and_then(|arguments| operator.build(None, arguments));
        Ok(ElExpressValue::from_result(result).into_data_value())
    });
    debug_assert!(registered, "duplicate LiteFlow QLExpress function: {name}");
}

fn data_values_to_args(values: &[DataValue]) -> Result<Vec<Arg>, String> {
    values.iter().map(data_value_to_arg).collect()
}

fn data_value_to_arg(value: &DataValue) -> Result<Arg, String> {
    match value {
        DataValue::Null => Ok(Arg::Null),
        DataValue::Bool(value) => Ok(Arg::Bool(*value)),
        DataValue::Byte(value) => Ok(Arg::Num(f64::from(*value))),
        DataValue::Short(value) => Ok(Arg::Num(f64::from(*value))),
        DataValue::Int(value) => Ok(Arg::Num(f64::from(*value))),
        DataValue::Long(value) => Ok(Arg::Num(*value as f64)),
        DataValue::Float(value) => Ok(Arg::Num(f64::from(*value))),
        DataValue::Double(value) => Ok(Arg::Num(*value)),
        DataValue::BigInt(value) => value
            .to_string()
            .parse::<f64>()
            .map(Arg::Num)
            .map_err(|error| format!("cannot convert QLExpress BigInteger: {error}")),
        DataValue::BigDec(value) => value
            .parse::<f64>()
            .map(Arg::Num)
            .map_err(|error| format!("cannot convert QLExpress BigDecimal: {error}")),
        DataValue::Char(value) => Ok(Arg::Str(value.to_string())),
        DataValue::Str(value) => Ok(Arg::Str(value.clone())),
        DataValue::Object(value) => {
            let value = value.borrow();
            let Some(value) = value.as_any().downcast_ref::<ElExpressValue>() else {
                return Err(format!(
                    "unsupported QLExpress native EL argument: {}",
                    value.native_type_name()
                ));
            };
            value
                .result
                .clone()
                .map(Arg::Expr)
                .map_err(liteflow_error_detail)
        }
        other => Err(format!(
            "unsupported QLExpress EL argument type: {}",
            other.data_type_name()
        )),
    }
}

fn extract_el_result(result: DataValue) -> LFResult<El> {
    let DataValue::Object(value) = result else {
        return Err(LiteflowError::Parse(format!(
            "QLExpress EL must return Executable, got {}",
            result.data_type_name()
        )));
    };
    let value = value.borrow();
    let Some(value) = value.as_any().downcast_ref::<ElExpressValue>() else {
        return Err(LiteflowError::Parse(format!(
            "QLExpress EL returned unsupported native object: {}",
            value.native_type_name()
        )));
    };
    value.result.clone()
}

fn qlexpress_parse_error(expression: &str, detail: &str) -> LiteflowError {
    LiteflowError::Parse(format!("{detail}\n EL: {expression}"))
}

fn liteflow_error_detail(error: LiteflowError) -> String {
    match error {
        LiteflowError::Parse(message) => message,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use qlexpress::DataValue;

    use super::QlExpressUtils;
    use crate::el::El;

    #[test]
    fn el_runner_executes_with_published_qlexpress_qvm() {
        let expression = QlExpressUtils::get_el_express_runner()
            .execute("THEN(a, WHEN(b, c))")
            .unwrap();
        assert!(matches!(expression, El::Then(items) if items.len() == 2));
    }

    #[test]
    fn context_search_runner_executes_real_qlexpress_expression() {
        let context = HashMap::from([
            ("left".to_string(), DataValue::Int(20)),
            ("right".to_string(), DataValue::Int(22)),
        ]);
        let result = QlExpressUtils::get_context_search_express_runner()
            .execute("left + right", context)
            .unwrap();
        assert_eq!(result, DataValue::Int(42));
    }
}
