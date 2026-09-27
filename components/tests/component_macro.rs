use bq_components::component;
use bq_components::maud::{Markup, Render, html};

#[component]
fn greeting(#[builder(start_fn)] name: &str, #[builder(default)] loud: bool) -> Markup {
    html! {
        p.loud[loud] { "Hello, " (name) }
    }
}

#[test]
fn builder_renders_and_is_spliceable() {
    let inline = html! { div { (greeting("Ava")) } }.into_string();
    assert_eq!(inline, r#"<div><p class="">Hello, Ava</p></div>"#);

    let built = greeting("Ava").loud(true).build().into_string();
    assert_eq!(built, r#"<p class="loud">Hello, Ava</p>"#);

    let rendered = greeting("Ava").loud(true).render().into_string();
    assert_eq!(rendered, built);
}
