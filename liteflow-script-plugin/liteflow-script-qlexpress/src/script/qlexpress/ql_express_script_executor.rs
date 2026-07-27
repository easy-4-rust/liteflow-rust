//! QLExpress 脚本执行器。

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use liteflow_core::common::entity::ValidationResp;
use liteflow_core::enums::ScriptTypeEnum;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::script::proxy::ScriptBeanProxy;
use liteflow_core::script::{
    ScriptBeanManager, ScriptExecutor, ScriptExecutorComponent, ScriptExecutorFactory, ScriptKind,
};
use liteflow_core::slot::CmpContext;
use serde_json::{Number, Value, json};

/// 阿里 QLExpress 脚本语言的 Rust 执行器。
///
/// 本实现使用独立词法器、Pratt 表达式解析器和语句解释器，不再把 QLExpress 文本
/// 伪装成 Rhai。编译阶段缓存强类型语法树；执行阶段绑定 LiteFlow `defaultContext`、
/// `_meta` 和受控 `ScriptBeanManager`。对应 Java:
/// `com.yomahub.liteflow.script.qlexpress.QLExpressScriptExecutor`。
pub struct QlExpressScriptExecutor {
    compiled_script_map: RwLock<HashMap<String, Program>>,
}

impl Default for QlExpressScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl QlExpressScriptExecutor {
    /// 创建空的 QLExpress 执行器。
    ///
    /// 对应 Java: `QLExpressScriptExecutor#init`。
    #[must_use]
    pub fn new() -> Self {
        Self {
            compiled_script_map: RwLock::new(HashMap::new()),
        }
    }

    /// 注册 `qlexpress` 语言组件构建器。
    pub fn register() -> LFResult<()> {
        ScriptExecutorFactory::register("qlexpress", Self::build)
    }

    /// 编译 QLExpress 源代码为强类型语法树。
    ///
    /// `script` 是 Java 侧 `Express4Runner#parseToDefinitionWithCache` 接收的原始文本。
    /// 对应 Java: `QLExpressScriptExecutor#compile`。
    fn compile(&self, script: &str) -> Result<Program, QlExpressError> {
        Parser::new(Lexer::new(script).tokenize()?).parse_program()
    }

    fn build(
        node_id: &str,
        kind: ScriptKind,
        script: &str,
    ) -> LFResult<Arc<dyn liteflow_core::NodeComponent>> {
        let executor = shared_executor();
        executor.load(node_id, script)?;
        let executor: Arc<dyn ScriptExecutor> = executor;
        Ok(Arc::new(ScriptExecutorComponent::new(
            node_id, kind, executor,
        )))
    }
}

impl ScriptExecutor for QlExpressScriptExecutor {
    /// 编译并缓存节点脚本。
    ///
    /// 对应 Java: `QLExpressScriptExecutor#load`。
    fn load(&self, node_id: &str, script: &str) -> LFResult<()> {
        let program = self
            .compile(script)
            .map_err(|error| script_error(node_id, error))?;
        self.compiled_script_map
            .write()
            .map_err(|_| cache_error("write"))?
            .insert(node_id.to_string(), program);
        Ok(())
    }

    /// 卸载节点脚本。
    ///
    /// 对应 Java: `QLExpressScriptExecutor#unLoad`。
    fn unload(&self, node_id: &str) -> LFResult<()> {
        self.compiled_script_map
            .write()
            .map_err(|_| cache_error("write"))?
            .remove(node_id);
        Ok(())
    }

    /// 返回已加载节点 ID。
    ///
    /// 对应 Java: `QLExpressScriptExecutor#getNodeIds`。
    fn node_ids(&self) -> LFResult<Vec<String>> {
        let mut node_ids = self
            .compiled_script_map
            .read()
            .map_err(|_| cache_error("read"))?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        node_ids.sort();
        Ok(node_ids)
    }

    /// 使用 LiteFlow 上下文执行已编译脚本。
    ///
    /// 对应 Java: `QLExpressScriptExecutor#executeScript`。
    fn execute_script(&self, node_id: &str, context: &CmpContext) -> LFResult<Value> {
        let program = self
            .compiled_script_map
            .read()
            .map_err(|_| cache_error("read"))?
            .get(node_id)
            .cloned()
            .ok_or_else(|| LiteflowError::Script {
                node: node_id.to_string(),
                msg: format!("script for node[{node_id}] is not loaded"),
            })?;
        let mut environment = Environment::new(context);
        program
            .execute(&mut environment)
            .map_err(|error| script_error(node_id, error))
    }

