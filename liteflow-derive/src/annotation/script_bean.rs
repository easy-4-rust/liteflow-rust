//! 对应 Java: com.yomahub.liteflow.script.annotation.ScriptBean

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemStruct, LitStr, Result, Token, parse_macro_input};

/// `ScriptBean` 注解参数。
///
/// Rust 采用 `#[script_bean("name", include = "a,b", exclude = "c")]`，
/// 逗号分隔值对应 Java 注解的字符串数组。
struct ScriptBeanArgs {
    name: LitStr,
    include: Option<LitStr>,
    exclude: Option<LitStr>,
}

impl Parse for ScriptBeanArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        let mut include = None;
        let mut exclude = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            match key.to_string().as_str() {
                "include" => include = Some(value),
                "exclude" => exclude = Some(value),
                _ => return Err(syn::Error::new(key.span(), "仅支持 include 或 exclude")),
            }
        }
        Ok(Self {
            name,
            include,
            exclude,
        })
    }
}

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ScriptBeanArgs);
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let name = args.name;
    let includes = args
        .include
        .map(|value| value.value())
        .unwrap_or_default()
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(|value| LitStr::new(value.trim(), name.span()))
        .collect::<Vec<_>>();
    let excludes = args
        .exclude
        .map(|value| value.value())
        .unwrap_or_default()
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(|value| LitStr::new(value.trim(), name.span()))
        .collect::<Vec<_>>();

    quote! {
        #input

        impl #ident {
            /// 注解声明的脚本 Bean 名称。
            pub const LITEFLOW_SCRIPT_BEAN_NAME: &'static str = #name;

            /// 根据注解包含/排除规则构建受控代理并注册。
            pub fn register_script_bean(
                methods: ::std::vec::Vec<::liteflow_core::script::proxy::ScriptMethodProxy>,
            ) {
                let proxy = ::liteflow_core::script::proxy::ScriptBeanProxy::new(
                    Self::LITEFLOW_SCRIPT_BEAN_NAME,
                    &[#(#includes),*],
                    &[#(#excludes),*],
                    methods,
                );
                ::liteflow_core::script::ScriptBeanManager::add_script_bean(proxy);
            }
        }
    }
    .into()
}
