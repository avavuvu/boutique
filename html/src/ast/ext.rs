use proc_macro2::{Delimiter, Spacing, TokenStream};
use proc_macro2_diagnostics::Diagnostic;
use quote::ToTokens;
use syn::{
    Error, Expr, Ident, Path,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{At, Bracket, Colon, Comma, DotDot, Eq, EqEq, FatArrow, Minus, Or, OrOr, Paren},
};

use super::{Attribute, DiagnosticParse, DiagonsticParseExt, HtmlName, HtmlNameFragment, HtmlNamePunct};
use crate::conventions::BQ_ON;

pub mod kw {
    syn::custom_keyword!(element);
}

#[derive(Debug, Clone)]
pub enum ElementName {
    Static(HtmlName),
    Dynamic {
        at_token: At,
        element_token: kw::element,
        paren_token: Paren,
        expr: Box<Expr>,
    },
    Component {
        path: Path,
        paren_token: Option<Paren>,
        args: Punctuated<ComponentArg, Comma>,
    },
}

fn peek_spread(input: ParseStream) -> bool {
    let Some((inside, _, _)) = input.cursor().group(Delimiter::Parenthesis) else {
        return false;
    };
    let Some((first, next)) = inside.punct() else {
        return false;
    };
    first.as_char() == '.' && first.spacing() == Spacing::Joint && next.punct().is_some_and(|(second, _)| second.as_char() == '.')
}

fn peek_component(input: ParseStream) -> bool {
    let fork = input.fork();
    let Ok(path) = fork.call(Path::parse_mod_style) else {
        return false;
    };
    let capitalised = path
        .segments
        .last()
        .and_then(|segment| segment.ident.unraw().to_string().chars().next())
        .is_some_and(|first| first.is_ascii_uppercase());
    capitalised && !fork.peek(Minus) && !fork.peek(Colon)
}

impl DiagnosticParse for ElementName {
    fn diagnostic_parse(input: ParseStream, diagnostics: &mut Vec<Diagnostic>) -> syn::Result<Self> {
        if input.peek(At) {
            let content;
            Ok(Self::Dynamic {
                at_token: input.parse()?,
                element_token: input.parse()?,
                paren_token: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else if peek_component(input) {
            let path = input.call(Path::parse_mod_style)?;
            if !input.peek(Paren) || peek_spread(input) {
                return Ok(Self::Component {
                    path,
                    paren_token: None,
                    args: Punctuated::new(),
                });
            }
            let content;
            let paren_token = parenthesized!(content in input);
            let args = Punctuated::<ComponentArg, Comma>::parse_terminated(&content)?;
            let mut named = None;
            for arg in &args {
                match arg {
                    ComponentArg::Named { name, .. } => named = Some(name),
                    ComponentArg::Positional(expr) => {
                        if let Some(name) = named {
                            return Err(Error::new_spanned(
                                expr,
                                format!("positional arguments must come before named arguments like `{name} = ..`"),
                            ));
                        }
                    }
                }
            }
            Ok(Self::Component {
                path,
                paren_token: Some(paren_token),
                args,
            })
        } else {
            input.diagnostic_parse(diagnostics).map(Self::Static)
        }
    }
}

#[derive(Debug, Clone)]
pub enum ComponentArg {
    Positional(Expr),
    Named {
        name: Ident,
        eq_token: Eq,
        maybe: Option<Bracket>,
        value: Expr,
    },
}

impl Parse for ComponentArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let named = input.peek(Ident) && input.peek2(Eq) && !input.peek2(EqEq) && !input.peek2(FatArrow);
        if named {
            let name = input.parse()?;
            let eq_token = input.parse()?;
            let (maybe, value) = match input.parse()? {
                Expr::Array(array) if array.elems.len() == 1 && !array.elems.trailing_punct() => {
                    (Some(array.bracket_token), array.elems.into_iter().next().unwrap())
                }
                value => (None, value),
            };
            Ok(Self::Named {
                name,
                eq_token,
                maybe,
                value,
            })
        } else {
            input.parse().map(Self::Positional)
        }
    }
}

impl ToTokens for ComponentArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Positional(expr) => expr.to_tokens(tokens),
            Self::Named {
                name,
                eq_token,
                maybe,
                value,
            } => {
                name.to_tokens(tokens);
                eq_token.to_tokens(tokens);
                match maybe {
                    Some(bracket_token) => bracket_token.surround(tokens, |tokens| value.to_tokens(tokens)),
                    None => value.to_tokens(tokens),
                }
            }
        }
    }
}

impl ToTokens for ElementName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Static(name) => name.to_tokens(tokens),
            Self::Dynamic {
                at_token,
                element_token,
                paren_token,
                expr,
            } => {
                at_token.to_tokens(tokens);
                element_token.to_tokens(tokens);
                paren_token.surround(tokens, |tokens| expr.to_tokens(tokens));
            }
            Self::Component { path, paren_token, args } => {
                path.to_tokens(tokens);
                if let Some(paren_token) = paren_token {
                    paren_token.surround(tokens, |tokens| args.to_tokens(tokens));
                }
            }
        }
    }
}

pub(super) fn reject_spread(paren_token: Paren, content: ParseStream) -> syn::Result<()> {
    if content.peek(DotDot) {
        return Err(Error::new(paren_token.span.join(), "`(..attrs)` must come after an element"));
    }
    Ok(())
}

pub(super) fn parse_spread(input: ParseStream) -> syn::Result<Attribute> {
    let content;
    let paren_token = parenthesized!(content in input);
    if !content.peek(DotDot) {
        return Err(Error::new(
            paren_token.span.join(),
            "expected `..` before the attributes to spread, for example `(..attrs)`",
        ));
    }
    Ok(Attribute::Spread {
        paren_token,
        dot2_token: content.parse()?,
        expr: content.parse()?,
    })
}

pub(super) fn spread_to_tokens(paren_token: &Paren, dot2_token: &DotDot, expr: &Expr, tokens: &mut TokenStream) {
    paren_token.surround(tokens, |tokens| {
        dot2_token.to_tokens(tokens);
        expr.to_tokens(tokens);
    });
}

pub(super) fn parse_modifier_pipe(
    input: ParseStream,
    name: &Punctuated<HtmlNameFragment, HtmlNamePunct>,
) -> syn::Result<Option<HtmlNamePunct>> {
    if !input.peek(Or) || !is_event_name(name) {
        return Ok(None);
    }
    if input.peek(OrOr) {
        return Err(input.error("write one `|` between modifiers, for example `bq-on:click|prevent`"));
    }
    Ok(Some(HtmlNamePunct::Pipe(input.parse()?)))
}

fn is_event_name(name: &Punctuated<HtmlNameFragment, HtmlNamePunct>) -> bool {
    let text = HtmlName { name: name.clone() }.to_string();
    text.strip_prefix(BQ_ON).is_some_and(|event| !event.is_empty())
}
