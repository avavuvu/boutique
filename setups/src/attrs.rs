use bq_html::{
    ast::{Attribute, AttributeType, Markup},
    conventions::{BQ_ON, BQ_REF, BQ_SETUP, kebab},
};
use heck::{ToKebabCase, ToLowerCamelCase};
use std::collections::BTreeSet;

use crate::diagnostics::{Loc, Sink};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Element,
    Window,
    Document,
}

impl Target {
    pub fn name(self) -> Option<&'static str> {
        match self {
            Target::Element => None,
            Target::Window => Some("window"),
            Target::Document => Some("document"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ElementAttrs {
    pub setups: Vec<String>,
    pub refs: Vec<RefAttr>,
    pub ons: Vec<OnAttr>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RefAttr {
    pub setup: Option<String>,
    pub name: String,
    pub text: String,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnAttr {
    pub attribute: String,
    pub event: String,
    pub target: Target,
    pub setup: Option<String>,
    pub handler: String,
    pub key: String,
    pub loc: Loc,
}

pub struct Lexical<'a> {
    pub name: &'a str,
    pub component: bool,
    pub setup: Option<&'a str>,
}

const EVENT_MODIFIERS: [&str; 6] = ["prevent", "stop", "self", "once", "passive", "capture"];
const KEYS: [&str; 7] = ["enter", "escape", "tab", "space", "delete", "up", "down"];
const SYSTEM: [&str; 6] = ["ctrl", "alt", "shift", "meta", "mod", "exact"];

const KEYBOARD: [&str; 3] = ["keydown", "keyup", "keypress"];
const BUTTON: [&str; 8] = [
    "click",
    "dblclick",
    "mousedown",
    "mouseup",
    "contextmenu",
    "auxclick",
    "pointerdown",
    "pointerup",
];
const POINTER: [&str; 15] = [
    "mousemove",
    "mouseover",
    "mouseout",
    "mouseenter",
    "mouseleave",
    "pointermove",
    "pointerover",
    "pointerout",
    "pointerenter",
    "pointerleave",
    "wheel",
    "touchstart",
    "touchend",
    "touchmove",
    "touchcancel",
];

pub fn element(attrs: &[Attribute], loc: &Loc, function: Option<Lexical>, sink: &mut Sink) -> ElementAttrs {
    let mut parsed = ElementAttrs::default();
    for attr in attrs {
        let Attribute::Named { name, attr_type } = attr else { continue };
        match name.to_string().as_str() {
            BQ_SETUP => {
                if let Some(setup) = setup(attr_type, loc, function.as_ref(), sink) {
                    parsed.setups.push(setup);
                }
            }
            BQ_REF => parsed.refs.extend(refs(attr_type, loc, sink)),
            attribute if attribute.starts_with(BQ_ON) => {
                let value = match attr_type {
                    AttributeType::Normal {
                        value: Markup::Lit(lit), ..
                    } => lit.lit.value(),
                    AttributeType::Empty(None) => String::new(),
                    _ => {
                        sink.error(
                            loc,
                            format!("{attribute} needs a fixed string, for example {attribute}=\"save\""),
                            None,
                        );
                        continue;
                    }
                };
                match parse_on(attribute, &value, loc) {
                    Ok(ons) => parsed.ons.extend(ons),
                    Err(message) => sink.error(loc, message, None),
                }
            }
            _ => {}
        }
    }
    parsed
}

fn setup(value: &AttributeType, loc: &Loc, function: Option<&Lexical>, sink: &mut Sink) -> Option<String> {
    if !matches!(value, AttributeType::Empty(None)) {
        sink.error(
            loc,
            "bq-setup does not take a value; write a bare bq-setup inside the #[component] function whose setup it attaches",
            None,
        );
        return None;
    }
    if let Some(Lexical { setup: Some(name), .. }) = function {
        return Some(name.to_string());
    }
    let place = match function {
        Some(function) if !function.component => format!("in `{}`, which is not a #[component]", function.name),
        Some(function) => format!("in `{}`, which bq_setups did not recognise as having a setup", function.name),
        None => "outside a #[component] function".to_string(),
    };
    sink.error(
        loc,
        format!(
            "bare bq-setup {place}; bare bq-setup can only be used directly inside a #[component] function, where it attaches that component's setup"
        ),
        None,
    );
    None
}

fn refs(value: &AttributeType, loc: &Loc, sink: &mut Sink) -> Vec<RefAttr> {
    let AttributeType::Normal {
        value: Markup::Lit(lit), ..
    } = value
    else {
        sink.error(loc, "bq-ref needs a fixed string, for example bq-ref=\"select\"", None);
        return Vec::new();
    };

    let value = lit.lit.value();
    if value.split_whitespace().next().is_none() {
        sink.error(loc, "bq-ref needs a name, for example bq-ref=\"select\"", None);
    }

    let mut names = Vec::new();
    for text in value.split_whitespace() {
        let (setup, name) = match text.split_once('.') {
            Some((setup, name)) => (Some(setup), name),
            None => (None, text),
        };
        if !kebab(name) || setup.is_some_and(|setup| !kebab(setup)) {
            sink.error(
                loc,
                format!(
                    "bq-ref=\"{text}\" must be lowercase kebab-case, for example bq-ref=\"close-button\" or bq-ref=\"menu.close-button\""
                ),
                None,
            );
            continue;
        }
        names.push(RefAttr {
            setup: setup.map(str::to_string),
            name: name.to_lower_camel_case(),
            text: text.to_string(),
            loc: loc.clone(),
        });
    }
    names
}

fn single_key(modifier: &str) -> bool {
    modifier.len() == 1
        && modifier
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

fn parse_on(attribute: &str, value: &str, loc: &Loc) -> Result<Vec<OnAttr>, String> {
    let rest = attribute.strip_prefix(BQ_ON).unwrap_or_default();
    let mut parts = rest.split('|');
    let event = parts.next().unwrap_or_default().to_string();
    let modifiers: Vec<&str> = parts.collect();

    if event.is_empty() {
        return Err(format!("{attribute} needs an event name, for example bq-on:click=\"save\""));
    }
    if event.chars().any(|character| character.is_ascii_uppercase()) {
        return Err(format!(
            "{attribute}: event names must be lowercase, because HTML makes attribute names lowercase"
        ));
    }

    let keyboard = KEYBOARD.contains(&event.as_str());
    let button = BUTTON.contains(&event.as_str());
    let pointer = button || POINTER.contains(&event.as_str());

    let mut seen = BTreeSet::new();
    let mut target = Target::Element;
    for &modifier in &modifiers {
        if !seen.insert(modifier) {
            return Err(format!("{attribute}: the modifier `{modifier}` is given two times"));
        }
        let allowed = match modifier {
            "window" | "document" => {
                if target != Target::Element {
                    return Err(format!("{attribute}: use only one of `window` and `document`"));
                }
                target = if modifier == "window" { Target::Window } else { Target::Document };
                true
            }
            _ if EVENT_MODIFIERS.contains(&modifier) => true,
            "left" | "right" => keyboard || button,
            "middle" => button,
            _ if KEYS.contains(&modifier) || single_key(modifier) => keyboard,
            _ if SYSTEM.contains(&modifier) => keyboard || pointer,
            _ => {
                return Err(format!(
                    "{attribute}: `{modifier}` is not a modifier. Use prevent, stop, self, once, passive, capture, window, document, \
                     a key (enter, escape, tab, space, delete, up, down, left, right, a-z, 0-9), \
                     ctrl, alt, shift, meta, mod, exact, or a mouse button (left, middle, right)"
                ));
            }
        };
        if !allowed {
            return Err(format!("{attribute}: `{modifier}` cannot be used with the `{event}` event"));
        }
    }

    if seen.contains("passive") && seen.contains("prevent") {
        return Err(format!(
            "{attribute}: `passive` and `prevent` cannot be used together, because a passive listener cannot prevent the default"
        ));
    }
    if seen.contains("self") && target != Target::Element {
        return Err(format!("{attribute}: `self` cannot be used with `window` or `document`"));
    }

    let tokens: Vec<&str> = value.split_whitespace().collect();
    if tokens.is_empty() {
        if seen.contains("prevent") || seen.contains("stop") {
            return Ok(Vec::new());
        }
        return Err(format!(
            "{attribute} needs a handler, for example {attribute}=\"save\". Only `prevent` and `stop` can be used without a handler"
        ));
    }
    tokens
        .into_iter()
        .map(|token| handler(attribute, token, &event, target, loc))
        .collect()
}

fn handler(attribute: &str, value: &str, event: &str, target: Target, loc: &Loc) -> Result<OnAttr, String> {
    let (setup, handler) = match value.split_once('.') {
        Some((setup, handler)) => (Some(setup.to_string()), handler.to_string()),
        None => (None, value.to_string()),
    };

    if !kebab(&handler) {
        let example = match &setup {
            Some(setup) => format!("{setup}.{}", handler.to_kebab_case()),
            None => handler.to_kebab_case(),
        };
        return Err(format!(
            "{attribute}=\"{value}\": the handler must be kebab-case, for example {attribute}=\"{example}\""
        ));
    }
    if handler == "disconnect" {
        return Err(format!(
            "{attribute}=\"{value}\": `disconnect` is reserved for the code that runs when the setup stops"
        ));
    }
    if let Some(setup) = &setup
        && !kebab(setup)
    {
        return Err(format!(
            "{attribute}=\"{value}\": the setup name must be kebab-case, for example \"dirty-form.save\""
        ));
    }

    let key = handler.to_lower_camel_case();
    Ok(OnAttr {
        attribute: attribute.to_string(),
        event: event.to_string(),
        target,
        setup,
        handler,
        key,
        loc: loc.clone(),
    })
}
