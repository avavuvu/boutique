use bq_html::conventions::{ATTRS, CHILDREN, ComponentOptions, FINISH_FN, MAYBE_PREFIX, component_alias};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use syn::{
    Expr, GenericParam, Ident, ItemFn, Lifetime, LitStr, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    visit_mut::VisitMut,
};

struct AttributePair {
    name: String,
    span: proc_macro2::Span,
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

fn crate_path() -> proc_macro2::TokenStream {
    let ident = |name: &str| Ident::new(name, proc_macro2::Span::call_site());
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

#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    bq_html::expand(input.into(), crate_path()).into()
}

#[proc_macro]
pub fn attrs(input: TokenStream) -> TokenStream {
    let pairs = parse_macro_input!(input with Punctuated::<AttributePair, Token![,]>::parse_terminated);
    let attrs = proc_macro2::Ident::new("__bq_attrs", proc_macro2::Span::mixed_site());
    let buffer = proc_macro2::Ident::new("__bq_value", proc_macro2::Span::mixed_site());
    let crate_path = crate_path();

    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    for pair in pairs {
        if !valid_attribute_name(&pair.name) {
            let message = format!("{:?} is not a valid attribute name", pair.name);
            return syn::Error::new(pair.span, message).to_compile_error().into();
        }
        if pair.name != "class" && !seen.insert(pair.name.clone()) {
            let message = format!("the attribute {:?} is given two times", pair.name);
            return syn::Error::new(pair.span, message).to_compile_error().into();
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

    quote! {{
        let mut #attrs = #crate_path::Attrs::new();
        #(#items)*
        #attrs
    }}
    .into()
}

/// Turns a function that returns `Markup` into a component.
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as ComponentOptions);
    let mut func = parse_macro_input!(item as ItemFn);
    if let Err(error) = translate_params(&mut func) {
        return error.to_compile_error().into();
    }
    if let Err(error) = attach_setup(&mut func, &options) {
        return error.to_compile_error().into();
    }
    let crate_path = crate_path();

    let pascal = component_alias(&func.sig.ident);
    let docs = component_docs(&pascal, &func);
    if func.attrs.iter().any(is_doc) {
        func.attrs.push(syn::parse_quote!(#[doc = ""]));
    }
    func.attrs
        .extend(docs.iter().map(|line| -> syn::Attribute { syn::parse_quote!(#[doc = #line]) }));

    let fn_name = &func.sig.ident;
    let visibility = &func.vis;
    let builder = format_ident!("{pascal}Builder");
    let state_mod = format_ident!("{fn_name}_builder");
    let finish_fn = Ident::new(FINISH_FN, proc_macro2::Span::call_site());
    let alias = (fn_name.unraw() != pascal).then(|| {
        let alias = Ident::new(&pascal, fn_name.span());
        quote!(#[allow(unused_imports)] #visibility use #fn_name as #alias;)
    });

    name_elided_lifetimes(&mut func.sig);
    let mut generics = func.sig.generics.clone();
    generics.params.push(syn::parse_quote!(__S: #state_mod::IsComplete));
    let type_generics = {
        let (_, type_generics, _) = generics.split_for_impl();
        quote!(#type_generics)
    };
    generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote!(#builder #type_generics: ::core::clone::Clone));
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        #[#crate_path::bon::builder(
            crate = #crate_path::bon,
            builder_type(name = #builder, doc { #(#[doc = #docs])* }),
            state_mod = #state_mod,
            finish_fn = #finish_fn,
            derive(Clone)
        )]
        #func

        #alias

        impl #impl_generics #crate_path::maud::Render for #builder #type_generics #where_clause {
            fn render(&self) -> #crate_path::maud::Markup {
                ::core::clone::Clone::clone(self).#finish_fn()
            }
        }
    };

    expanded.into()
}

fn attach_setup(func: &mut ItemFn, options: &ComponentOptions) -> syn::Result<()> {
    if !bq_html::setup::has_bare_setup(&func.block) {
        let option = options
            .eager
            .as_ref()
            .map(Ident::span)
            .or_else(|| options.setup.as_ref().map(LitStr::span));
        if let Some(span) = option {
            return Err(syn::Error::new(
                span,
                format!(
                    "`{}` has no bare `bq-setup`, so it has no setup to configure; add `bq-setup` to the element its setup attaches to",
                    func.sig.ident.unraw()
                ),
            ));
        }
    }
    bq_html::setup::mark(&mut func.block, &options.name(&func.sig.ident));
    Ok(())
}

fn is_type(ty: &syn::Type, name: &str) -> bool {
    let syn::Type::Path(path) = ty else { return false };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name && segment.arguments.is_none())
}

fn translate_params(func: &mut ItemFn) -> syn::Result<()> {
    for input in func.sig.inputs.iter_mut() {
        let syn::FnArg::Typed(typed) = input else { continue };
        let raw = builder_options(&typed.attrs);
        let mut defaulted = raw.default.is_some();
        let mut fixed = raw.start_fn || raw.fixed;
        let mut translated: Vec<syn::Attribute> = Vec::new();
        let mut kept = Vec::new();

        for attr in std::mem::take(&mut typed.attrs) {
            if attr.path().is_ident("default") {
                if defaulted {
                    return Err(syn::Error::new_spanned(&attr, "this parameter already has a default"));
                }
                defaulted = true;
                translated.push(match &attr.meta {
                    syn::Meta::Path(_) => syn::parse_quote!(#[builder(default)]),
                    syn::Meta::NameValue(pair) => {
                        let value = &pair.value;
                        syn::parse_quote!(#[builder(default = #value)])
                    }
                    syn::Meta::List(_) => {
                        return Err(syn::Error::new_spanned(&attr, "expected `#[default]` or `#[default = …]`"));
                    }
                });
            } else if attr.path().is_ident("positional") {
                if !matches!(attr.meta, syn::Meta::Path(_)) {
                    return Err(syn::Error::new_spanned(&attr, "`#[positional]` takes no arguments"));
                }
                fixed = true;
                translated.push(syn::parse_quote!(#[builder(start_fn)]));
            } else {
                kept.push(attr);
            }
        }

        if !defaulted && !fixed {
            let name = match typed.pat.as_ref() {
                syn::Pat::Ident(pattern) => pattern.ident.unraw().to_string(),
                _ => String::new(),
            };
            if is_type(&typed.ty, "bool") {
                translated.push(syn::parse_quote!(#[builder(default = false)]));
            } else if (name == ATTRS && is_type(&typed.ty, "Attrs")) || name == CHILDREN {
                translated.push(syn::parse_quote!(#[builder(default)]));
            }
        }

        kept.extend(translated);
        typed.attrs = kept;
    }
    Ok(())
}

#[derive(PartialEq)]
enum Role {
    Positional,
    Named,
    Children,
    Attrs,
}

struct Prop {
    name: String,
    ty: String,
    role: Role,
    kind: String,
    required: bool,
    maybe: bool,
    docs: String,
}

#[derive(Default)]
struct BuilderOptions {
    start_fn: bool,
    fixed: bool,
    required: bool,
    into: bool,
    default: Option<Option<String>>,
}

fn is_doc(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("doc")
}

fn doc_text(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| is_doc(attr))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(syn::MetaNameValue {
                value: Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text), ..
                }),
                ..
            }) => Some(text.value().trim().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn builder_options(attrs: &[syn::Attribute]) -> BuilderOptions {
    let mut options = BuilderOptions::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("builder")) {
        let _ = attr.parse_nested_meta(|meta| {
            let value = if meta.input.peek(Token![=]) {
                Some(meta.value()?.parse::<Expr>()?)
            } else {
                if !meta.input.is_empty() && !meta.input.peek(Token![,]) {
                    meta.input.parse::<proc_macro2::TokenTree>()?;
                }
                None
            };

            if meta.path.is_ident("start_fn") {
                options.start_fn = true;
            } else if meta.path.is_ident("finish_fn") || meta.path.is_ident("skip") || meta.path.is_ident("field") {
                options.fixed = true;
            } else if meta.path.is_ident("required") {
                options.required = true;
            } else if meta.path.is_ident("into") {
                options.into = true;
            } else if meta.path.is_ident("default") {
                options.default = Some(value.map(|value| tidy(&quote!(#value).to_string())));
            }
            Ok(())
        });
    }
    options
}

fn option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(path) = ty else { return None };
    let segment = path
        .path
        .segments
        .last()
        .filter(|segment| path.qself.is_none() && segment.ident == "Option")?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match arguments.args.first() {
        Some(syn::GenericArgument::Type(inner)) => Some(inner),
        _ => None,
    }
}

fn tidy(tokens: &str) -> String {
    [
        (" < ", "<"),
        ("< ", "<"),
        (" >", ">"),
        ("& ", "&"),
        (" ,", ","),
        (" :: ", "::"),
        (":: ", "::"),
        (" !", "!"),
        ("! ", "!"),
    ]
    .iter()
    .fold(tokens.to_string(), |text, (from, to)| text.replace(from, to))
}

fn props(func: &ItemFn) -> Vec<Prop> {
    func.sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(typed) => Some(typed),
            syn::FnArg::Receiver(_) => None,
        })
        .filter_map(|typed| {
            let syn::Pat::Ident(pattern) = typed.pat.as_ref() else {
                return None;
            };
            let name = pattern.ident.unraw().to_string();
            let options = builder_options(&typed.attrs);
            let optional_option = !options.required && option_inner(&typed.ty).is_some();

            let role = match name.as_str() {
                _ if options.start_fn => Role::Positional,
                CHILDREN => Role::Children,
                ATTRS => Role::Attrs,
                _ => Role::Named,
            };

            let setter_type = if optional_option { option_inner(&typed.ty) } else { None }.unwrap_or(&typed.ty);
            let setter_type = tidy(&quote!(#setter_type).to_string());
            let ty = if options.into {
                format!("impl Into<{setter_type}>")
            } else {
                setter_type
            };

            let required = options.default.is_none() && !optional_option;
            let kind = match (&role, &options.default) {
                (Role::Positional, _) => "positional".to_string(),
                (Role::Attrs, _) => "call-site attributes".to_string(),
                (Role::Children, None) => "block body".to_string(),
                (Role::Children, Some(_)) => "block body, optional".to_string(),
                (Role::Named, Some(Some(default))) => format!("optional, default `{default}`"),
                (Role::Named, Some(None)) => "optional, default".to_string(),
                (Role::Named, None) if optional_option => "optional".to_string(),
                (Role::Named, None) => "required".to_string(),
            };

            Some(Prop {
                maybe: role == Role::Named && !required,
                name,
                ty: ty.replace('|', "\\|"),
                role,
                kind,
                required,
                docs: doc_text(&typed.attrs).replace('|', "\\|"),
            })
        })
        .collect()
}

fn example(pascal: &str, props: &[Prop]) -> Vec<String> {
    let mut arguments: Vec<String> = props
        .iter()
        .filter(|prop| prop.role == Role::Positional)
        .map(|prop| prop.name.clone())
        .collect();
    arguments.extend(
        props
            .iter()
            .filter(|prop| prop.role == Role::Named && prop.required)
            .map(|prop| format!("{} = …", prop.name)),
    );

    let body = if props.iter().any(|prop| prop.role == Role::Children) {
        " { … }"
    } else {
        ";"
    };

    match arguments.len() {
        0 => vec![format!("{pascal}{body}")],
        1..=2 => vec![format!("{pascal}({}){body}", arguments.join(", "))],
        _ => std::iter::once(format!("{pascal}("))
            .chain(arguments.iter().map(|argument| format!("    {argument},")))
            .chain(std::iter::once(format!("){body}")))
            .collect(),
    }
}

fn component_docs(pascal: &str, func: &ItemFn) -> Vec<String> {
    let props = props(func);
    let mut lines = vec![" # Example".to_string(), " ```rust,ignore".to_string()];
    lines.extend(example(pascal, &props).into_iter().map(|line| format!(" {line}")));
    lines.push(" ```".to_string());

    if props.is_empty() {
        return lines;
    }

    let described = props.iter().any(|prop| !prop.docs.is_empty());
    lines.push(String::new());
    lines.push(" # Props".to_string());
    lines.push(String::new());
    lines.push(
        if described {
            " | Prop | Type | Kind | Description |"
        } else {
            " | Prop | Type | Kind |"
        }
        .to_string(),
    );
    lines.push(if described { " |---|---|---|---|" } else { " |---|---|---|" }.to_string());
    for prop in &props {
        let row = format!(" | `{}` | `{}` | {} |", prop.name, prop.ty, prop.kind);
        lines.push(if described { format!("{row} {} |", prop.docs) } else { row });
    }

    if props.iter().any(|prop| prop.maybe) {
        lines.push(String::new());
        lines.push(format!(
            " Optional props also have a `{MAYBE_PREFIX}` setter that takes an `Option`. In `html!`, write `prop = [value]`."
        ));
    }

    lines
}

fn name_elided_lifetimes(sig: &mut syn::Signature) {
    struct Namer(Vec<Lifetime>);

    impl Namer {
        fn next(&mut self) -> Lifetime {
            let lifetime = Lifetime::new(&format!("'__l{}", self.0.len()), proc_macro2::Span::call_site());
            self.0.push(lifetime.clone());
            lifetime
        }
    }

    impl VisitMut for Namer {
        fn visit_item_mut(&mut self, _: &mut syn::Item) {}

        fn visit_type_bare_fn_mut(&mut self, _: &mut syn::TypeBareFn) {}

        fn visit_parenthesized_generic_arguments_mut(&mut self, _: &mut syn::ParenthesizedGenericArguments) {}

        fn visit_lifetime_mut(&mut self, lifetime: &mut Lifetime) {
            if lifetime.ident == "_" {
                *lifetime = self.next();
            }
        }

        fn visit_type_reference_mut(&mut self, reference: &mut syn::TypeReference) {
            syn::visit_mut::visit_type_reference_mut(self, reference);
            if reference.lifetime.is_none() {
                reference.lifetime = Some(self.next());
            }
        }
    }

    let mut namer = Namer(Vec::new());
    for input in &mut sig.inputs {
        namer.visit_fn_arg_mut(input);
    }
    for (index, lifetime) in namer.0.into_iter().enumerate() {
        sig.generics
            .params
            .insert(index, GenericParam::Lifetime(syn::LifetimeParam::new(lifetime)));
    }
}
