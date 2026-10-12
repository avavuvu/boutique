use maud::Markup;

use crate::{Attrs, component, html, setup};

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

#[component]
pub fn input(
    #[builder(start_fn)] name: &str,
    #[builder(default)] kind: Kind,
    label: Option<&str>,
    hint: Option<&str>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    #[builder(default)] attrs: Attrs,
) -> Markup {
    let mut attrs = attrs;
    if let Some(autocomplete) = kind.autocomplete(name) {
        attrs.set_default("autocomplete", autocomplete);
    }

    let is_password = kind == Kind::Password;
    let control = html! {
        input #(name) name=(name) type=(kind.as_str()) .password-input[is_password] (..attrs);
    };

    html! {
        div.input-component {
            @if let Some(label) = label {
                label for=(name) { (label) }
            }
            @if is_password {
                div.shell.password bq-setup=(PasswordToggle) {
                    (control)
                    button type="button" tabindex="-1" bq-on:click="toggle" { "Show" }
                }
            } @else {
                div.shell.prefixed[prefix.is_some() || suffix.is_some()] {
                    @if let Some(prefix) = prefix { span.prefix { (prefix) } }
                    (control)
                    @if let Some(suffix) = suffix { span.suffix { (suffix) } }
                }
            }
            @if let Some(hint) = hint {
                p.hint { (hint) }
            }
            p.error id={ (name) "-error" } {}
        }
    }
}

#[component]
pub fn textarea(
    #[builder(start_fn)] name: &str,
    label: Option<&str>,
    hint: Option<&str>,
    value: Option<&str>,
    #[builder(default)] attrs: Attrs,
) -> Markup {
    html! {
        div.input-component {
            @if let Some(label) = label {
                label for=(name) { (label) }
            }
            div.shell {
                textarea #(name) name=(name) (..attrs) {
                    @if let Some(value) = value { (value) }
                }
            }
            @if let Some(hint) = hint {
                p.hint { (hint) }
            }
            p.error id={ (name) "-error" } {}
        }
    }
}
