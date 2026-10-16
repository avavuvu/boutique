use bq_html::conventions::{ATTRS, CHILDREN};
use quote::ToTokens;
use syn::{
    Attribute, Error, Expr, ExprLit, FnArg, GenericArgument, Lit, Meta, MetaNameValue, Pat, PathArguments, Signature, Type, ext::IdentExt,
};

#[derive(PartialEq)]
pub enum Role {
    Positional,
    Named,
    Children,
    Attrs,
}

pub enum DefaultValue {
    None,
    Default,
    Expr { value: Expr, written: String },
    Implicit,
}

pub struct Param {
    pub name: String,
    pub ty: Type,
    pub role: Role,
    pub default: DefaultValue,
    pub docs: String,
}

impl Param {
    pub fn optional(&self) -> bool {
        option_inner(&self.ty).is_some()
    }

    pub fn required(&self) -> bool {
        matches!(self.default, DefaultValue::None) && !self.optional()
    }

    fn builder_attribute(&self) -> Option<Attribute> {
        Some(match (&self.role, &self.default) {
            (Role::Positional, _) => syn::parse_quote!(#[builder(start_fn)]),
            (_, DefaultValue::None) => return None,
            (_, DefaultValue::Default) => syn::parse_quote!(#[builder(default)]),
            (_, DefaultValue::Expr { value, .. }) => syn::parse_quote!(#[builder(default = #value)]),
            (_, DefaultValue::Implicit) if is_type(&self.ty, "bool") => syn::parse_quote!(#[builder(default = false)]),
            (_, DefaultValue::Implicit) => syn::parse_quote!(#[builder(default)]),
        })
    }
}

pub fn take(sig: &mut Signature) -> syn::Result<Vec<Param>> {
    let mut params = Vec::new();
    for input in &mut sig.inputs {
        let typed = match input {
            FnArg::Receiver(receiver) => {
                return Err(Error::new_spanned(
                    receiver,
                    "a #[component] cannot take `self`; make it a free function",
                ));
            }
            FnArg::Typed(typed) => typed,
        };
        let Pat::Ident(pattern) = typed.pat.as_ref() else {
            return Err(Error::new_spanned(
                &typed.pat,
                "component parameters must be plain names, for example `title: &str`; destructure inside the function",
            ));
        };
        let name = pattern.ident.unraw().to_string();

        let mut positional = None;
        let mut default = None;
        let mut kept = Vec::new();
        for attr in std::mem::take(&mut typed.attrs) {
            if attr.path().is_ident("positional") {
                if !matches!(attr.meta, Meta::Path(_)) {
                    return Err(Error::new_spanned(&attr, "`#[positional]` takes no arguments"));
                }
                if positional.is_some() {
                    return Err(Error::new_spanned(&attr, "this parameter is already `#[positional]`"));
                }
                positional = Some(attr);
            } else if attr.path().is_ident("default") {
                if default.is_some() {
                    return Err(Error::new_spanned(&attr, "this parameter already has a default"));
                }
                default = Some(match &attr.meta {
                    Meta::Path(_) => (DefaultValue::Default, attr),
                    Meta::NameValue(pair) => (
                        DefaultValue::Expr {
                            value: pair.value.clone(),
                            written: written(&attr, &pair.value),
                        },
                        attr,
                    ),
                    Meta::List(_) => return Err(Error::new_spanned(&attr, "expected `#[default]` or `#[default = …]`")),
                });
            } else if attr.path().is_ident("builder") {
                return Err(Error::new_spanned(
                    &attr,
                    "`#[builder(…)]` is not supported on component parameters; use `#[positional]`, `#[default]` or `#[default = …]`",
                ));
            } else {
                kept.push(attr);
            }
        }

        if let (Some(_), Some((_, attr))) = (&positional, &default) {
            return Err(Error::new_spanned(
                attr,
                "a `#[positional]` parameter cannot have a default; positional arguments are always given",
            ));
        }

        let role = match name.as_str() {
            _ if positional.is_some() => Role::Positional,
            CHILDREN => Role::Children,
            ATTRS => Role::Attrs,
            _ => Role::Named,
        };
        let default = match default {
            Some((default, _)) => default,
            None if role == Role::Positional => DefaultValue::None,
            None if is_type(&typed.ty, "bool") || role == Role::Children || (role == Role::Attrs && is_type(&typed.ty, "Attrs")) => {
                DefaultValue::Implicit
            }
            None => DefaultValue::None,
        };

        let param = Param {
            name,
            ty: (*typed.ty).clone(),
            role,
            default,
            docs: doc_text(&kept),
        };
        kept.extend(param.builder_attribute());
        typed.attrs = kept;
        params.push(param);
    }
    Ok(params)
}

fn written(attr: &Attribute, value: &Expr) -> String {
    attr.bracket_token
        .span
        .join()
        .source_text()
        .and_then(|text| {
            let inner = text.strip_prefix('[')?.strip_suffix(']')?.trim_start();
            let value = inner.strip_prefix("default")?.trim_start().strip_prefix('=')?;
            Some(
                value
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
        .unwrap_or_else(|| value.to_token_stream().to_string())
}

pub fn is_type(ty: &Type, name: &str) -> bool {
    let Type::Path(path) = ty else { return false };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name && segment.arguments.is_none())
}

pub fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path
        .path
        .segments
        .last()
        .filter(|segment| path.qself.is_none() && segment.ident == "Option")?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match arguments.args.first() {
        Some(GenericArgument::Type(inner)) => Some(inner),
        _ => None,
    }
}

pub fn doc_text(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                value: Expr::Lit(ExprLit { lit: Lit::Str(text), .. }),
                ..
            }) => Some(text.value().trim().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
