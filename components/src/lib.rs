extern crate self as bq_components;

mod attrs;
#[cfg(feature = "ui")]
mod button;
#[cfg(feature = "ui")]
mod input;

pub use attrs::{AttrValue, Attrs};
#[cfg(feature = "ui")]
pub use button::{Button, button};
#[cfg(feature = "ui")]
pub use input::{Input, Kind, Textarea, input, textarea};

pub use bq_macros::{attrs, component, html};

#[doc(hidden)]
pub use bon;
pub use maud;

mod files {
    include!(concat!(env!("OUT_DIR"), "/files.rs"));
}

#[doc(hidden)]
pub use files::FILES;

#[doc(hidden)]
pub mod __private {
    pub use crate::attrs::{class, extend, flag, text, write};

    pub fn tag(name: &'static str) -> &'static str {
        debug_assert!(
            !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "{name:?} is not a valid tag name for @element"
        );
        name
    }
}
