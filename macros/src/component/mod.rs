mod docs;
mod params;
mod render;
mod setup;

use bq_html::conventions::{ComponentOptions, component_alias};
use proc_macro2::TokenStream;
use syn::{Error, ItemFn, ReturnType, Signature};

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let options: ComponentOptions = syn::parse2(attr)?;
    let mut func: ItemFn = syn::parse2(item)?;
    check_return_type(&func.sig)?;
    let params = params::take(&mut func.sig)?;
    setup::attach(&mut func, &options)?;
    let alias = component_alias(&func.sig.ident);
    docs::attach(&mut func, &alias, &params);
    Ok(render::expand(func, &alias))
}

fn check_return_type(sig: &Signature) -> syn::Result<()> {
    match &sig.output {
        ReturnType::Type(_, ty) if params::is_type(ty, "Markup") => Ok(()),
        ReturnType::Type(_, ty) => Err(Error::new_spanned(ty, "a #[component] must return `Markup`")),
        ReturnType::Default => Err(Error::new_spanned(
            &sig.ident,
            "a #[component] must return `Markup`; add `-> Markup`",
        )),
    }
}
