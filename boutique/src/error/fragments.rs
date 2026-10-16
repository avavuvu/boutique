use maud::Markup;

use super::ErrorMessage;
use crate::{html, htmx::partial};

fn field_error(field: &str, text: &str) -> Markup {
    let id = format!("{field}-error");
    partial(
        &format!("#{id}"),
        html! {
            p.error id=(id) { (text) }
        },
    )
}

pub(crate) fn messages(messages: &[ErrorMessage]) -> Markup {
    html! {
        @for message in messages {
            @if let Some(field) = &message.field {
                (field_error(field, &message.text))
            } @else {
                p { (message.text) }
            }
        }
    }
}
