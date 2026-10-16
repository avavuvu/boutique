mod attrs;
mod component;
mod crate_path;

use proc_macro::TokenStream;

#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    bq_html::expand(input.into(), crate_path::crate_path()).into()
}

#[proc_macro]
pub fn attrs(input: TokenStream) -> TokenStream {
    attrs::expand(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Turns a function that returns `Markup` into a component.
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    component::expand(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
