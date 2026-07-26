use proc_macro::TokenStream;

/// 属性别名标记。
///
/// Java AliasFor 依赖运行期注解合并；Rust 宏参数在编译期已归一化，因此该
/// 标记保持被修饰项不变，供组合宏表达同一意图。
/// 对应 Java: `com.yomahub.liteflow.annotation.AliasFor`。
pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
