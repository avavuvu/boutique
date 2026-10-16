use heck::{ToKebabCase, ToLowerCamelCase, ToUpperCamelCase};
use proc_macro2::Ident;
use syn::{
    LitStr, Path, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

pub const HTML_MACROS: &[&str] = &["html"];

pub const ATTRS: &str = "attrs";
pub const CHILDREN: &str = "children";
pub const FINISH_FN: &str = "build";
pub const MAYBE_PREFIX: &str = "maybe_";

pub const BQ_SETUP: &str = "bq-setup";
pub const BQ_REF: &str = "bq-ref";
pub const BQ_ON: &str = "bq-on:";

pub fn is_html_macro(path: &Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| HTML_MACROS.iter().any(|name| segment.ident == name))
}

pub fn merges(attribute: &str) -> bool {
    attribute == BQ_SETUP || attribute == BQ_REF || attribute.starts_with(BQ_ON)
}

pub fn kebab(text: &str) -> bool {
    text.chars().next().is_some_and(|first| first.is_ascii_lowercase())
        && !text.ends_with('-')
        && !text.contains("--")
        && text
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-')
}

pub fn maybe_setter(prop: &str) -> String {
    format!("{MAYBE_PREFIX}{prop}")
}

pub fn component_alias(function: &Ident) -> String {
    function.unraw().to_string().to_upper_camel_case()
}

pub fn setup_name(function: &Ident) -> String {
    function.unraw().to_string().to_kebab_case()
}

pub fn ts_type_name(setup: &str) -> String {
    setup.to_upper_camel_case()
}

pub fn ts_export_name(setup: &str) -> String {
    setup.to_lower_camel_case()
}

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
            None => setup_name(function),
        }
    }
}
