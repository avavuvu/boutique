use heck::ToKebabCase;
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream, TokenTree};
use proc_macro2_diagnostics::{Diagnostic, SpanDiagnosticExt};
use syn::{
    Expr, ExprPath, LitStr, Path, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Paren,
    visit_mut::VisitMut,
};

use crate::ast::*;

const MARKER: &str = "bq_setup";

#[derive(Default)]
pub struct ComponentOptions {
    pub eager: Option<Ident>,
    pub setup: Option<LitStr>,
}

impl Parse for ComponentOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut options = Self::default();
        while !input.is_empty() {
            let key = input.call(Ident::parse_any)?;
            if key == "eager" {
                if options.eager.is_some() {
                    return Err(syn::Error::new(key.span(), "`eager` is given two times"));
                }
                options.eager = Some(key);
            } else if key == "setup" {
                if options.setup.is_some() {
                    return Err(syn::Error::new(key.span(), "`setup` is given two times"));
                }
                input.parse::<Token![=]>()?;
                let name: LitStr = input.parse()?;
                if !kebab(&name.value()) {
                    let message = format!(
                        "{:?} is not a valid setup name; use lowercase kebab-case, for example \"editor-toolbar\"",
                        name.value()
                    );
                    return Err(syn::Error::new(name.span(), message));
                }
                options.setup = Some(name);
            } else {
                return Err(syn::Error::new(key.span(), "expected `eager` or `setup = \"…\"`"));
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(options)
    }
}

impl ComponentOptions {
    pub fn name(&self, function: &Ident) -> String {
        match &self.setup {
            Some(name) => name.value(),
            None => function.unraw().to_string().to_kebab_case(),
        }
    }
}

pub fn kebab(text: &str) -> bool {
    text.chars().next().is_some_and(|first| first.is_ascii_lowercase())
        && !text.ends_with('-')
        && !text.contains("--")
        && text
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-')
}

pub fn mark(block: &mut syn::Block, setup: &Ident) -> bool {
    let mut marker = Marker { setup, found: false };
    marker.visit_block_mut(block);
    marker.found
}

struct Marker<'a> {
    setup: &'a Ident,
    found: bool,
}

impl VisitMut for Marker<'_> {
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}

    fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
        let tokens = std::mem::take(&mut mac.tokens);
        let html = mac.path.segments.last().is_some_and(|segment| segment.ident == "html");
        mac.tokens = if html { self.html(tokens) } else { self.tokens(tokens) };
    }
}

impl Marker<'_> {
    fn tokens(&mut self, tokens: TokenStream) -> TokenStream {
        let mut out: Vec<TokenTree> = Vec::new();
        for token in tokens {
            let token = match token {
                TokenTree::Group(group) => {
                    let html = matches!(
                        out.as_slice(),
                        [.., TokenTree::Ident(name), TokenTree::Punct(bang)] if name == "html" && bang.as_char() == '!'
                    );
                    let stream = if html {
                        self.html(group.stream())
                    } else {
                        self.tokens(group.stream())
                    };
                    let mut rebuilt = Group::new(group.delimiter(), stream);
                    rebuilt.set_span(group.span());
                    TokenTree::Group(rebuilt)
                }
                other => other,
            };
            out.push(token);
        }
        out.into_iter().collect()
    }

    fn html(&mut self, tokens: TokenStream) -> TokenStream {
        let tokens = self.tokens(tokens);
        if !has_bare(tokens.clone()) {
            return tokens;
        }
        self.found = true;
        let argument = Group::new(Delimiter::Parenthesis, TokenTree::Ident(self.setup.clone()).into());
        let inner: TokenStream = [TokenTree::Ident(Ident::new(MARKER, self.setup.span())), TokenTree::Group(argument)]
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
}

fn is_punct(token: Option<&TokenTree>, characters: &str) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if characters.contains(punct.as_char()))
}

