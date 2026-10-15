extern crate self as bq_components;

mod attrs;
mod button;
mod input;

pub use attrs::{AttrValue, Attrs};
pub use button::{Button, button};
pub use input::{Input, Kind, Textarea, input, textarea};

pub use bq_macros::{attrs, component, html};

pub use bon;
pub use maud;

pub trait Setup {
    const NAME: &'static str;
}

#[doc(hidden)]
pub mod __private {
    pub use crate::attrs::{push_class, write_attrs};

    pub fn tag(name: &'static str) -> &'static str {
        debug_assert!(
            !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "{name:?} is not a valid tag name for @element"
        );
        name
    }
}
