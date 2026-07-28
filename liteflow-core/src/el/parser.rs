//! LiteFlow EL 的 QLExpress 解析入口与链式扩展分派。
//!
//! Java v2.16.0 由 `QlExpressUtils` 注册主函数和扩展函数；Rust 同样把词法、
//! 语法、编译和 QVM 交给发布版 QlExpress Rust，本文件仅把 QVM 动态调用分派
//! 到一对象一文件的强类型 Operator。

use super::{Arg, El};
use crate::builder::el::operator::base::BaseOperator;
use crate::builder::el::operator::{
    AnyOperator, BindOperator, BreakOperator, DataOperator, DefaultOperator, DoOperator,
    ElifOperator, ElseOperator, IdOperator, IgnoreErrorOperator, MaxWaitMillisecondsOperator,
    MaxWaitSecondsOperator, MustOperator, ParallelOperator, PercentageOperator, RetryOperator,
    TagOperator, ThreadPoolOperator, ToOperator,
};
use crate::exception::{LFResult, LiteflowError};

/// 把 Java Builder 的 camelCase 方法名和文档中的 snake_case 写法统一为
/// Operator 使用的 SCREAMING_SNAKE_CASE 名称。
pub(crate) fn normalize_method_name(name: &str) -> String {
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

/// 把 QLExpress 产生的链式方法调用应用到强类型 EL。
///
/// 参数 `expression` 是方法接收者，`name` 与 `arguments` 对应 Java
/// `addExtendFunction` 传给 Operator 的对象数组。返回构建后的表达式。
pub(crate) fn apply_el_method(expression: El, name: &str, arguments: Vec<Arg>) -> LFResult<El> {
    let operator = normalize_method_name(name);
    match operator.as_str() {
        "ELSE" => ElseOperator.build(Some(expression), arguments),
        "ELIF" => ElifOperator.build(Some(expression), arguments),
        "TO" => ToOperator.build(Some(expression), arguments),
        "DEFAULT" => DefaultOperator.build(Some(expression), arguments),
        "DO" => DoOperator.build(Some(expression), arguments),
        "BREAK" => BreakOperator.build(Some(expression), arguments),
        "PARALLEL" => ParallelOperator.build(Some(expression), arguments),
        "THREAD_POOL" => ThreadPoolOperator.build(Some(expression), arguments),
        "ID" => IdOperator.build(Some(expression), arguments),
        "TAG" => TagOperator.build(Some(expression), arguments),
        "DATA" => DataOperator.build(Some(expression), arguments),
        "BIND" => BindOperator.build(Some(expression), arguments),
        "RETRY" => RetryOperator.build(Some(expression), arguments),
        "IGNORE_ERROR" => IgnoreErrorOperator.build(Some(expression), arguments),
        "ANY" => AnyOperator.build(Some(expression), arguments),
        "PERCENTAGE" => PercentageOperator.build(Some(expression), arguments),
        "MUST" => MustOperator.build(Some(expression), arguments),
        "MAX_WAIT_SECONDS" => MaxWaitSecondsOperator.build(Some(expression), arguments),
        "MAX_WAIT_MILLISECONDS" | "MAX_WAIT_TIME" => {
            MaxWaitMillisecondsOperator.build(Some(expression), arguments)
        }
        _ => Err(LiteflowError::Parse(format!("unknown method: {name}"))),
    }
}

/// 把无括号成员访问转换为声明式组件方法引用。
///
/// 参数 `expression` 必须是 Node，`name` 为 Java `cmpId.methodName` 中的方法名。
pub(crate) fn apply_el_method_ref(expression: El, name: &str) -> LFResult<El> {
    // 已知扩展操作符必须带括号，不能误当成声明式组件方法。
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
    match expression {
        El::Node(mut node) => {
            node.id = format!("{}.{}", node.id, name);
            Ok(El::Node(node))
        }
        _ => Err(LiteflowError::Parse(format!(
            "method reference must follow a node: .{name}"
        ))),
    }
}

/// 通过真实 QLExpress Runner 解析 EL 文本。
///
/// 参数 `text` 对应 Java `Express4Runner#execute` 的脚本文本；返回 QVM 构建出的
/// 强类型表达式树。对应 Java: `QlExpressUtils#getELExpressRunner` 调用链。
pub fn parse_el(text: &str) -> LFResult<El> {
    crate::util::QlExpressUtils::parse_el(text)
}

#[cfg(test)]
mod tests {
    use super::{El, parse_el};

    #[test]
    fn parse_then_when() {
        let expression = parse_el("THEN(a, b, WHEN(c, d, e), f)").unwrap();
        match expression {
            El::Then(items) => {
                assert_eq!(items.len(), 4);
                assert!(matches!(&items[2], El::When { items, .. } if items.len() == 3));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_if_elif_else() {
        let expression = parse_el("IF(x, a).ELIF(y, b).ELSE(c)").unwrap();
        assert!(matches!(
            expression,
            El::If {
                ref elifs,
                els: Some(_),
                ..
            } if elifs.len() == 1
        ));
    }

    #[test]
    fn parse_switch_to_default() {
        let expression = parse_el(r#"SWITCH(s).TO(a, b, "c:tag1").DEFAULT(d)"#).unwrap();
        match expression {
            El::Switch {
                targets, default, ..
            } => {
                assert_eq!(targets.len(), 3);
                assert!(
                    matches!(&targets[2], El::Node(node) if node.id == "c" && node.tag.as_deref() == Some("tag1"))
                );
                assert!(default.is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_for_do_break() {
        let expression = parse_el("FOR(f).PARALLEL(2).DO(THEN(a, b)).BREAK(x)").unwrap();
        assert!(matches!(
            expression,
            El::For {
                parallel: Some(2),
                brk: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parse_modifiers() {
        let expression = parse_el("a.tag(\"t1\").data(\"{'k':1}\").retry(3)").unwrap();
        match expression {
            El::Mods(inner, modifiers) => {
                assert_eq!(modifiers.retry, Some(3));
                assert!(
                    matches!(*inner, El::Node(node) if node.tag.as_deref() == Some("t1") && node.data.is_some())
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_when_opts() {
        let expression = parse_el("WHEN(a, b, c).ANY(true).MAX_WAIT_SECONDS(2)").unwrap();
        assert!(matches!(
            expression,
            El::When { opts, .. } if opts.any && opts.max_wait_ms == Some(2_000)
        ));
    }

    #[test]
    fn parse_catch_do() {
        assert!(matches!(
            parse_el("THEN(CATCH(THEN(a, b)).DO(handle), d)").unwrap(),
            El::Then(_)
        ));
    }

    #[test]
    fn parse_pre_finally() {
        assert!(matches!(
            parse_el("THEN(PRE(p), a, b, FINALLY(z))").unwrap(),
            El::Then(_)
        ));
    }

    #[test]
    fn parse_and_or_not() {
        assert!(matches!(
            parse_el("IF(AND(x, OR(y, NOT(z))), a, b)").unwrap(),
            El::If { .. }
        ));
    }

    #[test]
    fn invalid_el_preserves_qlexpress_diagnostic_and_source() {
        let error = parse_el("THEN(a,\n  WHEN(b, c)").unwrap_err().to_string();
        assert!(error.contains("mismatched"), "{error}");
        assert!(error.contains("EL: THEN(a,"), "{error}");
    }

    #[test]
    fn unclosed_el_preserves_qlexpress_diagnostic() {
        let error = parse_el("THEN(a, WHEN(b, c)").unwrap_err().to_string();
        assert!(error.contains("mismatched"), "{error}");
        assert!(error.contains("EL: THEN(a,"), "{error}");
    }
}
