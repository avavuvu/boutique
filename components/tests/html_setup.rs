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

#[component]
fn menu_panel(#[builder(default)] attrs: bq_components::Attrs, #[builder(default = html! {})] children: Markup) -> Markup {
    html! { nav bq-setup=(Menu) (..attrs) { (children) } }
}

#[test]
fn call_site_setups_merge_with_the_component_setup() {
    let result = html! { MenuPanel bq-setup=(SaveStatus) {} };
    assert_eq!(result.into_string(), r#"<nav bq-setup="menu save-status"></nav>"#);
}

#[test]
fn spread_setups_merge_with_static_setups() {
    let extra = bq_components::attrs!("bq-setup" = "other", title = "t");
    let result = html! {
        div bq-setup=(Menu) (..extra) {}
        div bq-setup={ (Menu) " " (SaveStatus) } (..bq_components::attrs!("bq-setup" = "x")) (..bq_components::attrs!("bq-setup" = "y")) {}
        div (..bq_components::attrs!("bq-setup" = "only")) {}
    };
    assert_eq!(
        result.into_string(),
        r#"<div bq-setup="menu other" title="t"></div><div bq-setup="menu save-status x y"></div><div bq-setup="only"></div>"#
    );
}

#[test]
fn call_site_setups_merge_with_spread_setups() {
    let extra = bq_components::attrs!("bq-setup" = "other");
    let result = html! { MenuPanel bq-setup=(SaveStatus) (..extra) {} };
    assert_eq!(result.into_string(), r#"<nav bq-setup="menu save-status other"></nav>"#);
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
