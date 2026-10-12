use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Element,
    Window,
    Document,
}

impl Target {
    pub fn suffix(&self) -> &'static str {
        match self {
            Target::Element => "",
            Target::Window => "; target: \"window\"",
            Target::Document => "; target: \"document\"",
        }
    }
}

#[derive(Clone, Debug)]
pub struct On {
    pub attribute: String,
    pub event: String,
    pub target: Target,
    pub setup: Option<String>,
    pub handler: String,
}

const EVENT_MODIFIERS: [&str; 6] = ["prevent", "stop", "self", "once", "passive", "capture"];
const KEYS: [&str; 7] = ["enter", "escape", "tab", "space", "delete", "up", "down"];
const SYSTEM: [&str; 6] = ["ctrl", "alt", "shift", "meta", "mod", "exact"];

const KEYBOARD: [&str; 3] = ["keydown", "keyup", "keypress"];
const BUTTON: [&str; 8] = ["click", "dblclick", "mousedown", "mouseup", "contextmenu", "auxclick", "pointerdown", "pointerup"];
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

fn single_key(modifier: &str) -> bool {
    modifier.len() == 1 && modifier.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

pub fn parse(attribute: &str, value: &str) -> Result<On, String> {
    let rest = attribute.strip_prefix("bq-on:").unwrap_or_default();
    let mut parts = rest.split('|');
    let event = parts.next().unwrap_or_default().to_string();
    let modifiers: Vec<&str> = parts.collect();

    if event.is_empty() {
        return Err(format!("{attribute} needs an event name, for example bq-on:click=\"save\""));
    }
    if event.chars().any(|character| character.is_ascii_uppercase()) {
        return Err(format!("{attribute}: event names must be lowercase, because HTML makes attribute names lowercase"));
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
        return Err(format!("{attribute}: `passive` and `prevent` cannot be used together, because a passive listener cannot prevent the default"));
    }
    if seen.contains("self") && target != Target::Element {
        return Err(format!("{attribute}: `self` cannot be used with `window` or `document`"));
    }

    let (setup, handler) = match value.split_once('.') {
        Some((setup, handler)) => (Some(setup.to_string()), handler.to_string()),
        None => (None, value.to_string()),
    };

    let valid_handler = handler.chars().next().is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
        && handler.chars().all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '$');
    if !valid_handler {
        return Err(format!("{attribute}=\"{value}\": the handler must be a JavaScript name, for example bq-on:click=\"save\""));
    }
    if let Some(setup) = &setup {
        let valid_setup = setup.chars().next().is_some_and(|first| first.is_ascii_lowercase())
            && !setup.ends_with('-')
            && !setup.contains("--")
            && setup.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-');
        if !valid_setup {
            return Err(format!("{attribute}=\"{value}\": the setup name must be kebab-case, for example \"dirty-form.save\""));
        }
    }

    Ok(On { attribute: attribute.to_string(), event, target, setup, handler })
}
