use bq_components::{Attrs, attrs, component, html, maud::Markup};

#[component]
fn panel(attrs: Attrs, #[default = html! {}] children: Markup) -> Markup {
    html! { section bq-on:click="own" (..attrs) { (children) } }
}

#[test]
fn plain_event_attribute() {
    let result = html! { button bq-on:click="save" {} };
    assert_eq!(result.into_string(), r#"<button bq-on:click="save"></button>"#);
}

#[test]
fn modifiers_and_targets() {
    let result = html! {
        input bq-on:keydown|enter|prevent="next" bq-on:pointerdown|document="outside";
    };
    assert_eq!(
        result.into_string(),
        r#"<input bq-on:keydown|enter|prevent="next" bq-on:pointerdown|document="outside">"#
    );
}

#[test]
fn keyword_modifiers() {
    let result = html! { div bq-on:click|self|once="a" bq-on:keydown|mod|s="b" {} };
    assert_eq!(
        result.into_string(),
        r#"<div bq-on:click|self|once="a" bq-on:keydown|mod|s="b"></div>"#
    );
}

#[test]
fn event_names_with_colons() {
    let result = html! { div bq-on:htmx:before:request|document="flush" {} };
    assert_eq!(result.into_string(), r#"<div bq-on:htmx:before:request|document="flush"></div>"#);
}

#[test]
fn prefixed_handler_value() {
    let result = html! { form bq-on:input="dirty-form.update" {} };
    assert_eq!(result.into_string(), r#"<form bq-on:input="dirty-form.update"></form>"#);
}

#[test]
fn same_event_with_different_modifiers() {
    let result = html! { div bq-on:keydown|enter="a" bq-on:keydown|escape="b" {} };
    assert_eq!(
        result.into_string(),
        r#"<div bq-on:keydown|enter="a" bq-on:keydown|escape="b"></div>"#
    );
}

#[test]
fn through_spread_and_components() {
    let extra = attrs!("bq-on:keydown|escape" = "close");
    let result = html! {
        p (..extra) {}
        Panel bq-on:click="caller" bq-on:keydown|enter="go" {}
    };
    assert_eq!(
        result.into_string(),
        r#"<p bq-on:keydown|escape="close"></p><section bq-on:click="own caller" bq-on:keydown|enter="go"></section>"#
    );
}

#[test]
fn modifiers_without_a_handler() {
    let result = html! { a bq-on:mousedown|prevent {} };
    assert_eq!(result.into_string(), r#"<a bq-on:mousedown|prevent></a>"#);
}

#[test]
fn refs_merge_like_class() {
    let extra = attrs!("bq-ref" = "block-button");
    let result = html! { button bq-ref="button" (..extra) {} };
    assert_eq!(result.into_string(), r#"<button bq-ref="button block-button"></button>"#);
}

#[test]
fn same_attribute_merges_on_one_element() {
    let result = html! { input bq-on:input="a" type="text" bq-on:input="menu.b"; };
    assert_eq!(result.into_string(), r#"<input bq-on:input="a menu.b" type="text">"#);
}

#[test]
fn spread_merges_after_the_static_value() {
    let extra = attrs!("bq-on:input" = "preview.schedule", "bq-on:change" = "other");
    let more = attrs!("bq-on:input" = "third");
    let result = html! { form bq-on:input="dirty-form.update" method="post" (..extra) (..more) {} };
    assert_eq!(
        result.into_string(),
        r#"<form bq-on:input="dirty-form.update preview.schedule third" method="post" bq-on:change="other"></form>"#
    );
}
