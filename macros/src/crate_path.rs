use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn crate_path() -> TokenStream {
    let ident = |name: &str| Ident::new(name, Span::call_site());
    match crate_name("bq_components") {
        Ok(FoundCrate::Itself) => quote!(::bq_components),
        Ok(FoundCrate::Name(name)) => {
            let name = ident(&name);
            quote!(::#name)
        }
        Err(_) => match crate_name("boutique") {
            Ok(FoundCrate::Name(name)) => {
                let name = ident(&name);
                quote!(::#name::components)
            }
            _ => quote!(::bq_components),
        },
    }
}
