use proc_macro::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{ExprArray, ItemImpl, LitInt, Result, Token, parse_macro_input};

/// 组件重试策略宏。
///
/// 对应 Java: `com.yomahub.liteflow.annotation.LiteflowRetry`。
struct RetryArgs {
    count: LitInt,
    retry_for: Vec<syn::LitStr>,
}

impl Parse for RetryArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let count: LitInt = input.parse()?;
        let mut retry_for = Vec::new();
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
            // `for` 是 Rust 关键字；parse_any 允许宏参数沿用 Java 注解的字段名。
            let key = syn::Ident::parse_any(input)?;
            if key != "for" {
                return Err(syn::Error::new(key.span(), "仅支持 for = [\"Error\"]"));
            }
            input.parse::<Token![=]>()?;
            let values: ExprArray = input.parse()?;
            for value in values.elems {
                match value {
                    syn::Expr::Lit(expr) => match expr.lit {
                        syn::Lit::Str(value) => retry_for.push(value),
                        _ => {
                            return Err(syn::Error::new_spanned(expr, "for 数组只能包含字符串"));
                        }
                    },
                    other => {
                        return Err(syn::Error::new_spanned(other, "for 数组只能包含字符串"));
                    }
                }
            }
        }
        Ok(Self { count, retry_for })
    }
}

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as RetryArgs);
    let mut input = parse_macro_input!(item as ItemImpl);
    if input.trait_.is_none() {
        return syn::Error::new_spanned(
            input,
            "liteflow_retry 必须标注在 NodeComponent trait impl 上",
        )
        .to_compile_error()
        .into();
    }
    let count = args.count;
    let retry_for = args.retry_for;
    let has_retry_count = input
        .items
        .iter()
        .any(|item| matches!(item, syn::ImplItem::Fn(method) if method.sig.ident == "retry_count"));
    let has_retry_for = input.items.iter().any(
        |item| matches!(item, syn::ImplItem::Fn(method) if method.sig.ident == "is_retry_for"),
    );
    if has_retry_count || has_retry_for {
        return syn::Error::new_spanned(
            input,
            "liteflow_retry 会生成 retry_count/is_retry_for，请删除手写实现",
        )
        .to_compile_error()
        .into();
    }
    input.items.push(syn::parse_quote! {
        fn retry_count(&self) -> usize {
            #count
        }
    });
    input.items.push(syn::parse_quote! {
        fn is_retry_for(&self, error: &::liteflow_core::LiteflowError) -> bool {
            // Node::execute_once 会把业务错误包装成 NodeExec，并保留原始变体名。
            // 对应 Java `LiteflowRetry#forExceptions` 对原始异常类型的判断。
            let kind = match error {
                ::liteflow_core::LiteflowError::NodeExec { kind, .. } => {
                    kind.trim_end_matches("Exception").to_owned()
                }
                _ => format!("{error:?}")
                    .split([' ', '(', '{'])
                    .next()
                    .unwrap_or_default()
                    .trim_end_matches("Exception")
                    .to_owned(),
            };
            let filters: &[&str] = &[#(#retry_for),*];
            filters.is_empty()
                || filters.iter().any(|name| {
                    let simple = name.rsplit('.').next().unwrap_or(name);
                    simple.trim_end_matches("Exception") == kind.as_str()
                        || simple == "Error"
                        || simple == "LiteflowError"
                })
        }
    });
    quote!(#input).into()
}
