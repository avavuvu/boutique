use crate::{Source, analyse};

fn run(files: &[(usize, &str, &str)]) -> Result<(String, String, Vec<String>), Vec<String>> {
    let sources: Vec<Source> = files
        .iter()
        .map(|(root, name, text)| Source {
            root: *root,
            name: name.to_string(),
            import: format!("./{}", name.trim_end_matches(".rs")),
            text: text.to_string(),
        })
        .collect();
    analyse("rt", &sources)
}

fn ok(files: &[(usize, &str, &str)]) -> (String, String, Vec<String>) {
    run(files).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

fn setup(types: &str, name: &str) -> String {
    let prefix = format!("export type {name} = ");
    types
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("no {name} in\n{types}"))
        .trim_end_matches(';')
        .to_string()
}

#[test]
fn element_with_refs() {
    let (types, registry, warnings) = ok(&[(0, "a.rs", r##"
        setup!(Toggle);
        fn view() -> Markup {
            html! {
                div bq-setup=(Toggle) {
                    button bq-ref="close-button" {}
                    @if open { p bq-ref="body" {} }
                    @for item in items { li bq-ref="item" {} }
                }
            }
        }
    "##)]);
    assert_eq!(setup(&types, "Toggle"), r#"Setup<"div", { body?: "p"; closeButton: "button"; item: ("li")[] }>"#);
    assert!(registry.contains(r#"tags: ["div"],"#) && registry.contains(r#"many: ["item"],"#), "{registry}");
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn several_setups_on_one_element() {
    let (types, ..) = ok(&[(0, "a.rs", r##"
        setup!(A);
        setup!(B);
        fn view() -> Markup { html! { .box bq-setup={ (A) " " (B) } { span bq-ref="x" {} } } }
    "##)]);
    assert_eq!(setup(&types, "A"), r#"Setup<"div", { x: "span" }>"#);
    assert_eq!(setup(&types, "B"), r#"Setup<"div", { x: "span" }>"#);
}

#[test]
fn plain_function_is_followed_across_files() {
    let (types, _, warnings) = ok(&[
        (0, "a.rs", r##"
            setup!(Menu);
            fn view() -> Markup { html! { nav bq-setup=(Menu) { (item("x")) } } }
        "##),
        (0, "b.rs", r##"
            fn item(name: &str) -> Markup { html! { a bq-ref="link" { (name) } } }
        "##),
    ]);
    assert_eq!(setup(&types, "Menu"), r#"Setup<"nav", { link: "a" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn orphan_reference_warns() {
    let (_, _, warnings) = ok(&[(0, "a.rs", r##"
        fn lonely() -> Markup { html! { a bq-ref="link" {} } }
    "##)]);
    assert_eq!(warnings, [r#"a.rs:2: bq-ref="link" is not inside an element with bq-setup"#]);
}

#[test]
fn nested_html_in_a_splice_keeps_the_scope() {
    let (types, _, warnings) = ok(&[(0, "a.rs", r##"
        setup!(Panel);
        fn view() -> Markup { html! { div bq-setup=(Panel) { (wrap(html! { span bq-ref="label" {} })) } } }
    "##)]);
    assert_eq!(setup(&types, "Panel"), r#"Setup<"div", { label: "span" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn literal_element_tags() {
    let (types, registry, _) = ok(&[(0, "a.rs", r##"
        setup!(Link);
        setup!(Heading);
        setup!(Free);
        fn view() -> Markup {
            html! {
                @element(if link { "a" } else { "button" }) bq-setup=(Link) {}
                @element(match level { 1 => "h1", _ => "h2" }) bq-setup=(Heading) {}
                @element(tag) bq-setup=(Free) {}
            }
        }
    "##)]);
    assert_eq!(setup(&types, "Link"), r#"Setup<"a" | "button">"#);
    assert_eq!(setup(&types, "Heading"), r#"Setup<"h1" | "h2">"#);
    assert_eq!(setup(&types, "Free"), "Setup<string>");
    assert!(registry.contains("\"free\": {\n        tags: [],"), "{registry}");
}

const CARD: &str = r##"
    #[component]
    pub fn card(#[builder(default)] attrs: Attrs, children: Markup) -> Markup {
        html! { section.card (..attrs) { h2 {} (children) } }
    }
"##;

#[test]
fn call_site_setup_goes_to_the_spread_element() {
    let (types, _, warnings) = ok(&[
        (0, "card.rs", CARD),
        (0, "a.rs", r##"
            setup!(Collapse);
            fn view() -> Markup { html! { Card bq-setup=(Collapse) { button bq-ref="toggle" {} } } }
        "##),
    ]);
    assert_eq!(setup(&types, "Collapse"), r#"Setup<"section", { toggle: "button" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn component_setup_owns_the_children() {
    let dialog = r##"
        setup!(Dialog);
        #[component]
        pub fn dialog(children: Markup) -> Markup {
            html! { dialog bq-setup=(Dialog) { form { (children) } } }
        }
    "##;
    let (types, ..) = ok(&[
        (0, "dialog.rs", dialog),
        (0, "a.rs", r##"
            fn one() -> Markup { html! { Dialog { button bq-ref="close" {} } } }
            fn two() -> Markup { html! { Dialog { button bq-ref="close" {} } } }
        "##),
    ]);
    assert_eq!(setup(&types, "Dialog"), r#"Setup<"dialog", { close: "button" }>"#);

    let (types, ..) = ok(&[
        (0, "dialog.rs", dialog),
        (0, "a.rs", r##"
            fn one() -> Markup { html! { Dialog { button bq-ref="close" {} } } }
            fn two() -> Markup { html! { Dialog { p {} } } }
        "##),
    ]);
    assert_eq!(setup(&types, "Dialog"), r#"Setup<"dialog", { close?: "button" }>"#);
}

#[test]
fn call_site_reference_uses_the_component_tags() {
    let (types, _, warnings) = ok(&[
        (1, "button.rs", r##"
            #[component]
            pub fn button(href: Option<&str>, #[builder(default)] attrs: Attrs, children: Markup) -> Markup {
                html! { @element(if href.is_some() { "a" } else { "button" }) .button href=[href] (..attrs) { (children) } }
            }
        "##),
        (0, "a.rs", r##"
            setup!(Form);
            fn view() -> Markup { html! { form bq-setup=(Form) { Button bq-ref="save" { "Save" } } } }
        "##),
    ]);
    assert_eq!(setup(&types, "Form"), r#"Setup<"form", { save: "a" | "button" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn children_pass_through_nested_components() {
    let (types, _, warnings) = ok(&[
        (0, "ui.rs", r##"
            setup!(Inner);
            #[component]
            pub fn inner(#[builder(default)] attrs: Attrs, children: Markup) -> Markup {
                html! { div bq-setup=(Inner) (..attrs) { (children) } }
            }
            #[component]
            pub fn outer(#[builder(default)] attrs: Attrs, children: Markup) -> Markup {
                html! { Inner (..attrs) { aside { (children) } } }
            }
        "##),
        (0, "a.rs", r##"
            setup!(Extra);
            fn view() -> Markup { html! { Outer bq-setup=(Extra) { span bq-ref="x" {} } } }
        "##),
    ]);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(setup(&types, "Inner"), r#"Setup<"div", { x: "span" }>"#);
    assert_eq!(setup(&types, "Extra"), r#"Setup<"div", { x: "span" }>"#);
}

#[test]
fn unreached_component_is_walked_alone() {
    let (types, _, warnings) = ok(&[(1, "input.rs", r##"
        setup!(PasswordToggle);
        #[component]
        pub fn input(name: &str) -> Markup {
            html! { div bq-setup=(PasswordToggle) { button bq-ref="toggle" {} } }
        }
    "##)]);
    assert_eq!(setup(&types, "PasswordToggle"), r#"Setup<"div", { toggle: "button" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn unknown_component_keeps_its_children() {
    let (types, ..) = ok(&[(0, "a.rs", r##"
        setup!(Outside);
        fn view() -> Markup { html! { external::Widget bq-setup=(Outside) { i bq-ref="icon" {} } } }
    "##)]);
    assert_eq!(setup(&types, "Outside"), r#"Setup<string, { icon: "i" }>"#);
}

#[test]
fn component_path_uses_the_last_segment() {
    let (types, ..) = ok(&[
        (0, "card.rs", CARD),
        (0, "a.rs", r##"
            setup!(Collapse);
            fn view() -> Markup { html! { ui::Card bq-setup=(Collapse) {} } }
        "##),
    ]);
    assert_eq!(setup(&types, "Collapse"), r#"Setup<"section">"#);
}

#[test]
fn recursive_component_stops() {
    let (types, ..) = ok(&[(0, "a.rs", r##"
        setup!(Tree);
        #[component]
        pub fn tree(children: Markup) -> Markup {
            html! { ul bq-setup=(Tree) { li bq-ref="node" { Tree { (children) } } } }
        }
        fn view() -> Markup { html! { Tree { b {} } } }
    "##)]);
    assert!(setup(&types, "Tree").starts_with(r#"Setup<"ul""#), "{types}");
}

#[test]
fn same_root_wins_and_ambiguity_warns() {
    let card_b = CARD.replace("section", "article");
    let card_c = CARD.replace("section", "aside");
    let (types, _, warnings) = ok(&[
        (0, "a.rs", r##"
            setup!(Here);
            fn view() -> Markup { html! { Card bq-setup=(Here) {} } }
        "##),
        (0, "card.rs", CARD),
        (1, "b.rs", &card_b),
    ]);
    assert_eq!(setup(&types, "Here"), r#"Setup<"section">"#);
    assert!(warnings.is_empty(), "{warnings:?}");

    let (types, _, warnings) = ok(&[
        (0, "a.rs", r##"
            setup!(Here);
            fn view() -> Markup { html! { Card bq-setup=(Here) {} } }
        "##),
        (1, "b.rs", &card_b),
        (2, "c.rs", &card_c),
    ]);
    assert_eq!(setup(&types, "Here"), "Setup<string>");
    assert_eq!(warnings, ["a.rs:3: `Card` matches more than one function (b.rs, c.rs), so bq_setups does not follow it"]);
}

#[test]
fn errors_are_reported_once() {
    let errors = run(&[(0, "a.rs", r##"
        setup!(Menu);
        #[component]
        pub fn item(children: Markup) -> Markup { html! { a bq-ref="Bad" {} (children) } }
        fn view() -> Markup { html! { nav bq-setup=(Menu) { Item {} Item {} } div bq-setup=(Missing) { i bq-ref="signal" {} } } }
    "##)])
    .unwrap_err();
    assert_eq!(
        errors,
        [
            r#"a.rs:4: bq-ref="Bad" must be lowercase kebab-case, for example bq-ref="close-button""#,
            "a.rs:5: bq-setup=(Missing) has no setup!(Missing) declaration",
        ]
    );
}

#[test]
fn reserved_reference_name() {
    let errors = run(&[(0, "a.rs", r##"
        setup!(Menu);
        fn view() -> Markup { html! { nav bq-setup=(Menu) { i bq-ref="signal" {} } } }
    "##)])
    .unwrap_err();
    assert_eq!(errors, [r#"a.rs:3: bq-ref="signal" is a reserved name"#]);
}
