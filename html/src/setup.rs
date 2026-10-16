use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use proc_macro2_diagnostics::{Diagnostic, SpanDiagnosticExt};
use syn::{LitStr, parse::Parser, spanned::Spanned, visit::Visit, visit_mut::VisitMut};

use crate::{
    ast::*,
    conventions::{BQ_SETUP, HTML_MACROS, is_html_macro},
};

const MARKER: &str = "__bq_component_setup";

pub fn has_bare_setup(block: &syn::Block) -> bool {
    struct Finder(bool);

    impl<'ast> Visit<'ast> for Finder {
        fn visit_item(&mut self, _: &'ast syn::Item) {}

        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            rewrite(mac.tokens.clone(), is_html_macro(&mac.path), &mut |tokens| {
                self.0 |= tokens_have_bare_setup(&tokens);
                tokens
            });
        }
    }

    let mut finder = Finder(false);
    finder.visit_block(block);
    finder.0
}

pub fn markups_have_bare_setup(markups: &Markups<Element>) -> bool {
    let mut found = false;
    each_element(&mut markups.clone(), &mut |element| {
        found |= element.attrs.iter().any(is_bare_setup);
    });
    found
}

pub fn mark(block: &mut syn::Block, setup: &str) {
    struct Marker<'a>(&'a str);

    impl VisitMut for Marker<'_> {
        fn visit_item_mut(&mut self, _: &mut syn::Item) {}

        fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
            let tokens = std::mem::take(&mut mac.tokens);
            mac.tokens = rewrite(tokens, is_html_macro(&mac.path), &mut |tokens| marked(self.0, tokens));
        }
    }

    Marker(setup).visit_block_mut(block);
}

pub fn split_marker(input: TokenStream) -> (Option<LitStr>, TokenStream) {
    let mut tokens = input.clone().into_iter();
    let (Some(TokenTree::Punct(pound)), Some(TokenTree::Punct(bang)), Some(TokenTree::Group(group))) =
        (tokens.next(), tokens.next(), tokens.next())
    else {
        return (None, input);
    };
    if pound.as_char() != '#' || bang.as_char() != '!' || group.delimiter() != Delimiter::Bracket {
        return (None, input);
    }
    let mut inner = group.stream().into_iter();
    match (inner.next(), inner.next(), inner.next()) {
        (Some(TokenTree::Ident(name)), Some(TokenTree::Group(argument)), None)
            if name == MARKER && argument.delimiter() == Delimiter::Parenthesis =>
        {
            match syn::parse2::<LitStr>(argument.stream()) {
                Ok(setup) => (Some(setup), tokens.collect()),
                Err(_) => (None, input),
            }
        }
        _ => (None, input),
    }
}

pub fn bare(markups: &mut Markups<Element>, setup: Option<&LitStr>, diagnostics: &mut Vec<Diagnostic>) {
    each_element(markups, &mut |element| {
        for attr in &mut element.attrs {
            let span = attr.span();
            let Attribute::Named { name, attr_type } = attr else { continue };
            if name.to_string() != BQ_SETUP {
                continue;
            }
            match (&attr_type, setup) {
                (AttributeType::Empty(None), Some(setup)) => {
                    *attr_type = AttributeType::Normal {
                        eq_token: syn::token::Eq(span),
                        value: Markup::Lit(HtmlLit {
                            lit: LitStr::new(&setup.value(), span),
                        }),
                    }
                }
                (AttributeType::Empty(None), None) => diagnostics.push(
                    span.error("bare `bq-setup` can only be used directly inside a `#[component]` function, where it attaches that component's setup")
                        .help("helper functions cannot attach a setup: move this element into the component, or make the helper a `#[component]` that spreads `(..attrs)` and write `Helper bq-setup` in the component"),
                ),
                _ => diagnostics.push(
                    span.error("`bq-setup` does not take a value")
                        .help("write a bare `bq-setup` inside the `#[component]` function whose setup it attaches"),
                ),
            }
        }
    });
}

