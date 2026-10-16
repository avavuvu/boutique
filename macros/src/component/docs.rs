use bq_html::conventions::MAYBE_PREFIX;
use syn::ItemFn;

use super::params::{DefaultValue, Param, Role};

pub fn attach(func: &mut ItemFn, alias: &str, params: &[Param]) {
    if func.attrs.iter().any(|attr| attr.path().is_ident("doc")) {
        func.attrs.push(syn::parse_quote!(#[doc = ""]));
    }
    func.attrs.extend(
        lines(alias, params)
            .iter()
            .map(|line| -> syn::Attribute { syn::parse_quote!(#[doc = #line]) }),
    );
}

fn lines(alias: &str, params: &[Param]) -> Vec<String> {
    let mut lines = vec![" # Example".to_string(), " ```rust,ignore".to_string()];
    lines.extend(example(alias, params).into_iter().map(|line| format!(" {line}")));
    lines.push(" ```".to_string());

    if params.is_empty() {
        return lines;
    }

    let described = params.iter().any(|param| !param.docs.is_empty());
    lines.push(String::new());
    lines.push(" # Props".to_string());
    lines.push(String::new());
    lines.push(
        if described {
            " | Prop | Kind | Description |"
        } else {
            " | Prop | Kind |"
        }
        .to_string(),
    );
    lines.push(if described { " |---|---|---|" } else { " |---|---|" }.to_string());
    for param in params {
        let row = format!(" | `{}` | {} |", param.name, cell(&kind(param)));
        lines.push(if described { format!("{row} {} |", cell(&param.docs)) } else { row });
    }

    if params.iter().any(|param| param.role == Role::Named && !param.required()) {
        lines.push(String::new());
        lines.push(format!(
            " Optional props also have a `{MAYBE_PREFIX}` setter that takes an `Option`. In `html!`, write `prop = [value]`."
        ));
    }

    lines
}

fn example(alias: &str, params: &[Param]) -> Vec<String> {
    let mut arguments: Vec<String> = params
        .iter()
        .filter(|param| param.role == Role::Positional)
        .map(|param| param.name.clone())
        .collect();
    arguments.extend(
        params
            .iter()
            .filter(|param| param.role == Role::Named && param.required())
            .map(|param| format!("{} = …", param.name)),
    );

    let body = if params.iter().any(|param| param.role == Role::Children) {
        " { … }"
    } else {
        ";"
    };

    match arguments.len() {
        0 => vec![format!("{alias}{body}")],
        1..=2 => vec![format!("{alias}({}){body}", arguments.join(", "))],
        _ => std::iter::once(format!("{alias}("))
            .chain(arguments.iter().map(|argument| format!("    {argument},")))
            .chain(std::iter::once(format!("){body}")))
            .collect(),
    }
}

fn kind(param: &Param) -> String {
    match (&param.role, &param.default) {
        (Role::Positional, _) => "positional".to_string(),
        (Role::Attrs, _) => "call-site attributes".to_string(),
        (Role::Children, DefaultValue::None) => "block body".to_string(),
        (Role::Children, _) => "block body, optional".to_string(),
        (Role::Named, DefaultValue::Expr { written, .. }) => format!("optional, default `{written}`"),
        (Role::Named, DefaultValue::Implicit) => "optional, default `false`".to_string(),
        (Role::Named, DefaultValue::Default) => "optional, default".to_string(),
        (Role::Named, DefaultValue::None) if param.optional() => "optional".to_string(),
        (Role::Named, DefaultValue::None) => "required".to_string(),
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}
