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
            self.__text(name, escaped);
        }
    }

    #[doc(hidden)]
    pub fn __flag(&mut self, name: &'static str) {
        if name == "class" {
            self.class.get_or_insert_default();
        } else if !self.has(name) {
            self.entries.push((name, AttrValue::Flag));
        }
    }

    #[doc(hidden)]
    pub fn __text(&mut self, name: &'static str, escaped: String) {
        if name == "class" {
            self.__class(&escaped);
            return;
        }
        match self.entries.iter_mut().find(|(entry, _)| *entry == name) {
            Some((_, AttrValue::Text(PreEscaped(existing)))) if merges(name) => push_class(existing, &escaped),
            Some(_) => {}
            None => self.entries.push((name, AttrValue::Text(PreEscaped(escaped)))),
        }
    }

    #[doc(hidden)]
    pub fn __class(&mut self, escaped: &str) {
        if !escaped.is_empty() {
            push_class(self.class.get_or_insert_default(), escaped);
        }
    }

    #[doc(hidden)]
    pub fn __extend(&mut self, other: &Attrs) {
        self.__extend_except(other, &[]);
    }

    #[doc(hidden)]
    pub fn __extend_except(&mut self, other: &Attrs, skip: &[&str]) {
        match other.class.as_deref() {
            Some("") => self.__flag("class"),
            Some(class) => self.__class(class),
            None => {}
        }
        for (name, value) in &other.entries {
            match value {
                AttrValue::Text(PreEscaped(text)) if merges(name) => self.__text(name, text.clone()),
                _ if !skip.contains(name) && !self.has(name) => self.entries.push((name, value.clone())),
                _ => {}
            }
        }
    }

    #[doc(hidden)]
    pub fn write_to(&self, out: &mut String) {
        match self.class.as_deref() {
            Some("") => out.push_str(" class"),
            Some(class) => {
                out.push_str(" class=\"");
                out.push_str(class);
                out.push('"');
            }
            None => {}
        }
        for (name, value) in &self.entries {
            out.push(' ');
            out.push_str(name);
            if let AttrValue::Text(PreEscaped(text)) = value {
                out.push_str("=\"");
                out.push_str(text);
                out.push('"');
            }
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
