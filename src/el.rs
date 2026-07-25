//! EL 表达式解析器。
//!
//! 语义对齐 liteflow-core 的 EL 规则（2.12/2.13 语法），支持：
//! THEN / WHEN / IF / ELIF / ELSE / SWITCH / TO / DEFAULT / FOR / WHILE / ITERATOR /
//! DO / BREAK / CATCH / PRE / FINALLY / AND / OR / NOT / NODE / PARALLEL / RETRY /
//! ANY / MUST / PERCENTAGE / IGNORE_ERROR / MAX_WAIT_SECONDS / MAX_WAIT_MILLISECONDS /
//! THREAD_POOL / ID / TAG / DATA / BIND。
//!
//! Java 版 2.11 之后底层用 QLExpress4 解析，这里用手写递归下降解析器，
//! 接受文档规范内的全部 EL 写法。

use crate::exception::{LFResult, LiteflowError};

/// 节点引用，对应 Java 的 Node 元素 + id/tag/data/bind 修饰
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRef {
    pub id: String,
    /// .id("xxx") 别名（同一组件在一条链中的多个实例）
    pub alias: Option<String>,
    pub tag: Option<String>,
    pub data: Option<String>,
    pub bind: Vec<(String, String)>,
}

impl NodeRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            alias: None,
            tag: None,
            data: None,
            bind: Vec::new(),
        }
    }
    /// 展示名，对齐 Java 的 getDisplayName（优先别名）
    pub fn display(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.id)
    }
}

/// WHEN 的选项，对应 WhenCondition 的字段
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WhenOpts {
    pub any: bool,
    pub ignore_error: bool,
    pub percentage: Option<f64>,
    pub must: Vec<String>,
    pub max_wait_ms: Option<u64>,
    /// 线程池名（Java 语义）；Rust 端记录在案，执行统一走 tokio 调度
    pub thread_pool: Option<String>,
}

/// 通用修饰（可包裹任意表达式），对应 RetryCondition / TimeoutCondition / ignoreError
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mods {
    pub retry: Option<u32>,
    pub max_wait_ms: Option<u64>,
    pub ignore_error: bool,
}

/// EL 语法树，对应 liteflow-core flow.element.condition 包的全部 Condition 类型
#[derive(Debug, Clone, PartialEq)]
pub enum El {
    Node(NodeRef),
    Then(Vec<El>),
    When { items: Vec<El>, opts: WhenOpts },
    If {
        cond: Box<El>,
        then: Box<El>,
        elifs: Vec<(El, El)>,
        els: Option<Box<El>>,
    },
    Switch {
        node: Box<El>,
        targets: Vec<El>,
        default: Option<Box<El>>,
    },
    For {
        node: Box<El>,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    While {
        node: Box<El>,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    Iter {
        node: Box<El>,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    Catch { body: Box<El>, do_: Option<Box<El>> },
    And(Vec<El>),
    Or(Vec<El>),
    Not(Box<El>),
    /// PRE 子流程（仅 THEN 内有特殊语义）
    Pre(Box<El>),
    /// FINALLY 子流程（仅 THEN 内有特殊语义）
    Fin(Box<El>),
    /// RETRY / MAX_WAIT / IGNORE_ERROR 修饰
    Mods(Box<El>, Mods),
}

// ---------------- 词法 ----------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Num(f64),
    Bool(bool),
    LP,
    RP,
    Comma,
    Dot,
}

fn lex(s: &str) -> LFResult<Vec<Tok>> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                out.push(Tok::LP);
                i += 1;
            }
            ')' => {
                out.push(Tok::RP);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '.' => {
                out.push(Tok::Dot);
                i += 1;
            }
            '"' | '\'' => {
                let quote = c;
                i += 1;
                let mut buf = String::new();
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        match chars[i] {
                            'n' => buf.push('\n'),
                            't' => buf.push('\t'),
                            other => buf.push(other),
                        }
                    } else {
                        buf.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(LiteflowError::Parse("unclosed string literal".into()));
                }
                i += 1;
                out.push(Tok::Str(buf));
            }
            c if c.is_ascii_digit() || (c == '-' && chars.get(i + 1).map_or(false, |n| n.is_ascii_digit())) => {
                let mut j = i;
                if chars[j] == '-' {
                    j += 1;
                }
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                    j += 1;
                }
                let text: String = chars[i..j].iter().collect();
                let v: f64 = text
                    .parse()
                    .map_err(|_| LiteflowError::Parse(format!("invalid number: {text}")))?;
                out.push(Tok::Num(v));
                i = j;
            }
            c if c.is_alphabetic() || c == '_' || c == '$' => {
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '$') {
                    j += 1;
                }
                let word: String = chars[i..j].iter().collect();
                match word.as_str() {
                    "true" => out.push(Tok::Bool(true)),
                    "false" => out.push(Tok::Bool(false)),
                    _ => out.push(Tok::Ident(word)),
                }
                i = j;
            }
            ':' => {
                // 支持 SWITCH(x).TO(a:tag1) 这种无引号写法：并入 ident
                i += 1;
            }
            other => {
                return Err(LiteflowError::Parse(format!("unexpected character: {other}")));
            }
        }
    }
    Ok(out)
}

