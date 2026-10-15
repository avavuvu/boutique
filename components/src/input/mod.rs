use maud::Markup;

use crate::{Attrs, component, html};

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
            Kind::Text => AUTOFILL.contains(&name).then_some(name),
            Kind::Email => Some("email"),
            Kind::Password => Some("current-password"),
            _ => None,
        }
    }
}

const AUTOFILL: &[&str] = &[
    "name",
    "given-name",
    "family-name",
    "nickname",
    "username",
    "email",
    "tel",
    "url",
    "organization",
    "street-address",
    "postal-code",
    "country-name",
    "one-time-code",
    "current-password",
    "new-password",
];


fn described_by(id: &str, hint: Option<&str>) -> String {
    match hint {
        Some(_) => format!("{id}-hint {id}-error"),
        None => format!("{id}-error"),
    }
}

#[component(setup = "password-toggle")]
pub fn input(
    #[positional] name: &str,
    id: Option<&str>,
    #[default] kind: Kind,
    label: Option<&str>,
    hint: Option<&str>,
    error: Option<&str>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    attrs: Attrs,
) -> Markup {
    let id = id.unwrap_or(name);

    let mut attrs = attrs;
    if let Some(autocomplete) = kind.autocomplete(name) {
        attrs.set_default("autocomplete", autocomplete);
    }

    let is_password = kind == Kind::Password;
    let control = html! {
        input
            #(id)
            name=(name)
            type=(kind.as_str())
            .password-input[is_password]
            aria-describedby=(described_by(id, hint))
            aria-invalid=[error.map(|_| "true")]
            (..attrs);
    };

    html! {
        div.input-component {
            @if let Some(label) = label {
                label for=(id) { (label) }
            }
            @if is_password {
                div.shell.password bq-setup {
                    (control)
                    button
                        type="button"
                        aria-label="Show password"
                        aria-pressed="false"
                        aria-controls=(id)
                        bq-on:click="toggle"
                    {
                        "Show"
                    }
                }
            } @else {
                div.shell.prefixed[prefix.is_some() || suffix.is_some()] {
                    @if let Some(prefix) = prefix { span.prefix { (prefix) } }
                    (control)
                    @if let Some(suffix) = suffix { span.suffix { (suffix) } }
                }
            }
            @if let Some(hint) = hint {
                p.hint id={ (id) "-hint" } { (hint) }
            }
            p.error id={ (id) "-error" } aria-live="polite" {
                @if let Some(error) = error { (error) }
            }
        }
    }
}

#[component]
pub fn textarea(
    #[positional] name: &str,
    id: Option<&str>,
    label: Option<&str>,
    hint: Option<&str>,
    error: Option<&str>,
    value: Option<&str>,
    attrs: Attrs,
) -> Markup {
    let id = id.unwrap_or(name);

    html! {
        div.input-component {
            @if let Some(label) = label {
                label for=(id) { (label) }
            }
            div.shell {
                textarea
                    #(id)
                    name=(name)
                    aria-describedby=(described_by(id, hint))
                    aria-invalid=[error.map(|_| "true")]
                    (..attrs)
                {
                    @if let Some(value) = value { (value) }
                }
            }
            @if let Some(hint) = hint {
                p.hint id={ (id) "-hint" } { (hint) }
            }
            p.error id={ (id) "-error" } aria-live="polite" {
                @if let Some(error) = error { (error) }
            }
        }
    }
}
