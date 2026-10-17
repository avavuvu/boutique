use std::sync::Arc;

use bq_html::{
    ast::{DiagnosticParse, Element, Markups},
    conventions::{HTML_MACROS, is_html_macro},
    proc_macro2_diagnostics::Diagnostic,
};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{parse::Parser, spanned::Spanned, visit::Visit};

use crate::diagnostics::{Loc, Sink};

pub fn parse(found: Found, file: &Arc<str>, sink: &mut Sink) -> Option<Markups<Element>> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let parsed = Parser::parse2(
        |input: syn::parse::ParseStream| Markups::diagnostic_parse(input, &mut diagnostics),
        found.tokens,
    );
    for diagnostic in diagnostics {
        for error in syn::Error::from(diagnostic) {
            sink.warning(&Loc::new(file, error.span()), format!("html!: {error}"), None);
        }
    }
    match parsed {
        Ok(markups) => Some(markups),
        Err(error) => {
            sink.warning(
                &Loc::new(file, found.span),
                format!("html! could not be parsed, so its bq-* attributes are ignored: {error}"),
                None,
            );
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Flags {
    pub optional: bool,
    pub many: bool,
}

impl Flags {
    pub const OPTIONAL: Flags = Flags {
        optional: true,
        many: false,
    };
    pub const MANY: Flags = Flags {
        optional: false,
        many: true,
    };

    fn enter(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Closure(_) | syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => self.many = true,
            syn::Expr::If(_) | syn::Expr::Match(_) => self.optional = true,
            _ => {}
        }
    }
}

pub struct Found {
    pub span: Span,
    pub tokens: TokenStream,
    pub flags: Flags,
    pub binding: Option<String>,
}

#[derive(Default)]
pub struct HtmlFinder {
    found: Vec<Found>,
    flags: Flags,
    binding: Option<String>,
}

impl HtmlFinder {
    pub fn expr(expr: &syn::Expr) -> Vec<Found> {
        let mut finder = Self::default();
        finder.visit_expr(expr);
        finder.found
    }

    pub fn block(block: &syn::Block) -> Vec<Found> {
        let mut finder = Self::default();
        finder.visit_block(block);
        finder.found
    }

    pub fn mac(mac: &syn::Macro) -> Vec<Found> {
        let mut finder = Self::default();
        finder.visit_macro(mac);
        finder.found
    }
}

impl<'ast> Visit<'ast> for HtmlFinder {
    fn visit_item(&mut self, _: &'ast syn::Item) {}

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let name = match (binding_name(&local.pat), &local.init) {
            (Some(name), Some(init)) if matches!(init.expr.as_ref(), syn::Expr::Macro(mac) if is_html_macro(&mac.mac.path)) => Some(name),
            _ => None,
        };
        let previous = std::mem::replace(&mut self.binding, name);
        syn::visit::visit_local(self, local);
        self.binding = previous;
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        let saved = self.flags;
        self.flags.enter(expr);
        syn::visit::visit_expr(self, expr);
        self.flags = saved;
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let binding = self.binding.take();
        if is_html_macro(&mac.path) {
            self.found.push(Found {
                span: mac.path.span(),
                tokens: mac.tokens.clone(),
                flags: self.flags,
                binding,
            });
        } else {
            let flags = self.flags;
            groups(mac.tokens.clone(), &mut |span, tokens| {
                self.found.push(Found {
                    span,
                    tokens,
                    flags,
                    binding: None,
                })
            });
        }
    }
}

fn groups(tokens: TokenStream, found: &mut impl FnMut(Span, TokenStream)) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        let TokenTree::Group(group) = token else { continue };
        let html = index >= 2
            && matches!(&tokens[index - 2], TokenTree::Ident(ident) if HTML_MACROS.iter().any(|name| ident == name))
            && matches!(&tokens[index - 1], TokenTree::Punct(punct) if punct.as_char() == '!');
        if html {
            found(tokens[index - 2].span(), group.stream());
        } else {
            groups(group.stream(), found);
        }
    }
}

fn binding_name(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(pat) => Some(pat.ident.to_string()),
        syn::Pat::Type(pat) => binding_name(&pat.pat),
        _ => None,
    }
}
