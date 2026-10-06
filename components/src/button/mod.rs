use maud::Markup;

use crate::{Attrs, component, html};

#[component]
pub fn button(
    #[builder(into)] href: Option<String>,
    #[builder(into)] post: Option<String>,
    #[builder(default)] submit: bool,
    #[builder(default)] disabled: bool,
    #[builder(default)] attrs: Attrs,
    #[builder(default = html! {})] children: Markup,
) -> Markup {
    let link = href.is_some();
    let kind = match (link, submit || post.is_some()) {
        (true, _) => None,
        (false, true) => Some("submit"),
        (false, false) => Some("button"),
    };

    let element = html! {
        @element(if link { "a" } else { "button" })
            .button
            href=[href]
            type=[kind]
            disabled[disabled && !link]
            aria-disabled=[(disabled && link).then_some("true")]
            (..attrs)
        {
            (children)
        }
    };

    match post {
        Some(action) if !link => html! {
            form.button-form method="POST" action=(action) {
                (element)
            }
        },
        _ => element,
    }
}
