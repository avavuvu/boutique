use heck::{ToKebabCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    GenericParam, Ident, ItemFn, Lifetime, Token, Visibility,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

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

    quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #visibility struct #name;

        impl #name {
            #visibility const NAME: &'static str = #attribute;
        }

        impl ::bq_components::maud::Render for #name {
            fn render_to(&self, buffer: &mut ::std::string::String) {
                buffer.push_str(#attribute);
            }
        }
    }
    .into()
}

/// Turns a function that returns `Markup` into a component.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    let fn_name = &func.sig.ident;
    let builder = format_ident!("{}Builder", fn_name.to_string().to_upper_camel_case());
    let state_mod = format_ident!("{fn_name}_builder");

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

    let expanded = quote! {
        #[::bq_components::bon::builder(
            crate = ::bq_components::bon,
            builder_type = #builder,
            state_mod = #state_mod,
            finish_fn = build,
            derive(Clone)
        )]
        #func

        impl<#(#lifetimes_named,)* #(#generated,)* #(#others_named,)* __S: #state_mod::IsComplete>
            ::bq_components::maud::Render for #builder<#(#named_args,)* #(#generated,)* __S>
        {
            fn render(&self) -> ::bq_components::maud::Markup {
                ::core::clone::Clone::clone(self).build()
            }
        }
    };

    expanded.into()
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
