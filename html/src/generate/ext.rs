use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Expr, Path, ext::IdentExt, parse_quote, punctuated::Punctuated, spanned::Spanned, token::Comma};

use super::{Builder, Generator, split_attrs};
use crate::{
    ast::*,
    conventions::{ATTRS, CHILDREN, FINISH_FN, maybe_setter, merges},
    escape,
};

impl Generator {
    pub(super) fn merged_attrs(&self, attrs: Vec<Attribute>, build: &mut Builder) {
        let output_ident = &self.output_ident;
        let crate_path = &self.crate_path;
        let attrs_ident = Ident::new("__bq_attrs", Span::mixed_site());
        let stmts = self.build_attrs(&attrs_ident, attrs);
        build.push_tokens(quote!({
            #stmts
            #crate_path::__private::write(&#attrs_ident, &mut #output_ident);
        }));
    }

    pub(super) fn dynamic_element(&self, expr: Expr, attrs: Vec<Attribute>, body: ElementBody, build: &mut Builder) {
        let output_ident = &self.output_ident;
        let tag = Ident::new("__bq_tag", Span::mixed_site());
        let push_tag = quote!(#output_ident.push_str(#tag););

        let mut inner = self.builder();
        inner.push_str("<");
        inner.push_tokens(push_tag.clone());
        self.attrs(attrs, &mut inner);
        inner.push_str(">");
        if let ElementBody::Block(block) = body {
            self.markups(block.markups, &mut inner);
            inner.push_str("</");
            inner.push_tokens(push_tag);
            inner.push_str(">");
        }
        let inner = inner.finish();
        let crate_path = &self.crate_path;

        build.push_tokens(quote! {{
            let #tag: &'static str = #crate_path::__private::tag(#expr);
            #inner
        }});
    }

    pub(super) fn component(
        &self,
        path: Path,
        args: Punctuated<ComponentArg, Comma>,
        attrs: Vec<Attribute>,
        body: ElementBody,
        build: &mut Builder,
    ) {
        let output_ident = &self.output_ident;
        let crate_path = &self.crate_path;
        let span = path.span();
        let function = component_function(&path);

        let mut positional = Vec::new();
        let mut named = Vec::new();
        for arg in args {
            match arg {
                ComponentArg::Positional(expr) => positional.push(expr),
                ComponentArg::Named {
                    name, maybe: None, value, ..
                } => named.push(quote!(.#name(#value))),
                ComponentArg::Named {
                    name,
                    maybe: Some(_),
                    value,
                    ..
                } => {
                    let setter = Ident::new(&maybe_setter(&name.unraw().to_string()), name.span());
                    named.push(quote!(.#setter(#value)));
                }
            }
        }

        let attrs_ident = Ident::new("__bq_attrs", Span::mixed_site());
        let (attrs_stmts, attrs_setter) = if attrs.is_empty() {
            (TokenStream::new(), TokenStream::new())
        } else {
            let stmts = self.build_attrs(&attrs_ident, attrs);
            let attrs_method = Ident::new(ATTRS, span);
            (stmts, quote!(.#attrs_method(#attrs_ident)))
        };

        let children_setter = match body {
            ElementBody::Void(_) => TokenStream::new(),
            ElementBody::Block(block) => {
                let children = Ident::new("__bq_children", Span::mixed_site());
                let mut inner = Builder::new(children.clone());
                Generator::new(children.clone(), crate_path.clone()).markups(block.markups, &mut inner);
                let inner = inner.finish();
                let children_method = Ident::new(CHILDREN, span);
                quote! {
                    .#children_method({
                        let mut #children = alloc::string::String::new();
                        #inner
                        #crate_path::maud::PreEscaped(#children)
                    })
                }
            }
        };

        let build_method = Ident::new(FINISH_FN, span);
        build.push_tokens(quote! {{
            #attrs_stmts
            #crate_path::maud::macro_private::render_to!(
                &(#function(#(#positional),*) #(#named)* #attrs_setter #children_setter .#build_method()),
                &mut #output_ident
            );
        }});
    }

    fn build_attrs(&self, attrs_ident: &Ident, attrs: Vec<Attribute>) -> TokenStream {
        let crate_path = &self.crate_path;
        let (classes, id, named_attrs, spreads) = split_attrs(attrs);
        let mut stmts = vec![quote!(let mut #attrs_ident = #crate_path::Attrs::new();)];
        let mut skip = Vec::new();
        let private = quote!(#crate_path::__private);
        let text =
            |name: &str, value: TokenStream| quote!(#private::text(&mut #attrs_ident, #name, #crate_path::maud::PreEscaped(#value)););
        let class = |value: TokenStream| quote!(#private::class(&mut #attrs_ident, #crate_path::maud::PreEscaped(#value)););
        let flag = |name: &str| quote!(#private::flag(&mut #attrs_ident, #name););

        if let Some(name) = id {
            skip.push("id".to_string());
            stmts.push(match name {
                HtmlNameOrMarkup::HtmlName(name) => {
                    let escaped = escaped(&name.to_string());
                    text("id", quote!(alloc::string::String::from(#escaped)))
                }
                HtmlNameOrMarkup::Markup(markup) => rendered(
                    crate_path,
                    |generator, build| generator.markup(markup, build),
                    |part| text("id", quote!(#part)),
                ),
            });
        }

        for (name, toggler) in classes {
            let push = match name {
                HtmlNameOrMarkup::HtmlName(name) => {
                    let escaped = escaped(&name.to_string());
                    class(quote!(#escaped))
                }
                HtmlNameOrMarkup::Markup(markup) => rendered(
                    crate_path,
                    |generator, build| generator.markup(markup, build),
                    |part| class(quote!(&#part)),
                ),
            };
            stmts.push(match toggler {
                Some(cond) => quote!(if (#cond) { #push }),
                None => push,
            });
        }

        for (name, attr_type) in named_attrs {
            let name = name.to_string();
            if name != "class" && !merges(&name) {
                skip.push(name.clone());
            }
            stmts.push(match attr_type {
                AttributeType::Normal { value, .. } => rendered(
                    crate_path,
                    |generator, build| generator.markup(value, build),
                    |part| text(&name, quote!(#part)),
                ),
                AttributeType::Optional {
                    toggler: Toggler { cond, .. },
                    ..
                } => {
                    let inner_value: Expr = parse_quote!(inner_value);
                    let push = rendered(
                        crate_path,
                        |generator, build| generator.splice(inner_value.clone(), build),
                        |part| text(&name, quote!(#part)),
                    );
                    quote!(if let Some(#inner_value) = (#cond) { #push })
                }
                AttributeType::Empty(None) => flag(&name),
                AttributeType::Empty(Some(Toggler { cond, .. })) => {
                    let flag = flag(&name);
                    quote!(if (#cond) { #flag })
                }
            });
        }

        for expr in spreads {
            stmts.push(quote!(#private::extend(&mut #attrs_ident, &(#expr), &[#(#skip),*]);));
        }

        quote!(#(#stmts)*)
    }
}

pub(super) fn needs_attrs(attrs: &[Attribute]) -> bool {
    let mut names = Vec::new();
    let mut dotted = false;
    for attr in attrs {
        match attr {
            Attribute::Spread { .. } => return true,
            Attribute::Class { .. } => dotted = true,
            Attribute::Id { .. } => names.push("id".to_string()),
            Attribute::Named { name, .. } => names.push(name.to_string()),
        }
    }
    let repeated = names.iter().enumerate().any(|(index, name)| names[..index].contains(name));
    repeated || (dotted && names.iter().any(|name| name == "class"))
}

fn escaped(text: &str) -> String {
    let mut escaped = String::new();
    escape::escape_to_string(text, &mut escaped);
    escaped
}

fn rendered(
    crate_path: &TokenStream,
    write: impl FnOnce(&Generator, &mut Builder),
    then: impl FnOnce(&Ident) -> TokenStream,
) -> TokenStream {
    let part = Ident::new("__bq_part", Span::mixed_site());
    let mut build = Builder::new(part.clone());
    write(&Generator::new(part.clone(), crate_path.clone()), &mut build);
    let body = build.finish();
    let then = then(&part);
    quote! {{
        let mut #part = alloc::string::String::new();
        #body
        #then
    }}
}

fn component_function(path: &Path) -> Path {
    let mut path = path.clone();
    if let Some(last) = path.segments.last_mut() {
        let name = last.ident.unraw().to_string();
        last.ident = Ident::new(&name, Span::mixed_site().located_at(last.ident.span()));
    }
    path
}
