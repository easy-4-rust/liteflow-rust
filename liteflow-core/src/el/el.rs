//! LiteFlow EL 抽象语法树。

use super::{Mods, NodeRef, WhenOpts};

/// EL 语法树，对应 `flow.element.condition` 包的 Condition 族。
#[derive(Debug, Clone, PartialEq)]
pub enum El {
    /// 节点引用。
    Node(NodeRef),
    /// WHILE 布尔字面量。
    Boolean(bool),
    /// 串行表达式。
    Then(Vec<El>),
    /// 并行表达式。
    When { items: Vec<El>, opts: WhenOpts },
    /// 条件表达式。
    If {
        cond: Box<El>,
        then: Box<El>,
        elifs: Vec<(El, El)>,
        els: Option<Box<El>>,
    },
    /// 选择表达式。
    Switch {
        node: Box<El>,
        targets: Vec<El>,
        default: Option<Box<El>>,
    },
    /// 动态次数循环。
    For {
        node: Box<El>,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    /// 固定次数循环。
    ForCount {
        count: usize,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    /// WHILE 循环。
    While {
        node: Box<El>,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    /// 迭代循环。
    Iter {
        node: Box<El>,
        parallel: Option<usize>,
        body: Box<El>,
        brk: Option<Box<El>>,
    },
    /// 异常捕获表达式。
    Catch { body: Box<El>, do_: Option<Box<El>> },
    /// 逻辑与。
    And(Vec<El>),
    /// 逻辑或。
    Or(Vec<El>),
    /// 逻辑非。
    Not(Box<El>),
    /// PRE 子流程。
    Pre(Box<El>),
    /// FINALLY 子流程。
    Fin(Box<El>),
    /// 通用修饰。
    Mods(Box<El>, Mods),
}
