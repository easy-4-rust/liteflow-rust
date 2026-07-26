//! 三元值对象。
//!
//! 对应 Java: `com.yomahub.liteflow.util.TupleOf3`。

/// 保存三个可独立读取和修改的值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleOf3<A, B, C> {
    a: A,
    b: B,
    c: C,
}

impl<A, B, C> TupleOf3<A, B, C> {
    /// 创建三元值对象。
    pub fn new(a: A, b: B, c: C) -> Self {
        Self { a, b, c }
    }

    /// 返回第一个值的引用。对应 Java: `getA`。
    pub fn a(&self) -> &A {
        &self.a
    }

    /// 返回第二个值的引用。对应 Java: `getB`。
    pub fn b(&self) -> &B {
        &self.b
    }

    /// 返回第三个值的引用。对应 Java: `getC`。
    pub fn c(&self) -> &C {
        &self.c
    }

    /// 设置第一个值。对应 Java: `setA`。
    pub fn set_a(&mut self, a: A) {
        self.a = a;
    }

    /// 设置第二个值。对应 Java: `setB`。
    pub fn set_b(&mut self, b: B) {
        self.b = b;
    }

    /// 设置第三个值。对应 Java: `setC`。
    pub fn set_c(&mut self, c: C) {
        self.c = c;
    }
}
