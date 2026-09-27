use std::fmt::Write;

use maud::{Markup, PreEscaped, Render, html};

const TOGGLE_PASSWORD: &str = "const input = this.previousElementSibling; \
const hidden = input.type === 'password'; \
input.type = hidden ? 'text' : 'password'; \
this.textContent = hidden ? 'Hide' : 'Show';";

pub struct Input<'a> {
    name: &'a str,
    kind: &'a str,
    autocomplete: &'a str,
    password: bool,
    label: Option<&'a str>,
    placeholder: Option<&'a str>,
    value: Option<&'a str>,
    prefix: Option<&'a str>,
    required: bool,
    attrs: Vec<(&'a str, &'a str)>,
}

impl<'a> Input<'a> {
    fn new(name: &'a str, kind: &'a str, autocomplete: &'a str) -> Self {
        Self {
            name,
            kind,
            autocomplete,
            password: false,
            label: None,
            placeholder: None,
            value: None,
            prefix: None,
            required: false,
            attrs: Vec::new(),
        }
    }

    pub fn text(name: &'a str) -> Self {
        Self::new(name, "text", name)
    }

    pub fn email(name: &'a str) -> Self {
        Self::new(name, "email", "email")
    }

    pub fn password(name: &'a str) -> Self {
        let mut input = Self::new(name, "password", "current-password");
        input.password = true;
        input
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn autocomplete(mut self, autocomplete: &'a str) -> Self {
        self.autocomplete = autocomplete;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// any extra attribute on the `<input>`, e.g. `x-model` or `hx-post`
    pub fn attr(mut self, name: &'a str, value: &'a str) -> Self {
        self.attrs.push((name, value));
        self
    }

    /// maud has no splice in attribute position, so the tag is assembled by hand
    fn input_tag(&self, class: Option<&str>, type_attr: (&str, &str)) -> Markup {
        let mut attrs: Vec<(&str, &str)> = vec![
            ("id", self.name),
            ("name", self.name),
            type_attr,
            ("autocomplete", self.autocomplete),
        ];
        if let Some(class) = class {
            attrs.push(("class", class));
        }
        if let Some(p) = self.placeholder {
            attrs.push(("placeholder", p));
        }
        if let Some(v) = self.value {
            attrs.push(("value", v));
        }
        attrs.extend(self.attrs.iter().copied());

        let mut out = String::from("<input");
        for (name, value) in attrs {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            write!(maud::Escaper::new(&mut out), "{value}").expect("string write cannot fail");
            out.push('"');
        }
        if self.required {
            out.push_str(" required");
        }
        out.push('>');
        PreEscaped(out)
    }
}

impl Render for Input<'_> {
    fn render(&self) -> Markup {
        html! {
            div.input-component {
                @if let Some(label) = self.label {
                    label for=(self.name) { (label) }
                }
                @if self.password {
                    div.shell.password {
                        (self.input_tag(Some("password-input"), ("type", "password")))
                        button
                            type="button"
                            tabindex="-1"
                            hx-on:click=(TOGGLE_PASSWORD)
                            { "Show" }
                    }
                } @else {
                    div.shell .prefixed[self.prefix.is_some()] {
                        @if let Some(prefix) = self.prefix {
                            span.prefix { (prefix) }
                        }
                        (self.input_tag(None, ("type", self.kind)))
                    }
                }
                p.error id={ (self.name) "-error" } {}
            }
        }
    }
}
