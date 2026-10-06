use maud::{Markup, PreEscaped, html};

use crate::{
    component,
    tag::{Attrs, Tag},
};

#[component]
pub fn button(
    #[builder(start_fn)] children: Markup,
    #[builder(into)] href: Option<String>,
    #[builder(into)] post: Option<String>,
    #[builder(default, with = || true)] submit: bool,
    #[builder(default, with = || true)] primary: bool,
    #[builder(default, with = || true)] secondary: bool,
    #[builder(default, with = || true)] danger: bool,
    #[builder(default, with = || true)] ghost: bool,
    #[builder(default, with = || true)] small: bool,
    #[builder(default, with = || true)] disabled: bool,
    #[builder(into)] class: Option<String>,
    #[builder(into)] action: Option<String>,
    #[builder(into)] hx_get: Option<String>,
    #[builder(into)] hx_post: Option<String>,
    #[builder(into)] hx_delete: Option<String>,
    #[builder(into)] hx_target: Option<String>,
    #[builder(into)] hx_swap: Option<String>,
    #[builder(into)] hx_confirm: Option<String>,
    #[builder(into)] hx_status_4xx: Option<String>,
    #[builder(into)] hx_status_5xx: Option<String>,
    #[builder(default)] attrs: Attrs<'_>,
) -> Markup {
    let classes = [
        "button",
        if primary { "primary" } else { "" },
        if secondary { "secondary" } else { "" },
        if danger { "danger" } else { "" },
        if ghost { "ghost" } else { "" },
        if small { "small" } else { "" },
        class.as_deref().unwrap_or(""),
    ];

    let (element, tag) = match &href {
        Some(href) => ("a", Tag::open("a").classes(&classes).attr("href", href).maybe("aria-disabled", disabled.then_some("true"))),
        None => {
            let kind = if submit || post.is_some() { "submit" } else { "button" };
            ("button", Tag::open("button").classes(&classes).attr("type", kind).flag("disabled", disabled))
        }
    };

    let open = tag
        .maybe("data-action", action.as_deref())
        .maybe("hx-get", hx_get.as_deref())
        .maybe("hx-post", hx_post.as_deref())
        .maybe("hx-delete", hx_delete.as_deref())
        .maybe("hx-target", hx_target.as_deref())
        .maybe("hx-swap", hx_swap.as_deref())
        .maybe("hx-confirm", hx_confirm.as_deref())
        .maybe("hx-status:4xx", hx_status_4xx.as_deref())
        .maybe("hx-status:5xx", hx_status_5xx.as_deref())
        .attrs(attrs)
        .finish();

    let element = html! {
        (open)
        (children)
        (PreEscaped(format!("</{element}>")))
    };

    match post {
        Some(action) if href.is_none() => html! {
            form.button-form method="POST" action=(action) {
                (element)
            }
        },
        _ => element,
    }
}
