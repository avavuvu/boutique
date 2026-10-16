extern crate self as bq_components;

mod attrs;
#[cfg(feature = "ui")]
bq_assets::module!(button);
#[cfg(feature = "ui")]
bq_assets::module!(input);

pub use attrs::{AttrValue, Attrs};
#[cfg(feature = "ui")]
pub use button::{Button, button};
#[cfg(feature = "ui")]
pub use input::{Input, Kind, Textarea, input, textarea};

pub use bq_macros::{attrs, component, html};

#[doc(hidden)]
pub use bon;
pub use maud;

#[doc(hidden)]
pub use bq_assets::FILES;

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
