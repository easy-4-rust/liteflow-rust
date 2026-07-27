//! EL 表达式递归下降解析器。
//!
//! 语义对齐 liteflow-core 的 EL 规则（2.12/2.13 语法），支持：
//! THEN / WHEN / IF / ELIF / ELSE / SWITCH / TO / DEFAULT / FOR / WHILE / ITERATOR /
//! DO / BREAK / CATCH / PRE / FINALLY / AND / OR / NOT / NODE / PARALLEL / RETRY /
//! ANY / MUST / PERCENTAGE / IGNORE_ERROR / MAX_WAIT_SECONDS / MAX_WAIT_MILLISECONDS /
//! THREAD_POOL / ID / TAG / DATA / BIND。
//!
//! Java 版 2.11 之后底层用 QLExpress4 解析，这里用手写递归下降解析器，
//! 接受文档规范内的全部 EL 写法。

use super::{Arg, El, NodeRef, SpannedTok, Tok, format_el_parse_error, lex};
use crate::builder::el::operator::base::BaseOperator;
use crate::builder::el::operator::{
    AndOperator, AnyOperator, BindOperator, BreakOperator, CatchOperator, DataOperator,
    DefaultOperator, DoOperator, ElifOperator, ElseOperator, FinallyOperator, ForOperator,
    IdOperator, IfOperator, IgnoreErrorOperator, IteratorOperator, MaxWaitMillisecondsOperator,
    MaxWaitSecondsOperator, MustOperator, NodeOperator, NotOperator, OrOperator, ParallelOperator,
    PercentageOperator, PreOperator, RetryOperator, SwitchOperator, TagOperator, ThenOperator,
    ThreadPoolOperator, ToOperator, WhenOperator, WhileOperator,
};
use crate::exception::{LFResult, LiteflowError};

// ---------------- 语法 ----------------

struct Parser<'a> {
    source: &'a str,
    toks: Vec<SpannedTok>,
    pos: usize,
}