fn has_bare(tokens: TokenStream) -> bool {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    tokens.iter().enumerate().any(|(index, token)| match token {
        TokenTree::Group(group) => has_bare(group.stream()),
        TokenTree::Ident(ident) => {
            ident == "bq"
                && is_punct(tokens.get(index + 1), "-")
                && matches!(tokens.get(index + 2), Some(TokenTree::Ident(setup)) if setup == "setup")
                && !(index > 0 && is_punct(tokens.get(index - 1), ".#-:"))
                && !is_punct(tokens.get(index + 3), "=-:")
                && !matches!(tokens.get(index + 3), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket)
        }
        _ => false,
    })
}

pub fn split_marker(input: TokenStream) -> (Option<Path>, TokenStream) {
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
            match syn::parse2::<Path>(argument.stream()) {
                Ok(path) => (Some(path), tokens.collect()),
                Err(_) => (None, input),
            }
        }
        _ => (None, input),
    }
}

pub fn bare(markups: &mut Markups<Element>, setup: Option<&Path>, diagnostics: &mut Vec<Diagnostic>) {
    for markup in &mut markups.markups {
        bare_markup(markup, setup, diagnostics);
    }
}

fn bare_markup(markup: &mut Markup<Element>, setup: Option<&Path>, diagnostics: &mut Vec<Diagnostic>) {
    match markup {
        Markup::Block(block) => bare(&mut block.markups, setup, diagnostics),
        Markup::Element(element) => {
            bare_attrs(&mut element.attrs, setup, diagnostics);
            if let ElementBody::Block(block) = &mut element.body {
                bare(&mut block.markups, setup, diagnostics);
            }
        }
        Markup::ControlFlow(control_flow) => match &mut control_flow.kind {
            ControlFlowKind::If(if_) => bare_if(if_, setup, diagnostics),
            ControlFlowKind::For(for_) => bare(&mut for_.body.markups, setup, diagnostics),
            ControlFlowKind::While(while_) => bare(&mut while_.body.markups, setup, diagnostics),
            ControlFlowKind::Match(match_) => {
                for arm in &mut match_.arms {
                    bare_markup(&mut arm.body, setup, diagnostics);
                }
            }
            ControlFlowKind::Let(_) => {}
        },
        Markup::Lit(_) | Markup::Splice { .. } | Markup::Semi(_) => {}
    }
}

fn bare_if(if_: &mut IfExpr<Element>, setup: Option<&Path>, diagnostics: &mut Vec<Diagnostic>) {
    bare(&mut if_.then_branch.markups, setup, diagnostics);
    if let Some((_, _, else_)) = &mut if_.else_branch {
        match else_.as_mut() {
            IfOrBlock::If(else_if) => bare_if(else_if, setup, diagnostics),
            IfOrBlock::Block(block) => bare(&mut block.markups, setup, diagnostics),
        }
    }
}

fn bare_attrs(attrs: &mut [Attribute], setup: Option<&Path>, diagnostics: &mut Vec<Diagnostic>) {
    for attr in attrs {
        let Attribute::Named { name, attr_type } = attr else { continue };
        if !matches!(attr_type, AttributeType::Empty(None)) || name.to_string() != "bq-setup" {
            continue;
        }
        let span = name.span();
        match setup {
            Some(path) => {
                *attr_type = AttributeType::Normal {
                    eq_token: syn::token::Eq(span),
                    value: Markup::Splice {
                        paren_token: Paren(span),
                        expr: Expr::Path(ExprPath { attrs: Vec::new(), qself: None, path: path.clone() }),
                    },
                }
            }
            None => diagnostics.push(
                span.error("bare `bq-setup` can only be used directly inside a `#[component]` function, where it attaches that component's setup")
                    .help("helper functions cannot attach a setup: move this element into the component, or make the helper a `#[component]` that spreads `(..attrs)` and write `Helper bq-setup` in the component"),
            ),
        }
    }
}