    /// 清理全部编译缓存。
    ///
    /// 对应 Java: `QLExpressScriptExecutor#cleanCache`。
    fn clean_cache(&self) -> LFResult<()> {
        self.compiled_script_map
            .write()
            .map_err(|_| cache_error("write"))?
            .clear();
        Ok(())
    }

    fn script_type(&self) -> ScriptTypeEnum {
        ScriptTypeEnum::QlExpress
    }

    /// 使用 QLExpress 解析器校验源代码并保留失败原因。
    ///
    /// 对应 Java: `ScriptExecutor#validate`。
    fn validate_with_ex(&self, script: &str) -> ValidationResp {
        match self.compile(script) {
            Ok(_) => ValidationResp::success(),
            Err(error) => ValidationResp::fail(LiteflowError::Script {
                node: String::new(),
                msg: error.to_string(),
            }),
        }
    }
}

fn shared_executor() -> Arc<QlExpressScriptExecutor> {
    static EXECUTOR: OnceLock<Arc<QlExpressScriptExecutor>> = OnceLock::new();
    Arc::clone(EXECUTOR.get_or_init(|| Arc::new(QlExpressScriptExecutor::new())))
}

fn cache_error(operation: &str) -> LiteflowError {
    LiteflowError::Script {
        node: String::new(),
        msg: format!("qlexpress script cache {operation} lock poisoned"),
    }
}

fn script_error(node_id: &str, error: QlExpressError) -> LiteflowError {
    LiteflowError::Script {
        node: node_id.to_string(),
        msg: error.to_string(),
    }
}

#[derive(Debug, Clone)]
struct Program {
    statements: Vec<Statement>,
}

impl Program {
    fn execute(&self, environment: &mut Environment<'_>) -> Result<Value, QlExpressError> {
        let mut last = Value::Null;
        for statement in &self.statements {
            match statement.execute(environment)? {
                FlowControl::Continue(value) => last = value,
                FlowControl::Return(value) => return Ok(value),
            }
        }
        Ok(last)
    }
}

#[derive(Debug, Clone)]
enum Statement {
    Assignment(String, Expression),
    Expression(Expression),
    Return(Expression),
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
}

impl Statement {
    fn execute(&self, environment: &mut Environment<'_>) -> Result<FlowControl, QlExpressError> {
        match self {
            Self::Assignment(name, expression) => {
                let value = expression.evaluate(environment)?;
                environment.variables.insert(name.clone(), value.clone());
                Ok(FlowControl::Continue(value))
            }
            Self::Expression(expression) => {
                expression.evaluate(environment).map(FlowControl::Continue)
            }
            Self::Return(expression) => expression.evaluate(environment).map(FlowControl::Return),
            Self::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let selected = if as_bool(condition.evaluate(environment)?)? {
                    then_branch
                } else {
                    else_branch
                };
                let mut last = Value::Null;
                for statement in selected {
                    match statement.execute(environment)? {
                        FlowControl::Continue(value) => last = value,
                        returned @ FlowControl::Return(_) => return Ok(returned),
                    }
                }
                Ok(FlowControl::Continue(last))
            }
        }
    }
}

#[derive(Debug, Clone)]
enum FlowControl {
    Continue(Value),
    Return(Value),
}

#[derive(Debug, Clone)]
enum Expression {
    Literal(Value),
    Variable(String),
    Unary {
        operator: TokenKind,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: TokenKind,
        right: Box<Expression>,
    },
    Call {
        target: Vec<String>,
        arguments: Vec<Expression>,
    },
}