// ---------------- 语法 ----------------

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

/// 方法调用参数
#[derive(Debug, Clone)]
enum Arg {
    Expr(El),
    Str(String),
    Num(f64),
    Bool(bool),
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, t: Tok) -> LFResult<()> {
        match self.next() {
            Some(x) if x == t => Ok(()),
            other => Err(LiteflowError::Parse(format!(
                "expect {:?}, but got {:?}",
                t, other
            ))),
        }
    }


    /// expr := primary ( '.' method(args) )*
    fn parse_expr(&mut self) -> LFResult<El> {
        let mut e = self.parse_primary()?;
        while matches!(self.peek(), Some(Tok::Dot)) {
            self.next(); // consume '.'
            let name = match self.next() {
                Some(Tok::Ident(w)) => w,
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "expect method name after '.', got {:?}",
                        other
                    )))
                }
            };
            if matches!(self.peek(), Some(Tok::LP)) {
                let args = self.parse_args()?;
                e = Self::apply_method(e, &name, args)?;
            } else {
                // 无括号形式：声明式组件方法引用 cmpId.methodName
                // （对应 @LiteflowMethod；已知关键字不允许省略括号）
                e = Self::apply_method_ref(e, &name)?;
            }
        }
        Ok(e)
    }

    fn parse_args(&mut self) -> LFResult<Vec<Arg>> {
        self.expect(Tok::LP)?;
        let mut args = Vec::new();
        if matches!(self.peek(), Some(Tok::RP)) {
            self.next();
            return Ok(args);
        }
        loop {
            // 参数既可能是子表达式，也可能是字符串/数字/布尔字面量。
            // 判定方式：Ident 开头且为已知关键字 → 子表达式；否则若是 Ident 且
            // 下一个 token 是 '.' 或 '(' 或 ',' 或 ')'，按上下文区分。
            let arg = self.parse_arg()?;
            args.push(arg);
            match self.next() {
                Some(Tok::Comma) => continue,
                Some(Tok::RP) => break,
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "expect ',' or ')', got {:?}",
                        other
                    )))
                }
            }
        }
        Ok(args)
    }

    fn parse_arg(&mut self) -> LFResult<Arg> {
        match self.peek().cloned() {
            Some(Tok::Str(s)) => {
                self.next();
                Ok(Arg::Str(s))
            }
            Some(Tok::Num(n)) => {
                self.next();
                Ok(Arg::Num(n))
            }
            Some(Tok::Bool(b)) => {
                self.next();
                Ok(Arg::Bool(b))
            }
            Some(Tok::Ident(_)) => {
                // 子表达式（关键字或节点引用，均可带方法链）
                Ok(Arg::Expr(self.parse_expr()?))
            }
            other => Err(LiteflowError::Parse(format!(
                "unexpected arg token: {:?}",
                other
            ))),
        }
    }

    /// primary := KEYWORD(...) | node_id | NODE("id") | string-target
    fn parse_primary(&mut self) -> LFResult<El> {
        match self.next() {
            Some(Tok::Ident(w)) => {
                let kw = w.to_ascii_uppercase();
                match kw.as_str() {
                    "THEN" => Ok(El::Then(self.parse_expr_list()?)),
                    "WHEN" => Ok(El::When {
                        items: self.parse_expr_list()?,
                        opts: WhenOpts::default(),
                    }),
                    "AND" => Ok(El::And(self.parse_expr_list()?)),
                    "OR" => Ok(El::Or(self.parse_expr_list()?)),
                    "NOT" => {
                        let list = self.parse_expr_list()?;
                        if list.len() != 1 {
                            return Err(LiteflowError::Parse(
                                "NOT requires exactly one argument".into(),
                            ));
                        }
                        Ok(El::Not(Box::new(list.into_iter().next().unwrap())))
                    }
                    "IF" => {
                        let args = self.parse_args()?;
                        self.build_if(args)
                    }
                    "SWITCH" => {
                        let list = self.parse_expr_list()?;
                        if list.len() != 1 {
                            return Err(LiteflowError::Parse(
                                "SWITCH requires exactly one node".into(),
                            ));
                        }
                        Ok(El::Switch {
                            node: Box::new(list.into_iter().next().unwrap()),
                            targets: Vec::new(),
                            default: None,
                        })
                    }
                    "FOR" => {
                        let list = self.parse_expr_list()?;
                        if list.len() != 1 {
                            return Err(LiteflowError::Parse(
                                "FOR requires exactly one node".into(),
                            ));
                        }
                        Ok(El::For {
                            node: Box::new(list.into_iter().next().unwrap()),
                            parallel: None,
                            body: Box::new(El::Then(vec![])),
                            brk: None,
                        })
                    }
                    "WHILE" => {
                        let list = self.parse_expr_list()?;
                        if list.len() != 1 {
                            return Err(LiteflowError::Parse(
                                "WHILE requires exactly one node".into(),
                            ));
                        }
                        Ok(El::While {
                            node: Box::new(list.into_iter().next().unwrap()),
                            parallel: None,
                            body: Box::new(El::Then(vec![])),
                            brk: None,
                        })
                    }
                    "ITERATOR" => {
                        let list = self.parse_expr_list()?;
                        if list.len() != 1 {
                            return Err(LiteflowError::Parse(
                                "ITERATOR requires exactly one node".into(),
                            ));
                        }
                        Ok(El::Iter {
                            node: Box::new(list.into_iter().next().unwrap()),
                            parallel: None,
                            body: Box::new(El::Then(vec![])),
                            brk: None,
                        })
                    }
                    "CATCH" => {
                        let list = self.parse_expr_list()?;
                        if list.len() != 1 {
                            return Err(LiteflowError::Parse(
                                "CATCH requires exactly one expression".into(),
                            ));
                        }
                        Ok(El::Catch {
                            body: Box::new(list.into_iter().next().unwrap()),
                            do_: None,
                        })
                    }
                    "PRE" => {
                        let list = self.parse_expr_list()?;
                        Ok(El::Pre(Box::new(El::Then(list))))
                    }
                    "FINALLY" => {
                        let list = self.parse_expr_list()?;
                        Ok(El::Fin(Box::new(El::Then(list))))
                    }
                    "NODE" => {
                        let args = self.parse_args()?;
                        match args.as_slice() {
                            [Arg::Str(id)] => Ok(El::Node(NodeRef::new(id.clone()))),
                            _ => Err(LiteflowError::Parse(
                                "NODE requires one string argument".into(),
                            )),
                        }
                    }
                    "RETRY" => {
                        // RETRY(n) 作为前缀形式：RETRY(3, expr)? —— Java 只有后缀形式，
                        // 这里仅兼容 expr.RETRY(n)，前缀形式按错误处理。
                        Err(LiteflowError::Parse(
                            "RETRY must be used as a suffix: expr.RETRY(n)".into(),
                        ))
                    }
                    _ => Ok(El::Node(NodeRef::new(w))),
                }
            }
            Some(Tok::Str(s)) => {
                // 字符串形式的节点引用（用于 SWITCH TO 目标），支持 "id" 或 "id:tag"
                let mut parts = s.splitn(2, ':');
                let id = parts.next().unwrap_or("").to_string();
                let tag = parts.next().map(|t| t.to_string());
                let mut n = NodeRef::new(id);
                n.tag = tag;
                Ok(El::Node(n))
            }
            other => Err(LiteflowError::Parse(format!(
                "unexpected token: {:?}",
                other
            ))),
        }
    }

    fn parse_expr_list(&mut self) -> LFResult<Vec<El>> {
        let args = self.parse_args()?;
        let mut list = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => list.push(e),
                Arg::Str(s) => {
                    // 字符串节点引用（支持 "id:tag"）
                    let mut parts = s.splitn(2, ':');
                    let id = parts.next().unwrap_or("").to_string();
                    let tag = parts.next().map(|t| t.to_string());
                    let mut n = NodeRef::new(id);
                    n.tag = tag;
                    list.push(El::Node(n));
                }
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "invalid expression item: {:?}",
                        other
                    )))
                }
            }
        }
        if list.is_empty() {
            return Err(LiteflowError::Parse("empty expression list".into()));
        }
        Ok(list)
    }

    fn build_if(&mut self, args: Vec<Arg>) -> LFResult<El> {
        let mut it = args.into_iter();
        let cond = match it.next() {
            Some(Arg::Expr(e)) => e,
            _ => return Err(LiteflowError::Parse("IF requires a condition".into())),
        };
        let then = match it.next() {
            Some(Arg::Expr(e)) => e,
            _ => return Err(LiteflowError::Parse("IF requires a true-case".into())),
        };
        let els = match it.next() {
            Some(Arg::Expr(e)) => Some(Box::new(e)),
            None => None,
            _ => return Err(LiteflowError::Parse("invalid IF arguments".into())),
        };
        Ok(El::If {
            cond: Box::new(cond),
            then: Box::new(then),
            elifs: Vec::new(),
            els,
        })
    }

    fn apply_method(e: El, name: &str, args: Vec<Arg>) -> LFResult<El> {
        let kw = name.to_ascii_uppercase();
        match kw.as_str() {
            "ELSE" => match (e, args.as_slice()) {
                (El::If { cond, then, elifs, .. }, [Arg::Expr(els)]) => Ok(El::If {
                    cond,
                    then,
                    elifs,
                    els: Some(Box::new(els.clone())),
                }),
                _ => Err(LiteflowError::Parse("ELSE must follow IF".into())),
            },
            "ELIF" => match (e, args.as_slice()) {
                (El::If { cond, then, mut elifs, els }, [Arg::Expr(c), Arg::Expr(t)]) => {
                    elifs.push((c.clone(), t.clone()));
                    Ok(El::If { cond, then, elifs, els })
                }
                _ => Err(LiteflowError::Parse("ELIF must follow IF".into())),
            },
            "TO" => match e {
                El::Switch { node, default, .. } => {
                    let mut targets = Vec::new();
                    for a in args {
                        match a {
                            Arg::Expr(t) => targets.push(t),
                            Arg::Str(s) => {
                                let mut parts = s.splitn(2, ':');
                                let id = parts.next().unwrap_or("").to_string();
                                let tag = parts.next().map(|x| x.to_string());
                                let mut n = NodeRef::new(id);
                                n.tag = tag;
                                targets.push(El::Node(n));
                            }
                            other => {
                                return Err(LiteflowError::Parse(format!(
                                    "invalid TO target: {:?}",
                                    other
                                )))
                            }
                        }
                    }
                    Ok(El::Switch { node, targets, default })
                }
                _ => Err(LiteflowError::Parse("TO must follow SWITCH".into())),
            },
            "DEFAULT" => match (e, args.as_slice()) {
                (El::Switch { node, targets, .. }, [Arg::Expr(d)]) => Ok(El::Switch {
                    node,
                    targets,
                    default: Some(Box::new(d.clone())),
                }),
                _ => Err(LiteflowError::Parse("DEFAULT must follow SWITCH".into())),
            },
            "DO" => {
                let body = match args.as_slice() {
                    [Arg::Expr(b)] => Box::new(b.clone()),
                    _ => return Err(LiteflowError::Parse("DO requires one expression".into())),
                };
                match e {
                    El::For { node, parallel, brk, .. } => Ok(El::For { node, parallel, body, brk }),
                    El::While { node, parallel, brk, .. } => Ok(El::While { node, parallel, body, brk }),
                    El::Iter { node, parallel, brk, .. } => Ok(El::Iter { node, parallel, body, brk }),
                    El::Catch { body: c, .. } => Ok(El::Catch { body: c, do_: Some(body) }),
                    _ => Err(LiteflowError::Parse(
                        "DO must follow FOR/WHILE/ITERATOR/CATCH".into(),
                    )),
                }
            }
            "BREAK" => {
                let brk = match args.as_slice() {
                    [Arg::Expr(b)] => Some(Box::new(b.clone())),
                    _ => return Err(LiteflowError::Parse("BREAK requires one node".into())),
                };
                match e {
                    El::For { node, parallel, body, .. } => Ok(El::For { node, parallel, body, brk }),
                    El::While { node, parallel, body, .. } => Ok(El::While { node, parallel, body, brk }),
                    El::Iter { node, parallel, body, .. } => Ok(El::Iter { node, parallel, body, brk }),
                    _ => Err(LiteflowError::Parse(
                        "BREAK must follow FOR/WHILE/ITERATOR".into(),
                    )),
                }
            }
            "PARALLEL" => {
                let n = match args.as_slice() {
                    [Arg::Num(n)] => *n as usize,
                    _ => return Err(LiteflowError::Parse("PARALLEL requires a number".into())),
                };
                match e {
                    El::For { node, body, brk, .. } => Ok(El::For { node, parallel: Some(n), body, brk }),
                    El::While { node, body, brk, .. } => Ok(El::While { node, parallel: Some(n), body, brk }),
                    El::Iter { node, body, brk, .. } => Ok(El::Iter { node, parallel: Some(n), body, brk }),
                    _ => Err(LiteflowError::Parse(
                        "PARALLEL must follow FOR/WHILE/ITERATOR".into(),
                    )),
                }
            }
            "RETRY" => {
                let n = match args.as_slice() {
                    [Arg::Num(n)] => *n as u32,
                    _ => return Err(LiteflowError::Parse("RETRY requires a number".into())),
                };
                Ok(Self::add_mods(e, Mods { retry: Some(n), ..Default::default() }))
            }
            "IGNORE_ERROR" => {
                let b = match args.as_slice() {
                    [Arg::Bool(b)] => *b,
                    _ => return Err(LiteflowError::Parse("IGNORE_ERROR requires a bool".into())),
                };
                match e {
                    El::When { items, mut opts } => {
                        opts.ignore_error = b;
                        Ok(El::When { items, opts })
                    }
                    other => Ok(Self::add_mods(other, Mods { ignore_error: b, ..Default::default() })),
                }
            }
            "ANY" => {
                let b = match args.as_slice() {
                    [Arg::Bool(b)] => *b,
                    _ => return Err(LiteflowError::Parse("ANY requires a bool".into())),
                };
                match e {
                    El::When { items, mut opts } => {
                        opts.any = b;
                        Ok(El::When { items, opts })
                    }
                    _ => Err(LiteflowError::Parse("ANY must follow WHEN".into())),
                }
            }
            "PERCENTAGE" => {
                let n = match args.as_slice() {
                    [Arg::Num(n)] => *n,
                    _ => return Err(LiteflowError::Parse("PERCENTAGE requires a number".into())),
                };
                match e {
                    El::When { items, mut opts } => {
                        opts.percentage = Some(n);
                        Ok(El::When { items, opts })
                    }
                    _ => Err(LiteflowError::Parse("PERCENTAGE must follow WHEN".into())),
                }
            }
            "MUST" => {
                let mut ids = Vec::new();
                for a in &args {
                    match a {
                        Arg::Str(s) => ids.push(s.clone()),
                        Arg::Expr(El::Node(n)) => ids.push(n.id.clone()),
                        other => {
                            return Err(LiteflowError::Parse(format!(
                                "invalid MUST argument: {:?}",
                                other
                            )))
                        }
                    }
                }
                match e {
                    El::When { items, mut opts } => {
                        opts.must = ids;
                        Ok(El::When { items, opts })
                    }
                    _ => Err(LiteflowError::Parse("MUST must follow WHEN".into())),
                }
            }
            "MAX_WAIT_SECONDS" => {
                let n = match args.as_slice() {
                    [Arg::Num(n)] => *n,
                    _ => return Err(LiteflowError::Parse("MAX_WAIT_SECONDS requires a number".into())),
                };
                let ms = (n * 1000.0) as u64;
                match e {
                    El::When { items, mut opts } => {
                        opts.max_wait_ms = Some(ms);
                        Ok(El::When { items, opts })
                    }
                    other => Ok(Self::add_mods(
                        other,
                        Mods { max_wait_ms: Some(ms), ..Default::default() },
                    )),
                }
            }
            "MAX_WAIT_MILLISECONDS" | "MAX_WAIT_TIME" => {
                let n = match args.as_slice() {
                    [Arg::Num(n)] => *n as u64,
                    _ => return Err(LiteflowError::Parse("MAX_WAIT_MILLISECONDS requires a number".into())),
                };
                match e {
                    El::When { items, mut opts } => {
                        opts.max_wait_ms = Some(n);
                        Ok(El::When { items, opts })
                    }
                    other => Ok(Self::add_mods(
                        other,
                        Mods { max_wait_ms: Some(n), ..Default::default() },
                    )),
                }
            }
            "THREAD_POOL" => {
                let name = match args.as_slice() {
                    [Arg::Str(s)] => s.clone(),
                    _ => return Err(LiteflowError::Parse("THREAD_POOL requires a string".into())),
                };
                match e {
                    El::When { items, mut opts } => {
                        opts.thread_pool = Some(name);
                        Ok(El::When { items, opts })
                    }
                    _ => Err(LiteflowError::Parse("THREAD_POOL must follow WHEN".into())),
                }
            }
            "ID" => {
                let v = match args.as_slice() {
                    [Arg::Str(s)] => s.clone(),
                    _ => return Err(LiteflowError::Parse("ID requires a string".into())),
                };
                match e {
                    El::Node(mut n) => {
                        n.alias = Some(v);
                        Ok(El::Node(n))
                    }
                    _ => Err(LiteflowError::Parse("ID must follow a node".into())),
                }
            }
            "TAG" => {
                let v = match args.as_slice() {
                    [Arg::Str(s)] => s.clone(),
                    _ => return Err(LiteflowError::Parse("TAG requires a string".into())),
                };
                match e {
                    El::Node(mut n) => {
                        n.tag = Some(v);
                        Ok(El::Node(n))
                    }
                    _ => Err(LiteflowError::Parse("TAG must follow a node".into())),
                }
            }
            "DATA" => {
                let v = match args.as_slice() {
                    [Arg::Str(s)] => s.clone(),
                    _ => return Err(LiteflowError::Parse("DATA requires a string".into())),
                };
                match e {
                    El::Node(mut n) => {
                        n.data = Some(v);
                        Ok(El::Node(n))
                    }
                    _ => Err(LiteflowError::Parse("DATA must follow a node".into())),
                }
            }
            "BIND" => {
                let mut pairs = Vec::new();
                let mut it = args.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    match (k, v) {
                        (Arg::Str(k), Arg::Str(v)) => pairs.push((k, v)),
                        _ => {
                            return Err(LiteflowError::Parse(
                                "BIND requires pairs of strings".into(),
                            ))
                        }
                    }
                }
                match e {
                    El::Node(mut n) => {
                        n.bind = pairs;
                        Ok(El::Node(n))
                    }
                    _ => Err(LiteflowError::Parse("BIND must follow a node".into())),
                }
            }
            _ => Err(LiteflowError::Parse(format!("unknown method: {name}"))),
        }
    }

    /// 声明式组件方法引用：node.method → 节点 id 变为 "node.method"
    fn apply_method_ref(e: El, name: &str) -> LFResult<El> {
        // 已知关键字必须带括号
        const KEYWORDS: &[&str] = &[
            "ELSE", "ELIF", "TO", "DEFAULT", "DO", "BREAK", "PARALLEL", "RETRY",
            "IGNORE_ERROR", "ANY", "PERCENTAGE", "MUST", "MAX_WAIT_SECONDS",
            "MAX_WAIT_MILLISECONDS", "MAX_WAIT_TIME", "THREAD_POOL", "ID", "TAG",
            "DATA", "BIND",
        ];
        if KEYWORDS.contains(&name.to_ascii_uppercase().as_str()) {
            return Err(LiteflowError::Parse(format!(
                "{name} requires parentheses"
            )));
        }
        match e {
            El::Node(mut n) => {
                n.id = format!("{}.{}", n.id, name);
                Ok(El::Node(n))
            }
            _ => Err(LiteflowError::Parse(format!(
                "method reference must follow a node: .{name}"
            ))),
        }
    }

    /// 合并修饰到已有 Mods 包装（Java 允许多个修饰叠加，如 a.retry(2).maxWaitSeconds(3)）
    fn add_mods(e: El, m: Mods) -> El {
        match e {
            El::Mods(inner, mut old) => {
                if m.retry.is_some() {
                    old.retry = m.retry;
                }
                if m.max_wait_ms.is_some() {
                    old.max_wait_ms = m.max_wait_ms;
                }
                old.ignore_error = old.ignore_error || m.ignore_error;
                El::Mods(inner, old)
            }
            other => El::Mods(Box::new(other), m),
        }
    }
}

