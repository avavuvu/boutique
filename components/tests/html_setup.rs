use bq_components::{Setup, component, html, maud::Markup, setup};

setup!(SaveStatus);
setup!(Menu);

#[component]
fn save_status() -> Markup {
    html! { output bq-setup=(SaveStatus) { "Saved" } }
}

mod nested {
    bq_components::setup!(pub DeepThing);
}

#[test]
fn setup_and_component_can_share_a_name() {
    assert_eq!(html! { SaveStatus; }.into_string(), r#"<output bq-setup="save-status">Saved</output>"#);
}

#[test]
fn setup_name_constant() {
    assert_eq!(SaveStatus::NAME, "save-status");
    assert_eq!(<nested::DeepThing as Setup>::NAME, "deep-thing");
}

#[test]
fn several_setups_and_paths() {
    let result = html! { nav bq-setup={ (Menu) " " (nested::DeepThing) } {} };
    assert_eq!(result.into_string(), r#"<nav bq-setup="menu deep-thing"></nav>"#);
}

#[test]
fn setup_on_spread_and_dynamic_elements_and_component_call_sites() {
    let tag = "section";
    let result = html! {
        div bq-setup=(Menu) (..bq_components::Attrs::new()) {}
        @element(tag) bq-setup=(Menu) {}
        bq_components::Button bq-setup=(Menu) { "x" }
    };
    assert_eq!(
        result.into_string(),
        r#"<div bq-setup="menu"></div><section bq-setup="menu"></section><button class="button" type="button" bq-setup="menu">x</button>"#
    );
}
