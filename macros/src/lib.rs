use heck::{ToKebabCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use syn::{
    Expr, GenericParam, Ident, ItemFn, Lifetime, LitStr, Token, Visibility,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
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

        let value = if input.parse::<Option<Token![=]>>()?.is_some() { Some(input.parse()?) } else { None };
        Ok(Self { name, span, value })
    }
}

fn valid_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control() && !"\"'<>/=`".contains(character))
}

fn krate() -> proc_macro2::TokenStream {
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
    bq_html::expand(input.into(), krate()).into()
}

#[proc_macro]
pub fn attrs(input: TokenStream) -> TokenStream {
    let pairs = parse_macro_input!(input with Punctuated::<AttributePair, Token![,]>::parse_terminated);
    let attrs = proc_macro2::Ident::new("__bq_attrs", proc_macro2::Span::mixed_site());
    let buffer = proc_macro2::Ident::new("__bq_value", proc_macro2::Span::mixed_site());
    let krate = krate();

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
                #krate::maud::macro_private::render_to!(&(#value), &mut #buffer);
                #attrs.__text(#name, #buffer);
            }},
            None => quote!(#attrs.__flag(#name);),
        });
    }

    quote! {{
        let mut #attrs = #krate::Attrs::new();
        #(#items)*
        #attrs
    }}
    .into()
}

struct SetupDeclaration {
    visibility: Visibility,
    name: Ident,
}

impl Parse for SetupDeclaration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let visibility = input.parse()?;
        let name = input.parse()?;
        if input.parse::<Option<Token![,]>>()?.is_some() {
            let option: Ident = input.parse()?;
            if option != "eager" {
                return Err(syn::Error::new(option.span(), "the only option for setup!(..) is `eager`"));
            }
        }
        Ok(Self { visibility, name })
    }
}

#[proc_macro]
pub fn setup(input: TokenStream) -> TokenStream {
    let SetupDeclaration { visibility, name } = parse_macro_input!(input as SetupDeclaration);
    let attribute = name.to_string().to_kebab_case();
    let krate = krate();

    quote! {
        #visibility enum #name {}

        impl #name {
            #visibility const NAME: &'static str = #attribute;
        }

        impl #krate::Setup for #name {
            const NAME: &'static str = #attribute;
        }
    }
    .into()
}

