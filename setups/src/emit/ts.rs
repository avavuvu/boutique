use std::fmt::Write;

use crate::model::TagSet;

pub fn ts_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() || character == '\u{2028}' || character == '\u{2029}' => {
                write!(out, "\\u{:04x}", u32::from(character)).expect("write to a String");
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

pub fn union(tags: &TagSet) -> String {
    if tags.is_open() {
        return "string".into();
    }
    list(tags.iter(), " | ")
}

pub fn list<'a>(items: impl Iterator<Item = &'a String>, separator: &str) -> String {
    items.map(|item| ts_str(item)).collect::<Vec<_>>().join(separator)
}
