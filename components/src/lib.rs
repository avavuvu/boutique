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

pub use bon;
pub use maud;

include!(concat!(env!("OUT_DIR"), "/files.rs"));

pub trait Setup {
    const NAME: &'static str;
}

#[doc(hidden)]
pub mod __private {

    pub fn tag(name: &'static str) -> &'static str {
        debug_assert!(
            !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "{name:?} is not a valid tag name for @element"
        );
        name
    }
}