impl Expression {
    fn evaluate(&self, environment: &mut Environment<'_>) -> Result<Value, QlExpressError> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Variable(name) => environment.resolve(name),
            Self::Unary { operator, operand } => {
                let value = operand.evaluate(environment)?;
                match operator {
                    TokenKind::Bang => Ok(Value::Bool(!as_bool(value)?)),
                    TokenKind::Minus if value.as_i64().is_some() => {
                        Ok(json!(-value.as_i64().expect("checked integer")))
                    }
                    TokenKind::Minus => number_value(-as_f64(&value)?),
                    _ => Err(QlExpressError::runtime("unsupported unary operator")),
                }
            }
            Self::Binary {
                left,
                operator,
                right,
            } => {
                if *operator == TokenKind::AndAnd {
                    let left = as_bool(left.evaluate(environment)?)?;
                    return if left {
                        Ok(Value::Bool(as_bool(right.evaluate(environment)?)?))
                    } else {
                        Ok(Value::Bool(false))
                    };
                }
                if *operator == TokenKind::OrOr {
                    let left = as_bool(left.evaluate(environment)?)?;
                    return if left {
                        Ok(Value::Bool(true))
                    } else {
                        Ok(Value::Bool(as_bool(right.evaluate(environment)?)?))
                    };
                }
                evaluate_binary(
                    left.evaluate(environment)?,
                    operator,
                    right.evaluate(environment)?,
                )
            }
            Self::Call { target, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| argument.evaluate(environment))
                    .collect::<Result<Vec<_>, _>>()?;
                environment.call(target, &arguments)
            }
        }
    }
}

struct Environment<'a> {
    context: &'a CmpContext,
    variables: HashMap<String, Value>,
}

impl<'a> Environment<'a> {
    fn new(context: &'a CmpContext) -> Self {
        Self {
            context,
            variables: HashMap::new(),
        }
    }

    fn resolve(&self, name: &str) -> Result<Value, QlExpressError> {
        match name {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            "null" => Ok(Value::Null),
            "requestData" => Ok(self.context.request_data().unwrap_or(Value::Null)),
            _ => self.variables.get(name).cloned().ok_or_else(|| {
                QlExpressError::runtime(format!("variable [{name}] is not defined"))
            }),
        }
    }

    fn call(&self, target: &[String], arguments: &[Value]) -> Result<Value, QlExpressError> {
        match target {
            [object, method] if object == "defaultContext" && method == "getData" => {
                let key = string_argument(arguments, 0, target)?;
                Ok(self.context.get_data(key).unwrap_or(Value::Null))
            }
            [object, method] if object == "defaultContext" && method == "hasData" => {
                let key = string_argument(arguments, 0, target)?;
                Ok(Value::Bool(self.context.get_data(key).is_some()))
            }
            [object, method] if object == "defaultContext" && method == "setData" => {
                let key = string_argument(arguments, 0, target)?;
                let value = arguments.get(1).cloned().ok_or_else(|| {
                    QlExpressError::runtime("defaultContext.setData requires two arguments")
                })?;
                self.context.set_data(key, value.clone());
                Ok(value)
            }
            [object, method] if object == "_meta" && method == "get" => {
                let key = string_argument(arguments, 0, target)?;
                Ok(meta_value(self.context, key))
            }
            [system, out, method] if system == "System" && out == "out" && method == "println" => {
                Ok(arguments.first().cloned().unwrap_or(Value::Null))
            }
            [bean, method] => {
                // Java bindParam 会先把本次 Slot 的 context bean 绑定为脚本变量。
                // Rust 无运行期反射，因此调用方以 ScriptBeanProxy 显式描述可访问方法；
                // 执行级代理优先于进程级 ScriptBeanManager，避免并发请求互相污染。
                if let Some(proxy) = self.context.bean::<ScriptBeanProxy>(bean) {
                    return proxy
                        .invoke(method, arguments)
                        .map_err(|error| QlExpressError::runtime(error.to_string()));
                }
                ScriptBeanManager::invoke(bean, method, arguments)
                    .map_err(|error| QlExpressError::runtime(error.to_string()))
            }
            _ => Err(QlExpressError::runtime(format!(
                "unsupported call target [{}]",
                target.join(".")
            ))),
        }
    }
}

fn string_argument<'a>(
    arguments: &'a [Value],
    index: usize,
    target: &[String],
) -> Result<&'a str, QlExpressError> {
    arguments.get(index).and_then(Value::as_str).ok_or_else(|| {
        QlExpressError::runtime(format!(
            "{} argument {} must be a string",
            target.join("."),
            index + 1
        ))
    })
}