/// 解析 EL 文本为语法树。对齐 LiteFlowChainELBuilder.setEL(...)
pub fn parse_el(text: &str) -> LFResult<El> {
    let toks = lex(text)?;
    if toks.is_empty() {
        return Err(LiteflowError::Parse("empty EL".into()));
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.parse_expr()?;
    if p.pos != p.toks.len() {
        return Err(LiteflowError::Parse(format!(
            "unexpected trailing tokens at pos {}",
            p.pos
        )));
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_then_when() {
        let e = parse_el("THEN(a, b, WHEN(c, d, e), f)").unwrap();
        match e {
            El::Then(items) => {
                assert_eq!(items.len(), 4);
                assert!(matches!(&items[2], El::When { items, .. } if items.len() == 3));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_if_elif_else() {
        let e = parse_el("IF(x, a).ELIF(y, b).ELSE(c)").unwrap();
        match e {
            El::If { elifs, els, .. } => {
                assert_eq!(elifs.len(), 1);
                assert!(els.is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_switch_to_default() {
        let e = parse_el(r#"SWITCH(s).TO(a, b, "c:tag1").DEFAULT(d)"#).unwrap();
        match e {
            El::Switch { targets, default, .. } => {
                assert_eq!(targets.len(), 3);
                match &targets[2] {
                    El::Node(n) => {
                        assert_eq!(n.id, "c");
                        assert_eq!(n.tag.as_deref(), Some("tag1"));
                    }
                    other => panic!("{other:?}"),
                }
                assert!(default.is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_for_do_break() {
        let e = parse_el("FOR(f).PARALLEL(2).DO(THEN(a, b)).BREAK(x)").unwrap();
        match e {
            El::For { parallel, brk, .. } => {
                assert_eq!(parallel, Some(2));
                assert!(brk.is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_modifiers() {
        let e = parse_el("a.tag(\"t1\").data(\"{'k':1}\").retry(3)").unwrap();
        match e {
            El::Mods(inner, m) => {
                assert_eq!(m.retry, Some(3));
                match *inner {
                    El::Node(n) => {
                        assert_eq!(n.tag.as_deref(), Some("t1"));
                        assert!(n.data.is_some());
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_when_opts() {
        let e = parse_el("WHEN(a, b, c).ANY(true).MAX_WAIT_SECONDS(2)").unwrap();
        match e {
            El::When { opts, .. } => {
                assert!(opts.any);
                assert_eq!(opts.max_wait_ms, Some(2000));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_catch_do() {
        let e = parse_el("THEN(CATCH(THEN(a, b)).DO(handle), d)").unwrap();
        assert!(matches!(e, El::Then(_)));
    }

    #[test]
    fn parse_pre_finally() {
        let e = parse_el("THEN(PRE(p), a, b, FINALLY(z))").unwrap();
        match e {
            El::Then(items) => {
                assert!(matches!(items[0], El::Pre(_)));
                assert!(matches!(items[3], El::Fin(_)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_and_or_not() {
        let e = parse_el("IF(AND(x, OR(y, NOT(z))), a, b)").unwrap();
        assert!(matches!(e, El::If { .. }));
    }
}
