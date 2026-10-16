use std::collections::HashSet;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    Expr, LitStr, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
};

use crate::crate_path::crate_path;

struct AttributePair {
    name: String,
    span: Span,
    value: Option<Expr>,
}

impl Parse for AttributePair {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (name, span) = if input.peek(LitStr) {
            let literal: LitStr = input.parse()?;
            (literal.value(), literal.span())
        } else {
            let ident = input.call(Ident::parse_any)?;
            (ident.unraw().to_string().replace('_', "-"), ident.span())
        };

        let value = if input.parse::<Option<Token![=]>>()?.is_some() {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { name, span, value })
    }
}

fn valid_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control() && !"\"'<>/=`".contains(character))
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let pairs = Punctuated::<AttributePair, Token![,]>::parse_terminated.parse2(input)?;
    let attrs = Ident::new("__bq_attrs", Span::mixed_site());
    let buffer = Ident::new("__bq_value", Span::mixed_site());
    let crate_path = crate_path();

    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for pair in pairs {
        if !valid_attribute_name(&pair.name) {
            let message = format!("{:?} is not a valid attribute name", pair.name);
            return Err(syn::Error::new(pair.span, message));
        }
        if pair.name != "class" && !seen.insert(pair.name.clone()) {
            let message = format!("the attribute {:?} is given two times", pair.name);
            return Err(syn::Error::new(pair.span, message));
        }
        let name = &pair.name;
        items.push(match &pair.value {
            Some(value) => quote! {{
                let mut #buffer = ::std::string::String::new();
                #crate_path::maud::macro_private::render_to!(&(#value), &mut #buffer);
                #crate_path::__private::text(&mut #attrs, #name, #crate_path::maud::PreEscaped(#buffer));
            }},
            None => quote!(#crate_path::__private::flag(&mut #attrs, #name);),
        });
    }

    Ok(quote! {{
        let mut #attrs = #crate_path::Attrs::new();
        #(#items)*
        #attrs
    }})
}