fn meta_value(context: &CmpContext, key: &str) -> Value {
    match key {
        "nodeId" => json!(context.node_id()),
        "tag" => context.tag().map_or(Value::Null, |tag| json!(tag)),
        "cmpData" => context
            .cmp_data()
            .map_or(Value::Null, |cmp_data| json!(cmp_data)),
        "loopIndex" => context
            .loop_index()
            .map_or(Value::Null, |loop_index| json!(loop_index)),
        "loopObject" => context.loop_object().unwrap_or(Value::Null),
        "requestData" => context.request_data().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn evaluate_binary(
    left: Value,
    operator: &TokenKind,
    right: Value,
) -> Result<Value, QlExpressError> {
    match operator {
        TokenKind::Plus => {
            if left.is_string() || right.is_string() {
                return Ok(Value::String(format!(
                    "{}{}",
                    display_value(&left),
                    display_value(&right)
                )));
            }
            arithmetic(left, right, |a, b| a.checked_add(b), |a, b| a + b)
        }
        TokenKind::Minus => arithmetic(left, right, |a, b| a.checked_sub(b), |a, b| a - b),
        TokenKind::Star => arithmetic(left, right, |a, b| a.checked_mul(b), |a, b| a * b),
        TokenKind::Slash => {
            if right.as_f64() == Some(0.0) {
                return Err(QlExpressError::runtime("division by zero"));
            }
            arithmetic(left, right, |a, b| a.checked_div(b), |a, b| a / b)
        }
        TokenKind::Percent => {
            if right.as_f64() == Some(0.0) {
                return Err(QlExpressError::runtime("division by zero"));
            }
            arithmetic(left, right, |a, b| a.checked_rem(b), |a, b| a % b)
        }
        TokenKind::EqualEqual => Ok(Value::Bool(values_equal(&left, &right))),
        TokenKind::BangEqual => Ok(Value::Bool(!values_equal(&left, &right))),
        TokenKind::Greater => compare_numbers(left, right, |a, b| a > b),
        TokenKind::GreaterEqual => compare_numbers(left, right, |a, b| a >= b),
        TokenKind::Less => compare_numbers(left, right, |a, b| a < b),
        TokenKind::LessEqual => compare_numbers(left, right, |a, b| a <= b),
        _ => Err(QlExpressError::runtime("unsupported binary operator")),
    }
}

fn arithmetic(
    left: Value,
    right: Value,
    integer_operation: impl FnOnce(i64, i64) -> Option<i64>,
    float_operation: impl FnOnce(f64, f64) -> f64,
) -> Result<Value, QlExpressError> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return integer_operation(left, right)
            .map(|value| Value::Number(Number::from(value)))
            .ok_or_else(|| QlExpressError::runtime("integer arithmetic overflow"));
    }
    number_value(float_operation(as_f64(&left)?, as_f64(&right)?))
}

fn values_equal(left: &Value, right: &Value) -> bool {
    if left.is_number() && right.is_number() {
        return as_f64(left).ok() == as_f64(right).ok();
    }
    left == right
}

fn compare_numbers(
    left: Value,
    right: Value,
    predicate: impl FnOnce(f64, f64) -> bool,
) -> Result<Value, QlExpressError> {
    Ok(Value::Bool(predicate(as_f64(&left)?, as_f64(&right)?)))
}

fn as_bool(value: Value) -> Result<bool, QlExpressError> {
    value
        .as_bool()
        .ok_or_else(|| QlExpressError::runtime(format!("expected boolean, got {value}")))
}

fn as_f64(value: &Value) -> Result<f64, QlExpressError> {
    value
        .as_f64()
        .ok_or_else(|| QlExpressError::runtime(format!("expected number, got {value}")))
}

fn number_value(value: f64) -> Result<Value, QlExpressError> {
    Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| QlExpressError::runtime("numeric result is not finite"))
}

