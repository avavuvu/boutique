use std::fmt::Write;

use maud::{Escaper, Markup, PreEscaped, Render, html};

enum Action {
    Link(String),
    Post(String),
    Submit,
    Button,
}

pub struct Button {
    content: Markup,
    action: Action,
    classes: Vec<String>,
    attrs: Vec<(String, String)>,
}

impl Button {
    fn new(content: Markup, action: Action) -> Self {
        Self { content, action, classes: Vec::new(), attrs: Vec::new() }
    }

    pub fn link(content: Markup, href: impl Into<String>) -> Self {
        Self::new(content, Action::Link(href.into()))
    }

    pub fn post(content: Markup, url: impl Into<String>) -> Self {
        Self::new(content, Action::Post(url.into()))
    }

    pub fn submit(content: Markup) -> Self {
        Self::new(content, Action::Submit)
    }

    pub fn button(content: Markup) -> Self {
        Self::new(content, Action::Button)
    }

    pub fn attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.push((name.into(), value.into()));
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.classes.push(class.into());
        self
    }

    pub fn primary(self) -> Self {
        self.class("primary")
    }

    pub fn secondary(self) -> Self {
        self.class("secondary")
    }

    pub fn danger(self) -> Self {
        self.class("danger")
    }

    pub fn ghost(self) -> Self {
        self.class("ghost")
    }

    pub fn small(self) -> Self {
        self.class("small")
    }

    pub fn action(self, name: &str) -> Self {
        self.attr("data-action", name)
    }

    pub fn hx_get(self, url: impl Into<String>) -> Self {
        self.attr("hx-get", url)
    }

    pub fn hx_post(self, url: impl Into<String>) -> Self {
        self.attr("hx-post", url)
    }

    pub fn hx_delete(self, url: impl Into<String>) -> Self {
        self.attr("hx-delete", url)
    }

    pub fn hx_target(self, selector: impl Into<String>) -> Self {
        self.attr("hx-target", selector)
    }

    pub fn hx_swap(self, style: impl Into<String>) -> Self {
        self.attr("hx-swap", style)
    }

    pub fn hx_confirm(self, message: impl Into<String>) -> Self {
        self.attr("hx-confirm", message)
    }

    fn open_tag(&self, tag: &str, fixed: &[(&str, &str)]) -> Markup {
        let mut out = format!("<{tag} class=\"button");
        for class in &self.classes {
            out.push(' ');
            out.push_str(class);
        }
        out.push('"');

        let fixed = fixed.iter().map(|(n, v)| (*n, *v));
        let extra = self.attrs.iter().map(|(n, v)| (n.as_str(), v.as_str()));
        for (name, value) in fixed.chain(extra) {
            write!(out, " {name}=\"").unwrap();
            Escaper::new(&mut out).write_str(value).unwrap();
            out.push('"');
        }

        out.push('>');
        PreEscaped(out)
    }

    fn element(&self, tag: &str, fixed: &[(&str, &str)]) -> Markup {
        html! {
            (self.open_tag(tag, fixed))
            (self.content)
            (PreEscaped(format!("</{tag}>")))
        }
    }
}

impl Render for Button {
    fn render(&self) -> Markup {
        match &self.action {
            Action::Link(href) => self.element("a", &[("href", href)]),
            Action::Post(url) => html! {
                form.button-form method="POST" action=(url) {
                    (self.element("button", &[("type", "submit")]))
                }
            },
            Action::Submit => self.element("button", &[("type", "submit")]),
            Action::Button => self.element("button", &[("type", "button")]),
        }
    }
}