/// 把 Java Builder 的 camelCase 方法名和文档中的 snake_case 写法统一为
/// 解析器内部的 SCREAMING_SNAKE_CASE 关键字。
fn normalize_method_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len() + 4);
    let mut previous = None;
    for character in name.chars() {
        if character == '-' {
            normalized.push('_');
        } else if character.is_ascii_uppercase()
            && previous
                .is_some_and(|value: char| value.is_ascii_lowercase() || value.is_ascii_digit())
            && !normalized.ends_with('_')
        {
            normalized.push('_');
            normalized.push(character);
        } else {
            normalized.push(character.to_ascii_uppercase());
        }
        previous = Some(character);
    }
    normalized
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(token, _)| token)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).map(|(token, _)| token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn error_position(&self) -> usize {
        self.toks
            .get(self.pos)
            .or_else(|| self.toks.last())
            .map_or_else(|| self.source.chars().count(), |(_, position)| *position)
    }
    fn parse_error(&self, detail: impl AsRef<str>) -> LiteflowError {
        format_el_parse_error(self.source, self.error_position(), detail)
    }
    fn contextualize(&self, error: LiteflowError) -> LiteflowError {
        match error {
            LiteflowError::Parse(detail)
                if !detail.contains("\n EL: ") && !detail.contains("\n EL:") =>
            {
                self.parse_error(detail)
            }
            other => other,
        }
    }
    fn expect(&mut self, t: Tok) -> LFResult<()> {
        match self.next() {
            Some(x) if x == t => Ok(()),
            other => Err(self.parse_error(format!("expect {:?}, but got {:?}", t, other))),
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
                    return Err(
                        self.parse_error(format!("expect method name after '.', got {:?}", other))
                    );
                }
            };
            if matches!(self.peek(), Some(Tok::LP)) {
                let args = self.parse_args()?;
                e = Self::apply_method(e, &name, args)
                    .map_err(|error| self.contextualize(error))?;
            } else {
                // 无括号形式：声明式组件方法引用 cmpId.methodName
                // （对应 @LiteflowMethod；已知关键字不允许省略括号）
                e = Self::apply_method_ref(e, &name).map_err(|error| self.contextualize(error))?;
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
                    return Err(self.parse_error(format!("expect ',' or ')', got {:?}", other)));
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
            other => Err(self.parse_error(format!("unexpected arg token: {:?}", other))),
        }
    }

    /// primary := KEYWORD(...) | node_id | NODE("id") | string-target
    fn parse_primary(&mut self) -> LFResult<El> {
        match self.next() {
            Some(Tok::Ident(w)) => {
                let kw = w.to_ascii_uppercase();
                match kw.as_str() {
                    "THEN" | "SER" => ThenOperator.build(None, self.parse_args()?),
                    "WHEN" | "PAR" => WhenOperator.build(None, self.parse_args()?),
                    "AND" => AndOperator.build(None, self.parse_args()?),
                    "OR" => OrOperator.build(None, self.parse_args()?),
                    "NOT" => NotOperator.build(None, self.parse_args()?),
                    "IF" => IfOperator.build(None, self.parse_args()?),
                    "SWITCH" => SwitchOperator.build(None, self.parse_args()?),
                    "FOR" => ForOperator.build(None, self.parse_args()?),
                    "WHILE" => WhileOperator.build(None, self.parse_args()?),
                    "ITERATOR" => IteratorOperator.build(None, self.parse_args()?),
                    "CATCH" => CatchOperator.build(None, self.parse_args()?),
                    "PRE" => PreOperator.build(None, self.parse_args()?),
                    "FINALLY" => FinallyOperator.build(None, self.parse_args()?),
                    "NODE" => NodeOperator.build(None, self.parse_args()?),
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
            other => Err(self.parse_error(format!("unexpected token: {:?}", other))),
        }
    }

    fn apply_method(e: El, name: &str, args: Vec<Arg>) -> LFResult<El> {
        let operator = normalize_method_name(name);
        match operator.as_str() {
            "ELSE" => ElseOperator.build(Some(e), args),
            "ELIF" => ElifOperator.build(Some(e), args),
            "TO" => ToOperator.build(Some(e), args),
            "DEFAULT" => DefaultOperator.build(Some(e), args),
            "DO" => DoOperator.build(Some(e), args),
            "BREAK" => BreakOperator.build(Some(e), args),
            "PARALLEL" => ParallelOperator.build(Some(e), args),
            "THREAD_POOL" => ThreadPoolOperator.build(Some(e), args),
            "ID" => IdOperator.build(Some(e), args),
            "TAG" => TagOperator.build(Some(e), args),
            "DATA" => DataOperator.build(Some(e), args),
            "BIND" => BindOperator.build(Some(e), args),
            "RETRY" => RetryOperator.build(Some(e), args),
            "IGNORE_ERROR" => IgnoreErrorOperator.build(Some(e), args),
            "ANY" => AnyOperator.build(Some(e), args),
            "PERCENTAGE" => PercentageOperator.build(Some(e), args),
            "MUST" => MustOperator.build(Some(e), args),
            "MAX_WAIT_SECONDS" => MaxWaitSecondsOperator.build(Some(e), args),
            "MAX_WAIT_MILLISECONDS" | "MAX_WAIT_TIME" => {
                MaxWaitMillisecondsOperator.build(Some(e), args)
            }
            _ => Err(LiteflowError::Parse(format!("unknown method: {name}"))),
        }
    }

    /// 声明式组件方法引用：node.method → 节点 id 变为 "node.method"
    fn apply_method_ref(e: El, name: &str) -> LFResult<El> {
        // 已知关键字必须带括号
        const KEYWORDS: &[&str] = &[
            "ELSE",
            "ELIF",
            "TO",
            "DEFAULT",
            "DO",
            "BREAK",
            "PARALLEL",
            "RETRY",
            "IGNORE_ERROR",
            "ANY",
            "PERCENTAGE",
            "MUST",
            "MAX_WAIT_SECONDS",
            "MAX_WAIT_MILLISECONDS",
            "MAX_WAIT_TIME",
            "THREAD_POOL",
            "ID",
            "TAG",
            "DATA",
            "BIND",
        ];
        if KEYWORDS.contains(&normalize_method_name(name).as_str()) {
            return Err(LiteflowError::Parse(format!("{name} requires parentheses")));
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
}

/// 解析 EL 文本为语法树。对齐 LiteFlowChainELBuilder.setEL(...)
pub fn parse_el(text: &str) -> LFResult<El> {
    let toks = lex(text)?;
    if toks.is_empty() {
        return Err(format_el_parse_error(text, 0, "empty EL"));
    }
    let mut p = Parser {
        source: text,
        toks,
        pos: 0,
    };
    let e = p.parse_expr()?;
    if p.pos != p.toks.len() {
        return Err(p.parse_error("unexpected trailing tokens"));
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::{El, parse_el};

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
            El::Switch {
                targets, default, ..
            } => {
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

    #[test]
    fn invalid_el_reports_source_line_column_and_caret() {
        let error = parse_el("THEN(a,\n  WHEN(b, @))").unwrap_err().to_string();
        assert!(error.contains("unexpected character: @"));
        assert!(error.contains("line 2, column 11"));
        assert!(error.contains(" EL:   WHEN(b, @)"));
        assert!(error.lines().last().is_some_and(|line| line.ends_with('^')));
    }

    #[test]
    fn unclosed_string_points_to_opening_quote() {
        let error = parse_el("THEN(a, \"missing)").unwrap_err().to_string();
        assert!(error.contains("unclosed string literal"));
        assert!(error.contains("line 1, column 9"));
        assert!(error.contains(" EL: THEN(a, \"missing)"));
    }
}
