use std::fmt::Write;

use maud::{Escaper, PreEscaped};

#[derive(Clone, Copy, Debug, Default)]
pub struct Attrs<'a>(&'a [(&'static str, Option<&'a str>)]);

impl<'a> Attrs<'a> {
    #[doc(hidden)]
    pub const fn __checked(pairs: &'a [(&'static str, Option<&'a str>)]) -> Self {
        Self(pairs)
    }
}

pub(crate) struct Tag {
    out: String,
}

impl Tag {
    pub(crate) fn open(element: &'static str) -> Self {
        Self { out: format!("<{element}") }
    }

    pub(crate) fn attr(mut self, name: &'static str, value: &str) -> Self {
        write!(self.out, " {name}=\"").expect("writing to a String cannot fail");
        Escaper::new(&mut self.out).write_str(value).expect("writing to a String cannot fail");
        self.out.push('"');
        self
    }

    pub(crate) fn maybe(self, name: &'static str, value: Option<&str>) -> Self {
        match value {
            Some(value) => self.attr(name, value),
            None => self,
        }
    }

    pub(crate) fn flag(mut self, name: &'static str, on: bool) -> Self {
        if on {
            self.out.push(' ');
            self.out.push_str(name);
        }
        self
    }

    pub(crate) fn classes(self, classes: &[&str]) -> Self {
        let joined = classes.iter().filter(|class| !class.is_empty()).copied().collect::<Vec<_>>().join(" ");
        if joined.is_empty() { self } else { self.attr("class", &joined) }
    }

    pub(crate) fn attrs(self, attrs: Attrs<'_>) -> Self {
        attrs.0.iter().fold(self, |tag, &(name, value)| match value {
            Some(value) => tag.attr(name, value),
            None => tag.flag(name, true),
        })
    }

    pub(crate) fn finish(mut self) -> PreEscaped<String> {
        self.out.push('>');
        PreEscaped(self.out)
    }
}
