//! 对应 Java: com.yomahub.liteflow.script.annotation.ScriptMethod

use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{ImplItemFn, LitStr, parse_macro_input};

use super::anno_util::AnnoUtil;

/// 为方法生成稳定的脚本暴露名元数据。
///
/// 外层 `script_bean` 使用该名称构造 `ScriptMethodProxy`；空参数时沿用
/// Rust 方法名，对应 Java `@ScriptMethod` 的默认值。
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let method = parse_macro_input!(item as ImplItemFn);
    let rust_name = method.sig.ident.to_string();
    let exposed_name_value = if attr.is_empty() {
        rust_name.clone()
    } else {
        parse_macro_input!(attr as LitStr).value()
    };
    let cache_key = method.sig.to_token_stream().to_string();
    let exposed_name_value =
        AnnoUtil::get_annotation(&cache_key, "ScriptMethod", || exposed_name_value);
    let exposed_name = LitStr::new(&exposed_name_value, method.sig.ident.span());
    let const_name = format_ident!("LITEFLOW_SCRIPT_METHOD_{}", rust_name.to_ascii_uppercase());
    quote! {
        #method

        /// `ScriptMethod` 注解声明的脚本侧方法名。
        pub const #const_name: &'static str = #exposed_name;
    }
    .into()
}
