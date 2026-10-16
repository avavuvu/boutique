use std::fmt::Write;

use maud::PreEscaped;

fn merges(name: &str) -> bool {
    name == "bq-setup" || name == "bq-ref" || name.starts_with("bq-on:")
}

#[derive(Clone, Debug)]
pub enum AttrValue {
    Flag,
    Text(PreEscaped<String>),
}

#[derive(Clone, Debug, Default)]
pub struct Attrs {
    class: Option<String>,
    entries: Vec<(&'static str, AttrValue)>,
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.class.is_none() && self.entries.is_empty()
    }

    pub fn has(&self, name: &str) -> bool {
        if name == "class" {
            return self.class.is_some();
        }
        self.entries.iter().any(|(entry, _)| *entry == name)
    }

    pub fn class(&self) -> Option<PreEscaped<&str>> {
        self.class.as_deref().map(PreEscaped)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &AttrValue)> {
        self.entries.iter().map(|(name, value)| (*name, value))
    }

    pub fn set_default(&mut self, name: &'static str, value: &str) {
        if !self.has(name) {
            let mut escaped = String::with_capacity(value.len());
            maud::Escaper::new(&mut escaped)
                .write_str(value)
                .expect("writing to a String cannot fail");
            text(self, name, PreEscaped(escaped));
        }
    }
}

pub fn flag(attrs: &mut Attrs, name: &'static str) {
    if name == "class" {
        attrs.class.get_or_insert_default();
    } else if !attrs.has(name) {
        attrs.entries.push((name, AttrValue::Flag));
    }
}

pub fn text(attrs: &mut Attrs, name: &'static str, escaped: PreEscaped<String>) {
    if name == "class" {
        class(attrs, PreEscaped(&escaped.0));
        return;
    }
    match attrs.entries.iter_mut().find(|(entry, _)| *entry == name) {
        Some((_, AttrValue::Text(PreEscaped(existing)))) if merges(name) => push_class(existing, &escaped.0),
        Some(_) => {}
        None => attrs.entries.push((name, AttrValue::Text(escaped))),
    }
}

pub fn class(attrs: &mut Attrs, escaped: PreEscaped<&str>) {
    if !escaped.0.is_empty() {
        push_class(attrs.class.get_or_insert_default(), escaped.0);
    }
}

pub fn extend(attrs: &mut Attrs, other: &Attrs, skip: &[&str]) {
    match other.class.as_deref() {
        Some("") => flag(attrs, "class"),
        Some(escaped) => class(attrs, PreEscaped(escaped)),
        None => {}
    }
    for (name, value) in &other.entries {
        match value {
            AttrValue::Text(escaped) if merges(name) => text(attrs, name, escaped.clone()),
            _ if !skip.contains(name) && !attrs.has(name) => attrs.entries.push((name, value.clone())),
            _ => {}
        }
    }
}

pub fn write(attrs: &Attrs, out: &mut String) {
    match attrs.class.as_deref() {
        Some("") => out.push_str(" class"),
        Some(class) => {
            out.push_str(" class=\"");
            out.push_str(class);
            out.push('"');
        }
        None => {}
    }
    for (name, value) in &attrs.entries {
        out.push(' ');
        out.push_str(name);
        if let AttrValue::Text(PreEscaped(text)) = value {
            out.push_str("=\"");
            out.push_str(text);
            out.push('"');
        }
    }
}

fn push_class(class: &mut String, escaped: &str) {
    if escaped.is_empty() {
        return;
    }
    if !class.is_empty() {
        class.push(' ');
    }
    class.push_str(escaped);
}
