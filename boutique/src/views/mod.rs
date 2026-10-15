mod base;
mod error;
mod head;

pub use base::base;
pub use error::{back_url, error_content, error_page};
pub use head::{Head, Metadata, OgType};
