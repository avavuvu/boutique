use std::fmt::Write;

use maud::PreEscaped;

fn merges(name: &str) -> bool {
    name == "bq-setup" || name.starts_with("bq-on:")
}

#[derive(Clone, Debug)]
pub enum AttrValue {
    Flag,
    Text(PreEscaped<String>),
}

#[derive(Clone, Debug, Default)]
pub struct Attrs {
    class: String,
    entries: Vec<(&'static str, AttrValue)>,
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.class.is_empty() && self.entries.is_empty()
    }

    pub fn has(&self, name: &str) -> bool {
        if name == "class" {
            return !self.class.is_empty();
        }
        self.entries.iter().any(|(entry, _)| *entry == name)
    }

    pub fn class(&self) -> Option<PreEscaped<&str>> {
        (!self.class.is_empty()).then_some(PreEscaped(self.class.as_str()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &AttrValue)> {
        self.entries.iter().map(|(name, value)| (*name, value))
    }

    pub fn set_default(&mut self, name: &'static str, value: &str) {
        if !self.has(name) {
            let mut escaped = String::with_capacity(value.len());
            maud::Escaper::new(&mut escaped).write_str(value).expect("writing to a String cannot fail");
            self.__text(name, escaped);
        }
    }

    #[doc(hidden)]
    pub fn __flag(&mut self, name: &'static str) {
        if name != "class" && !self.has(name) {
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
    pub fn __value(&self, name: &str) -> Option<&str> {
        self.entries.iter().find_map(|(entry, value)| match value {
            AttrValue::Text(PreEscaped(text)) if *entry == name => Some(text.as_str()),
            _ => None,
        })
    }

    #[doc(hidden)]
    pub fn __class(&mut self, escaped: &str) {
        push_class(&mut self.class, escaped);
    }


    #[doc(hidden)]
    pub fn __extend(&mut self, other: &Attrs) {
        self.__extend_except(other, &[]);
    }

    #[doc(hidden)]
    pub fn __extend_except(&mut self, other: &Attrs, skip: &[&str]) {
        self.__class(&other.class);
        for (name, value) in &other.entries {
            if skip.contains(name) {
                continue;
            }
            match value {
                AttrValue::Text(PreEscaped(text)) if merges(name) => self.__text(name, text.clone()),
                _ if !self.has(name) => self.entries.push((name, value.clone())),
                _ => {}
            }
        }
    }
}

pub fn push_class(class: &mut String, escaped: &str) {
    if escaped.is_empty() {
        return;
    }
    if !class.is_empty() {
        class.push(' ');
    }
    class.push_str(escaped);
}

pub fn write_attrs(out: &mut String, attrs: &Attrs, skip: &[&str]) {
    for (name, value) in &attrs.entries {
        if skip.contains(name) {
            continue;
        }
        out.push(' ');
        out.push_str(name);
        if let AttrValue::Text(PreEscaped(text)) = value {
            out.push_str("=\"");
            out.push_str(text);
            out.push('"');
        }
    }
}
