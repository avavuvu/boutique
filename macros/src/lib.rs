mod attrs;
mod component;
mod crate_path;
mod ts;

use proc_macro::TokenStream;

#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    bq_html::expand(input.into(), crate_path::crate_path()).into()
}

#[proc_macro]
pub fn attrs(input: TokenStream) -> TokenStream {
    attrs::expand(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Exports a struct or enum to `bindings/types/<Name>.ts` with ts-rs.
///
/// Adds `#[derive(TS)]` and `#[ts(export, export_to = "types/")]`, plus `optional_fields = nullable` on structs.
/// If the item derives `Serialize` or `Deserialize` and has no `#[serde(rename_all = ...)]`, adds
/// `#[serde(rename_all = "camelCase")]`; write your own `rename_all` to keep another casing.
#[proc_macro_attribute]
pub fn ts(attr: TokenStream, item: TokenStream) -> TokenStream {
    ts::expand(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Turns a function that returns `Markup` into a component.
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    component::expand(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