/// Turns a function that returns `Markup` into a component.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    if let Err(error) = translate_params(&mut func) {
        return error.to_compile_error().into();
    }

    let pascal = func.sig.ident.unraw().to_string().to_upper_camel_case();
    let docs = component_docs(&pascal, &func);
    if func.attrs.iter().any(is_doc) {
        func.attrs.push(syn::parse_quote!(#[doc = ""]));
    }
    func.attrs.extend(docs.iter().map(|line| -> syn::Attribute { syn::parse_quote!(#[doc = #line]) }));

    let fn_name = &func.sig.ident;
    let visibility = &func.vis;
    let builder = format_ident!("{pascal}Builder");
    let state_mod = format_ident!("{fn_name}_builder");
    let alias = (fn_name.unraw() != pascal).then(|| {
        let alias = Ident::new(&pascal, fn_name.span());
        quote!(#visibility use #fn_name as #alias;)
    });

    let elided = count_elided_lifetimes(&func);
    let named: Vec<&GenericParam> = func.sig.generics.params.iter().collect();

    let generated: Vec<Lifetime> = (0..elided)
        .map(|index| Lifetime::new(&format!("'__l{index}"), proc_macro2::Span::call_site()))
        .collect();

    let named_args = named.iter().map(|param| match param {
        GenericParam::Lifetime(lt) => {
            let lt = &lt.lifetime;
            quote! { #lt }
        }
        GenericParam::Type(ty) => {
            let ident = &ty.ident;
            quote! { #ident }
        }
        GenericParam::Const(c) => {
            let ident = &c.ident;
            quote! { #ident }
        }
    });

    let lifetimes_named = named.iter().filter(|p| matches!(p, GenericParam::Lifetime(_)));
    let others_named = named.iter().filter(|p| !matches!(p, GenericParam::Lifetime(_)));
    let krate = krate();

    let expanded = quote! {
        #[#krate::bon::builder(
            crate = #krate::bon,
            builder_type(name = #builder, doc { #(#[doc = #docs])* }),
            state_mod = #state_mod,
            finish_fn = build,
            derive(Clone)
        )]
        #func

        #alias

        impl<#(#lifetimes_named,)* #(#generated,)* #(#others_named,)* __S: #state_mod::IsComplete>
            #krate::maud::Render for #builder<#(#named_args,)* #(#generated,)* __S>
        {
            fn render(&self) -> #krate::maud::Markup {
                ::core::clone::Clone::clone(self).build()
            }
        }
    };

    expanded.into()
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
            } else if (name == "attrs" && is_type(&typed.ty, "Attrs")) || name == "children" {
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
                value: Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }),
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
    let segment = path.path.segments.last().filter(|segment| path.qself.is_none() && segment.ident == "Option")?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else { return None };
    match arguments.args.first() {
        Some(syn::GenericArgument::Type(inner)) => Some(inner),
        _ => None,
    }
}

fn tidy(tokens: &str) -> String {
    [(" < ", "<"), ("< ", "<"), (" >", ">"), ("& ", "&"), (" ,", ","), (" :: ", "::"), (":: ", "::"), (" !", "!"), ("! ", "!")]
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
            let syn::Pat::Ident(pattern) = typed.pat.as_ref() else { return None };
            let name = pattern.ident.unraw().to_string();
            let options = builder_options(&typed.attrs);
            let optional_option = !options.required && option_inner(&typed.ty).is_some();

            let role = match name.as_str() {
                _ if options.start_fn => Role::Positional,
                "children" => Role::Children,
                "attrs" => Role::Attrs,
                _ => Role::Named,
            };

            let setter_type = if optional_option { option_inner(&typed.ty) } else { None }.unwrap_or(&typed.ty);
            let setter_type = tidy(&quote!(#setter_type).to_string());
            let ty = if options.into { format!("impl Into<{setter_type}>") } else { setter_type };

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
    let mut arguments: Vec<String> = props.iter().filter(|prop| prop.role == Role::Positional).map(|prop| prop.name.clone()).collect();
    arguments.extend(
        props
            .iter()
            .filter(|prop| prop.role == Role::Named && prop.required)
            .map(|prop| format!("{} = …", prop.name)),
    );

    let body = if props.iter().any(|prop| prop.role == Role::Children) { " { … }" } else { ";" };

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
    lines.push(if described { " | Prop | Type | Kind | Description |" } else { " | Prop | Type | Kind |" }.to_string());
    lines.push(if described { " |---|---|---|---|" } else { " |---|---|---|" }.to_string());
    for prop in &props {
        let row = format!(" | `{}` | `{}` | {} |", prop.name, prop.ty, prop.kind);
        lines.push(if described { format!("{row} {} |", prop.docs) } else { row });
    }

    if props.iter().any(|prop| prop.maybe) {
        lines.push(String::new());
        lines.push(" Optional props also have a `maybe_` setter that takes an `Option`. In `html!`, write `prop = [value]`.".to_string());
    }

    lines
}

fn count_elided_lifetimes(func: &ItemFn) -> usize {
    struct Counter(usize);

    impl<'ast> syn::visit::Visit<'ast> for Counter {
        fn visit_type_reference(&mut self, reference: &'ast syn::TypeReference) {
            if reference.lifetime.is_none() {
                self.0 += 1;
            }
            syn::visit::visit_type_reference(self, reference);
        }

        fn visit_lifetime(&mut self, lifetime: &'ast Lifetime) {
            if lifetime.ident == "_" {
                self.0 += 1;
            }
        }
    }

    let mut counter = Counter(0);
    for input in &func.sig.inputs {
        syn::visit::Visit::visit_fn_arg(&mut counter, input);
    }
    counter.0
}
