extern crate self as bq_components;

mod button;
mod input;
mod tag;

pub use button::button;
pub use input::{Kind, input, textarea};
pub use tag::Attrs;

pub use bq_macros::{attrs, component, setup};

pub use bon;
pub use maud;
