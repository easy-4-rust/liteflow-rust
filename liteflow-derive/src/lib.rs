//! LiteFlow 注解体系的 Rust 过程宏实现。
//!
//! 每个 Java annotation 对应一个独立模块；本文件只保留 proc-macro 入口。

extern crate proc_macro;

mod annotation;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn alias_for(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::alias_for::expand(attr, item)
}

#[proc_macro_attribute]
pub fn context_bean(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::context_bean::expand(attr, item)
}

#[proc_macro_attribute]
pub fn fallback_cmp(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::fallback_cmp::expand(attr, item)
}

#[proc_macro_attribute]
pub fn liteflow_cmp_define(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::liteflow_cmp_define::expand(attr, item)
}

#[proc_macro_attribute]
pub fn liteflow_component(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::liteflow_component::expand(attr, item)
}

#[proc_macro_attribute]
pub fn liteflow_fact(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::liteflow_fact::expand(attr, item)
}

#[proc_macro_attribute]
pub fn liteflow_method(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::liteflow_method::expand(attr, item)
}

#[proc_macro_attribute]
pub fn liteflow_retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    annotation::liteflow_retry::expand(attr, item)
}