fn display_value(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    lexeme: String,
    offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Identifier,
    Number,
    String,
    If,
    Else,
    Return,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Semicolon,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    EqualEqual,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    AndAnd,
    OrOr,
    Eof,
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, QlExpressError> {
        let mut tokens = Vec::new();
        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                self.advance();
                continue;
            }
            if character == '/' && self.peek_next() == Some('/') {
                self.skip_line_comment();
                continue;
            }
            if character == '/' && self.peek_next() == Some('*') {
                self.skip_block_comment()?;
                continue;
            }
            let start = self.offset;
            let token = if character.is_ascii_alphabetic() || character == '_' {
                self.identifier(start)
            } else if character.is_ascii_digit() {
                self.number(start)
            } else {
                self.symbol_or_string(start)?
            };
            tokens.push(token);
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            offset: self.offset,
        });
        Ok(tokens)
    }

    fn identifier(&mut self, start: usize) -> Token {
        self.advance();
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            self.advance();
        }
        let lexeme = self.source[start..self.offset].to_string();
        let kind = match lexeme.as_str() {
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "return" => TokenKind::Return,
            "and" => TokenKind::AndAnd,
            "or" => TokenKind::OrOr,
            "not" => TokenKind::Bang,
            _ => TokenKind::Identifier,
        };
        Token {
            kind,
            lexeme,
            offset: start,
        }
    }

    fn number(&mut self, start: usize) -> Token {
        self.advance();
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
        }
        if self.peek() == Some('.') {
            self.advance();
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }
        Token {
            kind: TokenKind::Number,
            lexeme: self.source[start..self.offset].to_string(),
            offset: start,
        }
    }

    fn symbol_or_string(&mut self, start: usize) -> Result<Token, QlExpressError> {
        let character = self.advance().expect("peeked character must exist");
        let kind = match character {
            '"' | '\'' => return self.string(start, character),
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            ';' => TokenKind::Semicolon,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '=' if self.consume('=') => TokenKind::EqualEqual,
            '=' => TokenKind::Assign,
            '!' if self.consume('=') => TokenKind::BangEqual,
            '!' => TokenKind::Bang,
            '>' if self.consume('=') => TokenKind::GreaterEqual,
            '>' => TokenKind::Greater,
            '<' if self.consume('=') => TokenKind::LessEqual,
            '<' => TokenKind::Less,
            '&' if self.consume('&') => TokenKind::AndAnd,
            '|' if self.consume('|') => TokenKind::OrOr,
            _ => {
                return Err(QlExpressError::syntax(
                    start,
                    format!("unexpected character [{character}]"),
                ));
            }
        };
        Ok(Token {
            kind,
            lexeme: self.source[start..self.offset].to_string(),
            offset: start,
        })
    }

    fn string(&mut self, start: usize, quote: char) -> Result<Token, QlExpressError> {
        let mut value = String::new();
        loop {
            let character = self
                .advance()
                .ok_or_else(|| QlExpressError::syntax(start, "unterminated string"))?;
            if character == quote {
                break;
            }
            if character == '\\' {
                let escaped = self
                    .advance()
                    .ok_or_else(|| QlExpressError::syntax(start, "unterminated escape"))?;
                value.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    other => other,
                });
            } else {
                value.push(character);
            }
        }
        Ok(Token {
            kind: TokenKind::String,
            lexeme: value,
            offset: start,
        })
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut characters = self.source[self.offset..].chars();
        characters.next()?;
        characters.next()
    }

    fn skip_line_comment(&mut self) {
        while self.peek().is_some_and(|character| character != '\n') {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), QlExpressError> {
        let start = self.offset;
        self.advance();
        self.advance();
        while self.peek().is_some() {
            if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(QlExpressError::syntax(start, "unterminated block comment"))
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn parse_program(mut self) -> Result<Program, QlExpressError> {
        let mut statements = Vec::new();
        while !self.check(TokenKind::Eof) {
            statements.push(self.statement()?);
        }
        Ok(Program { statements })
    }

    fn statement(&mut self) -> Result<Statement, QlExpressError> {
        if self.matches(TokenKind::If) {
            return self.if_statement();
        }
        if self.matches(TokenKind::Return) {
            let expression = self.expression(0)?;
            self.consume_optional_semicolon();
            return Ok(Statement::Return(expression));
        }
        if self.check(TokenKind::Identifier) && self.check_next(TokenKind::Assign) {
            let name = self.advance().lexeme.clone();
            self.expect(TokenKind::Assign, "expected '=' after variable")?;
            let expression = self.expression(0)?;
            self.consume_optional_semicolon();
            return Ok(Statement::Assignment(name, expression));
        }
        let expression = self.expression(0)?;
        self.consume_optional_semicolon();
        Ok(Statement::Expression(expression))
    }

    fn if_statement(&mut self) -> Result<Statement, QlExpressError> {
        self.expect(TokenKind::LeftParen, "expected '(' after if")?;
        let condition = self.expression(0)?;
        self.expect(TokenKind::RightParen, "expected ')' after if condition")?;
        let then_branch = self.block()?;
        let else_branch = if self.matches(TokenKind::Else) {
            if self.matches(TokenKind::If) {
                vec![self.if_statement()?]
            } else {
                self.block()?
            }
        } else {
            Vec::new()
        };
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn block(&mut self) -> Result<Vec<Statement>, QlExpressError> {
        self.expect(TokenKind::LeftBrace, "expected '{'")?;
        let mut statements = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            statements.push(self.statement()?);
        }
        self.expect(TokenKind::RightBrace, "expected '}'")?;
        Ok(statements)
    }

    fn expression(&mut self, minimum_precedence: u8) -> Result<Expression, QlExpressError> {
        let mut left = self.unary()?;
        while let Some(precedence) = binary_precedence(self.peek().kind) {
            if precedence < minimum_precedence {
                break;
            }
            let operator = self.advance().kind;
            let right = self.expression(precedence + 1)?;
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expression, QlExpressError> {
        if self.matches(TokenKind::Bang) || self.matches(TokenKind::Minus) {
            let operator = self.previous().kind;
            return Ok(Expression::Unary {
                operator,
                operand: Box::new(self.unary()?),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expression, QlExpressError> {
        if self.matches(TokenKind::Number) {
            let token = self.previous();
            let value = if token.lexeme.contains('.') {
                let number = token.lexeme.parse::<f64>().map_err(|error| {
                    QlExpressError::syntax(token.offset, format!("invalid number: {error}"))
                })?;
                number_value(number)?
            } else {
                let number = token.lexeme.parse::<i64>().map_err(|error| {
                    QlExpressError::syntax(token.offset, format!("invalid integer: {error}"))
                })?;
                Value::Number(Number::from(number))
            };
            return Ok(Expression::Literal(value));
        }
        if self.matches(TokenKind::String) {
            return Ok(Expression::Literal(Value::String(
                self.previous().lexeme.clone(),
            )));
        }
        if self.matches(TokenKind::LeftParen) {
            let expression = self.expression(0)?;
            self.expect(TokenKind::RightParen, "expected ')' after expression")?;
            return Ok(expression);
        }
        if self.matches(TokenKind::Identifier) {
            let mut target = vec![self.previous().lexeme.clone()];
            while self.matches(TokenKind::Dot) {
                target.push(
                    self.expect(TokenKind::Identifier, "expected identifier after '.'")?
                        .lexeme
                        .clone(),
                );
            }
            if self.matches(TokenKind::LeftParen) {
                let mut arguments = Vec::new();
                if !self.check(TokenKind::RightParen) {
                    loop {
                        arguments.push(self.expression(0)?);
                        if !self.matches(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                return Ok(Expression::Call { target, arguments });
            }
            if target.len() == 1 {
                return Ok(Expression::Variable(target.remove(0)));
            }
            return Err(QlExpressError::syntax(
                self.previous().offset,
                "property access without method call is not supported",
            ));
        }
        Err(QlExpressError::syntax(
            self.peek().offset,
            format!("expected expression, found [{}]", self.peek().lexeme),
        ))
    }

    fn consume_optional_semicolon(&mut self) {
        self.matches(TokenKind::Semicolon);
    }

    fn expect(
        &mut self,
        kind: TokenKind,
        message: impl Into<String>,
    ) -> Result<&Token, QlExpressError> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        Err(QlExpressError::syntax(self.peek().offset, message))
    }

    fn matches(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn check_next(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.current + 1)
            .is_some_and(|token| token.kind == kind)
    }

    fn advance(&mut self) -> &Token {
        if !self.check(TokenKind::Eof) {
            self.current += 1;
        }
        self.previous()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }
}

fn binary_precedence(kind: TokenKind) -> Option<u8> {
    match kind {
        TokenKind::OrOr => Some(1),
        TokenKind::AndAnd => Some(2),
        TokenKind::EqualEqual | TokenKind::BangEqual => Some(3),
        TokenKind::Greater | TokenKind::GreaterEqual | TokenKind::Less | TokenKind::LessEqual => {
            Some(4)
        }
        TokenKind::Plus | TokenKind::Minus => Some(5),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some(6),
        _ => None,
    }
}

#[derive(Debug)]
struct QlExpressError {
    message: String,
}

impl QlExpressError {
    fn syntax(offset: usize, message: impl Into<String>) -> Self {
        Self {
            message: format!("syntax error at byte {offset}: {}", message.into()),
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            message: format!("runtime error: {}", message.into()),
        }
    }
}

impl std::fmt::Display for QlExpressError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for QlExpressError {}
