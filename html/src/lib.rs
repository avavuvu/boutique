#![allow(clippy::needless_pass_by_value)]

pub mod ast;
mod escape;
mod generate;

use ast::DiagnosticParse;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro2_diagnostics::Diagnostic;
use quote::quote;
use syn::parse::{ParseStream, Parser};

pub use proc_macro2_diagnostics;

pub fn expand(input: TokenStream, krate: TokenStream) -> TokenStream {
    let size_hint = input.to_string().len();

    let mut diagnostics = Vec::new();
    let markups = match Parser::parse2(
        |input: ParseStream| ast::Markups::diagnostic_parse(input, &mut diagnostics),
        input,
    ) {
        Ok(data) => data,
        Err(err) => {
            let err = err.to_compile_error();
            let diag_tokens = diagnostics.into_iter().map(Diagnostic::emit_as_expr_tokens);

            return quote! {{
                #err
                #(#diag_tokens)*
            }};
        }
    };

    let diag_tokens = diagnostics.into_iter().map(Diagnostic::emit_as_expr_tokens);

    let output_ident = Ident::new("__maud_output", Span::mixed_site());
    let stmts = generate::generate(markups, output_ident.clone(), krate.clone());
    quote! {{
        extern crate alloc;
        let mut #output_ident = alloc::string::String::with_capacity(#size_hint);
        #stmts
        #(#diag_tokens)*
        #krate::maud::PreEscaped(#output_ident)
    }}
}
