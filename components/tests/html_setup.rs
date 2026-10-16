use bq_components::{Attrs, component, html, maud::Markup};

#[component]
fn save_status() -> Markup {
    html! { output bq-setup { "Saved" } }
}

#[component(setup = "menu")]
fn menu_nav(attrs: Attrs) -> Markup {
    html! { nav bq-setup (..attrs) {} }
}

mod nested {
    use bq_components::{component, html, maud::Markup};

    #[component]
    pub fn deep_thing() -> Markup {
        html! { super::MenuNav bq-setup; }
    }
}

#[component(setup = "menu")]
fn menu_panel(#[default] attrs: bq_components::Attrs, #[default = html! {}] children: Markup) -> Markup {
    html! { nav bq-setup (..attrs) { (children) } }
}

#[component(setup = "save-status")]
fn save_status_menu() -> Markup {
    html! { MenuPanel bq-setup {} }
}

#[component(setup = "save-status")]
fn save_status_panel(attrs: Attrs) -> Markup {
    html! { MenuPanel bq-setup (..attrs) {} }
}

#[component(setup = "menu")]
fn menu_box(attrs: Attrs) -> Markup {
    html! { div bq-setup (..attrs) {} }
}

#[component(setup = "menu")]
fn menu_box_xy(attrs: Attrs) -> Markup {
    html! { div bq-setup (..attrs) (..bq_components::attrs!("bq-setup" = "x")) (..bq_components::attrs!("bq-setup" = "y")) {} }
}

#[component(setup = "save-status")]
fn save_status_box() -> Markup {
    html! { MenuBoxXy bq-setup; }
}

#[component(setup = "menu")]
fn menu_elements() -> Markup {
    let tag = "section";
    html! {
        div bq-setup (..bq_components::Attrs::new()) {}
        @element(tag) bq-setup {}
        bq_components::Button bq-setup { "x" }
    }
}

#[test]
fn call_site_setups_merge_with_the_component_setup() {
    let result = html! { SaveStatusMenu; };
    assert_eq!(result.into_string(), r#"<nav bq-setup="menu save-status"></nav>"#);
}

#[test]
fn spread_setups_merge_with_static_setups() {
    let extra = bq_components::attrs!("bq-setup" = "other", title = "t");
    let result = html! {
        MenuBox (..extra);
        SaveStatusBox;
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
    let result = html! { SaveStatusPanel (..extra); };
    assert_eq!(result.into_string(), r#"<nav bq-setup="menu save-status other"></nav>"#);
}

#[test]
fn setup_and_component_can_share_a_name() {
    assert_eq!(
        html! { SaveStatus; }.into_string(),
        r#"<output bq-setup="save-status">Saved</output>"#
    );
}

#[test]
fn several_setups_and_paths() {
    let result = html! { nested::DeepThing; };
    assert_eq!(result.into_string(), r#"<nav bq-setup="menu deep-thing"></nav>"#);
}

#[test]
fn setup_on_spread_and_dynamic_elements_and_component_call_sites() {
    let result = html! { MenuElements; };
    assert_eq!(
        result.into_string(),
        r#"<div bq-setup="menu"></div><section bq-setup="menu"></section><button class="button" type="button" bq-setup="menu">x</button>"#
    );
}
