//! 二元值对象。
//!
//! 对应 Java: `com.yomahub.liteflow.util.TupleOf2`。

/// 保存两个可独立读取和修改的值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleOf2<A, B> {
    a: A,
    b: B,
}

impl<A, B> TupleOf2<A, B> {
    /// 创建二元值对象。
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }

    /// 返回第一个值的引用。对应 Java: `getA`。
    pub fn a(&self) -> &A {
        &self.a
    }

    /// 返回第二个值的引用。对应 Java: `getB`。
    pub fn b(&self) -> &B {
        &self.b
    }

    /// 设置第一个值。对应 Java: `setA`。
    pub fn set_a(&mut self, a: A) {
        self.a = a;
    }

    /// 设置第二个值。对应 Java: `setB`。
    pub fn set_b(&mut self, b: B) {
        self.b = b;
    }
}
