use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Attribute, Item, Meta, parse::Parser, parse2, punctuated::Punctuated, spanned::Spanned, token::Comma};

const EXPORT_TO: &str = "types/";

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new(attr.span(), "#[ts] takes no arguments; use #[ts(...)] attributes from ts-rs for anything else"));
    }

    let mut item: Item = parse2(item)?;
    let (attrs, is_struct) = match &mut item {
        Item::Struct(item) => (&mut item.attrs, true),
        Item::Enum(item) => (&mut item.attrs, false),
        other => return Err(syn::Error::new(other.span(), "#[ts] works on structs and enums")),
    };

    let path = ts_rs_path();
    let path_text = path.to_string().replace(' ', "");
    let options = if is_struct {
        quote!(crate = #path_text, export, export_to = #EXPORT_TO, optional_fields = nullable)
    } else {
        quote!(crate = #path_text, export, export_to = #EXPORT_TO)
    };

    let mut added: Vec<Attribute> = vec![syn::parse_quote!(#[derive(#path::TS)]), syn::parse_quote!(#[ts(#options)])];
    added.append(attrs);
    *attrs = added;

    if derives_serde(attrs)? && !renames_all(attrs)? {
        attrs.push(syn::parse_quote!(#[serde(rename_all = "camelCase")]));
    }

    Ok(item.into_token_stream())
}

fn ts_rs_path() -> TokenStream {
    let ident = |name: &str| Ident::new(name, Span::call_site());
    match crate_name("boutique") {
        Ok(FoundCrate::Name(name)) => {
            let name = ident(&name);
            quote!(::#name::ts_rs)
        }
        _ => match crate_name("ts-rs") {
            Ok(FoundCrate::Name(name)) => {
                let name = ident(&name);
                quote!(::#name)
            }
            _ => quote!(::ts_rs),
        },
    }
}

fn derives_serde(attrs: &[Attribute]) -> syn::Result<bool> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("derive")) {
        let paths = attr.parse_args_with(Punctuated::<syn::Path, Comma>::parse_terminated)?;
        if paths
            .iter()
            .any(|path| path.segments.last().is_some_and(|segment| segment.ident == "Serialize" || segment.ident == "Deserialize"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn renames_all(attrs: &[Attribute]) -> syn::Result<bool> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        let Meta::List(list) = &attr.meta else { continue };
        let metas = Punctuated::<Meta, Comma>::parse_terminated.parse2(list.tokens.clone())?;
        if metas.iter().any(|meta| meta.path().is_ident("rename_all")) {
            return Ok(true);
        }
    }
    Ok(false)
}
