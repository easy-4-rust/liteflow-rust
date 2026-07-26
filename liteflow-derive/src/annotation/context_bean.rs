use proc_macro::TokenStream;

use quote::quote;
use syn::{ItemStruct, LitStr, parse_macro_input};

/// 上下文 Bean 别名注解。
///
/// 用法：`#[context_bean("orderContext")]`。Java 注解的 `value` 与 `name`
/// 互为镜像；Rust 宏只接收一个别名，在编译期生成稳定常量和类型安全的上下文
/// Bean 元组。对应 Java: `com.yomahub.liteflow.context.ContextBean`。
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let alias = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemStruct);
    if alias.value().trim().is_empty() {
        return syn::Error::new(alias.span(), "context bean alias 不能为空")
            .to_compile_error()
            .into();
    }
    let ident = &input.ident;

    quote! {
        #input

        impl #ident {
            /// 上下文 Bean 的注册与查找别名。
            ///
            /// 对应 Java `ContextBean#value` / `ContextBean#name`。
            pub const LITEFLOW_CONTEXT_NAME: &'static str = #alias;

            /// 转换为 `FlowExecutor` 可直接接收的上下文 Bean 元组。
            ///
            /// 返回值可传给 `FlowBus#execute_with` 或
            /// `ExecuteOption#context_bean`。
            pub fn into_context_bean(
                self,
            ) -> (
                String,
                ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>,
            )
            where
                Self: Send + Sync + 'static,
            {
                (
                    Self::LITEFLOW_CONTEXT_NAME.to_string(),
                    ::std::sync::Arc::new(self),
                )
            }
        }
    }
    .into()
}
