use bq_html::conventions::FINISH_FN;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{GenericParam, Ident, ItemFn, Lifetime, LifetimeParam, Signature, ext::IdentExt, visit_mut::VisitMut};

use crate::crate_path::crate_path;

pub fn expand(mut func: ItemFn, alias: &str) -> TokenStream {
    let crate_path = crate_path();
    func.attrs.push(syn::parse_quote!(#[allow(clippy::needless_pass_by_value)]));

    let fn_name = &func.sig.ident;
    let visibility = &func.vis;
    let builder = format_ident!("{alias}Builder");
    let state_mod = format_ident!("{fn_name}_builder");
    let finish_fn = Ident::new(FINISH_FN, Span::call_site());
    let alias = (fn_name.unraw() != alias).then(|| {
        let alias = Ident::new(alias, fn_name.span());
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

    quote! {
        #[#crate_path::bon::builder(
            crate = #crate_path::bon,
            builder_type(name = #builder),
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
    }
}

fn name_elided_lifetimes(sig: &mut Signature) {
    struct Namer(Vec<Lifetime>);

    impl Namer {
        fn next(&mut self) -> Lifetime {
            let lifetime = Lifetime::new(&format!("'__l{}", self.0.len()), Span::call_site());
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
            .insert(index, GenericParam::Lifetime(LifetimeParam::new(lifetime)));
    }
}