fn is_bare_setup(attr: &Attribute) -> bool {
    matches!(attr, Attribute::Named { name, attr_type: AttributeType::Empty(None) } if name.to_string() == BQ_SETUP)
}

fn tokens_have_bare_setup(tokens: &TokenStream) -> bool {
    let (_, tokens) = split_marker(tokens.clone());
    let mut diagnostics = Vec::new();
    Parser::parse2(
        |input: syn::parse::ParseStream| Markups::<Element>::diagnostic_parse(input, &mut diagnostics),
        tokens,
    )
    .is_ok_and(|markups| markups_have_bare_setup(&markups))
}

fn marked(setup: &str, tokens: TokenStream) -> TokenStream {
    let argument = Group::new(
        Delimiter::Parenthesis,
        TokenTree::Literal(proc_macro2::Literal::string(setup)).into(),
    );
    let inner: TokenStream = [TokenTree::Ident(Ident::new(MARKER, Span::call_site())), TokenTree::Group(argument)]
        .into_iter()
        .collect();
    let mut marked: Vec<TokenTree> = vec![
        TokenTree::Punct(Punct::new('#', Spacing::Alone)),
        TokenTree::Punct(Punct::new('!', Spacing::Alone)),
        TokenTree::Group(Group::new(Delimiter::Bracket, inner)),
    ];
    marked.extend(tokens);
    marked.into_iter().collect()
}

fn rewrite(tokens: TokenStream, html: bool, f: &mut impl FnMut(TokenStream) -> TokenStream) -> TokenStream {
    let mut out: Vec<TokenTree> = Vec::new();
    for token in tokens {
        let token = match token {
            TokenTree::Group(group) => {
                let nested = matches!(
                    out.as_slice(),
                    [.., TokenTree::Ident(name), TokenTree::Punct(bang)]
                        if HTML_MACROS.iter().any(|html| name == html) && bang.as_char() == '!'
                );
                let mut rebuilt = Group::new(group.delimiter(), rewrite(group.stream(), nested, f));
                rebuilt.set_span(group.span());
                TokenTree::Group(rebuilt)
            }
            other => other,
        };
        out.push(token);
    }
    let tokens = out.into_iter().collect();
    if html { f(tokens) } else { tokens }
}

fn each_element(markups: &mut Markups<Element>, f: &mut impl FnMut(&mut Element)) {
    for markup in &mut markups.markups {
        each_element_in(markup, f);
    }
}

fn each_element_in(markup: &mut Markup<Element>, f: &mut impl FnMut(&mut Element)) {
    match markup {
        Markup::Block(block) => each_element(&mut block.markups, f),
        Markup::Element(element) => {
            f(element);
            if let ElementBody::Block(block) = &mut element.body {
                each_element(&mut block.markups, f);
            }
        }
        Markup::ControlFlow(control_flow) => match &mut control_flow.kind {
            ControlFlowKind::If(if_) => each_element_in_if(if_, f),
            ControlFlowKind::For(for_) => each_element(&mut for_.body.markups, f),
            ControlFlowKind::While(while_) => each_element(&mut while_.body.markups, f),
            ControlFlowKind::Match(match_) => {
                for arm in &mut match_.arms {
                    each_element_in(&mut arm.body, f);
                }
            }
            ControlFlowKind::Let(_) => {}
        },
        Markup::Lit(_) | Markup::Splice { .. } | Markup::Semi(_) => {}
    }
}

fn each_element_in_if(if_: &mut IfExpr<Element>, f: &mut impl FnMut(&mut Element)) {
    each_element(&mut if_.then_branch.markups, f);
    if let Some((_, _, else_)) = &mut if_.else_branch {
        match else_.as_mut() {
            IfOrBlock::If(else_if) => each_element_in_if(else_if, f),
            IfOrBlock::Block(block) => each_element(&mut block.markups, f),
        }
    }
}
