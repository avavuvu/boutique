use maud::{Markup, PreEscaped, html};

use crate::{
    component, setup,
    tag::{Attrs, Tag},
};

setup!(PasswordToggle);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Text,
    Email,
    Password,
    Search,
    Url,
    Tel,
    Number,
    Color,
    File,
    Date,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Email => "email",
            Kind::Password => "password",
            Kind::Search => "search",
            Kind::Url => "url",
            Kind::Tel => "tel",
            Kind::Number => "number",
            Kind::Color => "color",
            Kind::File => "file",
            Kind::Date => "date",
        }
    }

    fn autocomplete(self, name: &str) -> Option<&str> {
        match self {
            Kind::Text => Some(name),
            Kind::Email => Some("email"),
            Kind::Password => Some("current-password"),
            _ => None,
        }
    }
}

fn field(name: &str, label: Option<&str>, hint: Option<&str>, shell_classes: &str, control: Markup) -> Markup {
    html! {
        div.input-component {
            @if let Some(label) = label {
                label for=(name) { (label) }
            }
            div class=(shell_classes) { (control) }
            @if let Some(hint) = hint {
                p.hint { (hint) }
            }
            p.error id={ (name) "-error" } {}
        }
    }
}

#[component]
pub fn input(
    #[builder(start_fn)] name: &str,
    #[builder(default)] kind: Kind,
    label: Option<&str>,
    hint: Option<&str>,
    placeholder: Option<&str>,
    value: Option<&str>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    autocomplete: Option<&str>,
    pattern: Option<&str>,
    accept: Option<&str>,
    min_length: Option<u32>,
    max_length: Option<u32>,
    #[builder(default, with = || true)] required: bool,
    #[builder(default, with = || true)] disabled: bool,
    #[builder(default)] attrs: Attrs<'_>,
) -> Markup {
    let password = kind == Kind::Password;
    let min_length = min_length.map(|length| length.to_string());
    let max_length = max_length.map(|length| length.to_string());

    let tag = Tag::open("input")
        .attr("id", name)
        .attr("name", name)
        .attr("type", kind.as_str())
        .maybe("autocomplete", autocomplete.or_else(|| kind.autocomplete(name)))
        .maybe("class", password.then_some("password-input"))
        .maybe("placeholder", placeholder)
        .maybe("value", value)
        .maybe("pattern", pattern)
        .maybe("accept", accept)
        .maybe("minlength", min_length.as_deref())
        .maybe("maxlength", max_length.as_deref())
        .attrs(attrs)
        .flag("required", required)
        .flag("disabled", disabled)
        .finish();

    if password {
        return html! {
            div.input-component {
                @if let Some(label) = label {
                    label for=(name) { (label) }
                }
                div.shell.password bq-setup=(PasswordToggle) {
                    (tag)
                    button type="button" tabindex="-1" bq-ref="toggle" { "Show" }
                }
                @if let Some(hint) = hint {
                    p.hint { (hint) }
                }
                p.error id={ (name) "-error" } {}
            }
        };
    }

    let affixed = prefix.is_some() || suffix.is_some();
    let control = html! {
        @if let Some(prefix) = prefix { span.prefix { (prefix) } }
        (tag)
        @if let Some(suffix) = suffix { span.suffix { (suffix) } }
    };

    field(name, label, hint, if affixed { "shell prefixed" } else { "shell" }, control)
}

#[component]
pub fn textarea(
    #[builder(start_fn)] name: &str,
    label: Option<&str>,
    hint: Option<&str>,
    placeholder: Option<&str>,
    value: Option<&str>,
    rows: Option<u16>,
    max_length: Option<u32>,
    #[builder(default, with = || true)] required: bool,
    #[builder(default, with = || true)] disabled: bool,
    #[builder(default)] attrs: Attrs<'_>,
) -> Markup {
    let rows = rows.map(|rows| rows.to_string());
    let max_length = max_length.map(|length| length.to_string());

    let open = Tag::open("textarea")
        .attr("id", name)
        .attr("name", name)
        .maybe("rows", rows.as_deref())
        .maybe("placeholder", placeholder)
        .maybe("maxlength", max_length.as_deref())
        .attrs(attrs)
        .flag("required", required)
        .flag("disabled", disabled)
        .finish();

    let control = html! {
        (open)
        @if let Some(value) = value { (value) }
        (PreEscaped("</textarea>"))
    };

    field(name, label, hint, "shell", control)
}
