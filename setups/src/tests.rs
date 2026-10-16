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
    let (types, registry, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "toggle")]
        fn view() -> Markup {
            html! {
                div bq-setup {
                    button bq-ref="close-button" {}
                    @if open { p bq-ref="body" {} }
                    @for item in items { li bq-ref="item" {} }
                }
            }
        }
    "##,
    )]);
    assert_eq!(
        setup(&types, "Toggle"),
        r#"Setup<"div", { body?: "p"; closeButton: "button"; item: ("li")[] }>"#
    );
    assert!(
        registry.contains(r#"tags: ["div"],"#) && registry.contains(r#"many: ["item"],"#),
        "{registry}"
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn several_setups_on_one_element() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component] pub fn a(attrs: Attrs, children: Markup) -> Markup { html! { .box bq-setup (..attrs) { (children) } } }
        #[component(setup = "b")]
        fn view() -> Markup { html! { A bq-setup { span bq-ref="a.x b.y" {} } } }
    "##,
    )]);
    assert_eq!(setup(&types, "A"), r#"Setup<"div", { x: "span" }>"#);
    assert_eq!(setup(&types, "B"), r#"Setup<"div", { y: "span" }>"#);

    let errors = run(&[(
        0,
        "a.rs",
        r##"
        #[component] pub fn a(attrs: Attrs, children: Markup) -> Markup { html! { .box bq-setup (..attrs) { (children) } } }
        #[component(setup = "b")]
        fn view() -> Markup { html! { A bq-setup { span bq-ref="x" {} } } }
    "##,
    )])
    .unwrap_err();
    assert_eq!(
        errors,
        [r#"a.rs:4: bq-ref="x" is inside more than one setup (a, b); write the setup name, for example bq-ref="a.x""#]
    );
}

const MENU: &str = r##"
    #[component]
    pub fn menu(attrs: Attrs, children: Markup) -> Markup {
        html! {
            div.menu bq-setup {
                button bq-ref="button" bq-on:click="toggle" (..attrs) {}
                div bq-ref="panel" bq-on:mousedown|prevent bq-on:click="choose" { (children) }
            }
        }
    }
"##;

#[test]
fn a_name_without_a_prefix_goes_to_the_nearest_setup() {
    let (types, registry, warnings) = ok(&[
        (0, "menu.rs", MENU),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "toolbar")]
            fn view() -> Markup {
                html! {
                    div bq-setup {
                        Menu bq-ref="toolbar.block-button" {
                            @for block in blocks { button bq-ref="toolbar.block-option" bq-on:click="toolbar.set-block" {} }
                            b bq-ref="item" {}
                        }
                    }
                }
            }
        "##,
        ),
    ]);
    assert_eq!(
        setup(&types, "Menu"),
        r#"Setup<"div", { button: "button"; item: "b"; panel: "div" }, { choose: { event: "click"; tag: "div" }; toggle: { event: "click"; tag: "button" } }>"#
    );
    assert_eq!(
        setup(&types, "Toolbar"),
        r#"Setup<"div", { blockButton: "button"; blockOption: ("button")[] }, { setBlock: { event: "click"; tag: "button" } }>"#
    );
    assert!(
        registry.contains("\"menu\": {\n        tags: [\"div\"],\n        many: [],\n"),
        "{registry}"
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn a_followed_helper_is_not_an_orphan() {
    let (types, _, warnings) = ok(&[
        (0, "menu.rs", MENU),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "toolbar")]
            fn view() -> Markup { html! { div bq-setup { Menu { (option()) } } } }
            fn option() -> Markup { html! { b bq-ref="toolbar.option" bq-on:click="toolbar.pick" {} } }
        "##,
        ),
    ]);
    assert_eq!(
        setup(&types, "Toolbar"),
        r#"Setup<"div", { option: "b" }, { pick: { event: "click"; tag: "b" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn markup_in_component_arguments_is_walked_where_it_is_spliced() {
    let (types, _, warnings) = ok(&[
        (
            0,
            "label.rs",
            r##"
            #[component]
            pub fn labelled(label: Markup, children: Markup) -> Markup {
                html! { div bq-setup { button { (label) } (children) } }
            }
        "##,
        ),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "toolbar")]
            fn view() -> Markup {
                html! { div bq-setup { Labelled(label = html! { span bq-ref="toolbar.current" {} i bq-ref="own" {} }) {} } }
            }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Toolbar"), r#"Setup<"div", { current: "span" }>"#);
    assert_eq!(setup(&types, "Labelled"), r#"Setup<"div", { own: "i" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn markup_in_closures_and_branches_is_many_or_optional() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "list")]
        fn view() -> Markup {
            html! {
                ul bq-setup {
                    (items.iter().map(|item| html! { li bq-ref="item" {} }).collect::<Markup>())
                    (if open { html! { p bq-ref="note" {} } } else { html! {} })
                }
            }
        }
    "##,
    )]);
    assert_eq!(setup(&types, "List"), r#"Setup<"ul", { item: ("li")[]; note?: "p" }>"#);
}

#[test]
fn markup_in_a_let_binding_is_walked_where_it_is_spliced() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "card")]
        fn view() -> Markup {
            let title = html! { h2 bq-ref="title" {} };
            html! { section bq-setup { (title) } }
        }
    "##,
    )]);
    assert_eq!(setup(&types, "Card"), r#"Setup<"section", { title: "h2" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn markup_in_a_let_binding_that_is_never_spliced_is_walked() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "clue")]
        fn clue_row(as_partial: bool) -> Markup {
            let row = html! { li bq-setup { (body()) button bq-on:click="save" {} } };
            if as_partial { partial("#clue", row) } else { row }
        }
        fn body() -> Markup {
            let field = html! { textarea bq-ref="body" {} };
            field
        }
    "##,
    )]);
    assert_eq!(
        setup(&types, "Clue"),
        r#"Setup<"li", { body: "textarea" }, { save: { event: "click"; tag: "button" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn prefixed_ref_gets_the_flags_of_its_owner() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component] fn inner(children: Markup) -> Markup { html! { div bq-setup { (children) } } }
        #[component(setup = "outer")]
        fn view() -> Markup {
            html! {
                div bq-setup {
                    @for row in rows {
                        Inner { i bq-ref="outer.icon own" {} }
                    }
                }
            }
        }
    "##,
    )]);
    assert_eq!(setup(&types, "Outer"), r#"Setup<"div", { icon: ("i")[] }>"#);
    assert_eq!(setup(&types, "Inner"), r#"Setup<"div", { own: "i" }>"#);
}

#[test]
fn handler_without_a_value_needs_prevent_or_stop() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "menu")]
        fn view() -> Markup { html! { nav bq-setup { a bq-on:mousedown|prevent {} b bq-on:click|stop {} } } }
    "##,
    )]);
    assert_eq!(setup(&types, "Menu"), r#"Setup<"nav">"#);

    let errors = run(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "menu")]
        fn view() -> Markup { html! { nav bq-setup { a bq-on:mousedown|once {} } } }
    "##,
    )])
    .unwrap_err();
    assert!(
        errors[0].contains("Only `prevent` and `stop` can be used without a handler"),
        "{errors:?}"
    );
}

#[test]
fn plain_function_is_followed_across_files() {
    let (types, _, warnings) = ok(&[
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "menu")]
            fn view() -> Markup { html! { nav bq-setup { (item("x")) } } }
        "##,
        ),
        (
            0,
            "b.rs",
            r##"
            fn item(name: &str) -> Markup { html! { a bq-ref="link" { (name) } } }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Menu"), r#"Setup<"nav", { link: "a" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn orphan_reference_is_an_error() {
    let errors = run(&[(
        0,
        "a.rs",
        r##"
        fn lonely() -> Markup { html! { a bq-ref="link" {} } }
    "##,
    )])
    .unwrap_err();
    assert_eq!(errors, [r#"a.rs:2: bq-ref="link" is not inside an element with bq-setup"#]);
}

#[test]
fn nested_html_in_a_splice_keeps_the_scope() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "panel")]
        fn view() -> Markup { html! { div bq-setup { (wrap(html! { span bq-ref="label" {} })) } } }
    "##,
    )]);
    assert_eq!(setup(&types, "Panel"), r#"Setup<"div", { label: "span" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn literal_element_tags() {
    let (types, registry, _) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component]
        fn link() -> Markup { html! { @element(if link { "a" } else { "button" }) bq-setup {} } }
        #[component]
        fn heading() -> Markup { html! { @element(match level { 1 => "h1", _ => "h2" }) bq-setup {} } }
        #[component]
        fn free() -> Markup { html! { @element(tag) bq-setup {} } }
    "##,
    )]);
    assert_eq!(setup(&types, "Link"), r#"Setup<"a" | "button">"#);
    assert_eq!(setup(&types, "Heading"), r#"Setup<"h1" | "h2">"#);
    assert_eq!(setup(&types, "Free"), "Setup<string>");
    assert!(registry.contains("\"free\": {\n        tags: [],"), "{registry}");
}

const CARD: &str = r##"
    #[component]
    pub fn card(attrs: Attrs, children: Markup) -> Markup {
        html! { section.card (..attrs) { h2 {} (children) } }
    }
"##;

#[test]
fn call_site_setup_goes_to_the_spread_element() {
    let (types, _, warnings) = ok(&[
        (0, "card.rs", CARD),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "collapse")]
            fn view() -> Markup { html! { Card bq-setup { button bq-ref="toggle" {} } } }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Collapse"), r#"Setup<"section", { toggle: "button" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn component_setup_owns_the_children() {
    let dialog = r##"
        #[component]
        pub fn dialog(children: Markup) -> Markup {
            html! { dialog bq-setup { form { (children) } } }
        }
    "##;
    let (types, ..) = ok(&[
        (0, "dialog.rs", dialog),
        (
            0,
            "a.rs",
            r##"
            fn one() -> Markup { html! { Dialog { button bq-ref="close" {} } } }
            fn two() -> Markup { html! { Dialog { button bq-ref="close" {} } } }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Dialog"), r#"Setup<"dialog", { close: "button" }>"#);

    let (types, ..) = ok(&[
        (0, "dialog.rs", dialog),
        (
            0,
            "a.rs",
            r##"
            fn one() -> Markup { html! { Dialog { button bq-ref="close" {} } } }
            fn two() -> Markup { html! { Dialog { p {} } } }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Dialog"), r#"Setup<"dialog", { close?: "button" }>"#);
}

#[test]
fn call_site_reference_uses_the_component_tags() {
    let (types, _, warnings) = ok(&[
        (
            1,
            "button.rs",
            r##"
            #[component]
            pub fn button(href: Option<&str>, attrs: Attrs, children: Markup) -> Markup {
                html! { @element(if href.is_some() { "a" } else { "button" }) .button href=[href] (..attrs) { (children) } }
            }
        "##,
        ),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "form")]
            fn view() -> Markup { html! { form bq-setup { Button bq-ref="save" { "Save" } } } }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Form"), r#"Setup<"form", { save: "a" | "button" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn children_pass_through_nested_components() {
    let (types, _, warnings) = ok(&[
        (
            0,
            "ui.rs",
            r##"
            #[component]
            pub fn inner(attrs: Attrs, children: Markup) -> Markup {
                html! { div bq-setup (..attrs) { (children) } }
            }
            #[component]
            pub fn outer(attrs: Attrs, children: Markup) -> Markup {
                html! { Inner (..attrs) { aside { (children) } } }
            }
        "##,
        ),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "extra")]
            fn view() -> Markup { html! { Outer bq-setup { span bq-ref="extra.x" {} } } }
        "##,
        ),
    ]);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(setup(&types, "Inner"), r#"Setup<"div">"#);
    assert_eq!(setup(&types, "Extra"), r#"Setup<"div", { x: "span" }>"#);
}

#[test]
fn unreached_component_is_walked_alone() {
    let (types, _, warnings) = ok(&[(
        1,
        "input.rs",
        r##"
        #[component(setup = "password-toggle")]
        pub fn input(name: &str) -> Markup {
            html! { div bq-setup { button bq-ref="toggle" {} } }
        }
    "##,
    )]);
    assert_eq!(setup(&types, "PasswordToggle"), r#"Setup<"div", { toggle: "button" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn unknown_component_keeps_its_children() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "outside")]
        fn view() -> Markup { html! { external::Widget bq-setup { i bq-ref="icon" {} } } }
    "##,
    )]);
    assert_eq!(setup(&types, "Outside"), r#"Setup<string, { icon: "i" }>"#);
}

#[test]
fn component_path_uses_the_last_segment() {
    let (types, ..) = ok(&[
        (0, "card.rs", CARD),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "collapse")]
            fn view() -> Markup { html! { ui::Card bq-setup {} } }
        "##,
        ),
    ]);
    assert_eq!(setup(&types, "Collapse"), "Setup<string>");
}

#[test]
fn recursive_component_stops() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component]
        pub fn tree(children: Markup) -> Markup {
            html! { ul bq-setup { li bq-ref="node" { Tree { (children) } } } }
        }
        fn view() -> Markup { html! { Tree { b {} } } }
    "##,
    )]);
    assert!(setup(&types, "Tree").starts_with(r#"Setup<"ul""#), "{types}");
}

#[test]
fn same_root_wins_and_ambiguity_warns() {
    let card_b = CARD.replace("section", "article");
    let card_c = CARD.replace("section", "aside");
    let (types, _, warnings) = ok(&[
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "here")]
            fn view() -> Markup { html! { Card bq-setup {} } }
        "##,
        ),
        (0, "card.rs", CARD),
        (1, "b.rs", &card_b),
    ]);
    assert_eq!(setup(&types, "Here"), r#"Setup<"section">"#);
    assert!(warnings.is_empty(), "{warnings:?}");

    let (types, _, warnings) = ok(&[
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "here")]
            fn view() -> Markup { html! { Card bq-setup {} } }
        "##,
        ),
        (1, "b.rs", &card_b),
        (2, "c.rs", &card_c),
    ]);
    assert_eq!(setup(&types, "Here"), "Setup<string>");
    assert_eq!(
        warnings,
        ["a.rs:3: `Card` matches more than one function (b.rs, c.rs), so bq_setups does not follow it"]
    );
}

#[test]
fn errors_are_reported_once() {
    let errors = run(&[(
        0,
        "a.rs",
        r##"

        #[component]
        pub fn item(children: Markup) -> Markup { html! { a bq-ref="Bad" {} (children) } }
        #[component(setup = "menu")] fn view() -> Markup { html! { nav bq-setup { Item {} Item {} } } }
    "##,
    )])
    .unwrap_err();
    assert_eq!(
        errors,
        [r#"a.rs:4: bq-ref="Bad" must be lowercase kebab-case, for example bq-ref="close-button" or bq-ref="menu.close-button""#,]
    );
}

#[test]
fn handlers_on_the_setup_element_and_inside_it() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "dropdown")]
        fn view() -> Markup {
            html! {
                details bq-setup bq-on:keydown|escape="close" bq-on:pointerdown|document="outside" {
                    summary bq-on:click|prevent="toggle" {}
                }
            }
        }
    "##,
    )]);
    assert_eq!(
        setup(&types, "Dropdown"),
        r#"Setup<"details", {}, { close: { event: "keydown"; tag: "details" }; outside: { event: "pointerdown"; tag: "details"; target: "document" }; toggle: { event: "click"; tag: "summary" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn handlers_with_refs_and_several_uses() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "picker")]
        fn view() -> Markup {
            html! {
                div bq-setup {
                    input bq-ref="custom" bq-on:input="pick";
                    @for color in colors { button bq-on:click="pick" {} }
                    button bq-on:resize|window="pick" {}
                }
            }
        }
    "##,
    )]);
    assert_eq!(
        setup(&types, "Picker"),
        r#"Setup<"div", { custom: "input" }, { pick: { event: "click"; tag: "button" } | { event: "input"; tag: "input" } | { event: "resize"; tag: "button"; target: "window" } }>"#
    );
}

#[test]
fn setups_without_handlers_are_unchanged() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "plain")]
        fn view() -> Markup { html! { div bq-setup { i bq-ref="icon" {} } } }
    "##,
    )]);
    assert_eq!(setup(&types, "Plain"), r#"Setup<"div", { icon: "i" }>"#);
}

#[test]
fn named_handler_reaches_an_outer_setup() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component] fn password_toggle(children: Markup) -> Markup { html! { div bq-setup { (children) } } }
        #[component(setup = "dirty-form")]
        fn view() -> Markup {
            html! {
                form bq-setup {
                    PasswordToggle {
                        input bq-on:input="dirty-form.update" bq-on:focus="reveal";
                    }
                }
            }
        }
    "##,
    )]);
    assert_eq!(
        setup(&types, "DirtyForm"),
        r#"Setup<"form", {}, { update: { event: "input"; tag: "input" } }>"#
    );
    assert_eq!(
        setup(&types, "PasswordToggle"),
        r#"Setup<"div", {}, { reveal: { event: "focus"; tag: "input" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn two_setups_need_a_setup_name() {
    let errors = run(&[(
        0,
        "a.rs",
        r##"
        #[component] pub fn dirty_form(attrs: Attrs) -> Markup { html! { form bq-setup (..attrs) {} } }
        #[component(setup = "theme-preview")]
        fn view() -> Markup { html! { DirtyForm bq-setup bq-on:input="update" {} } }
    "##,
    )])
    .unwrap_err();
    assert_eq!(
        errors,
        [
            r#"a.rs:4: bq-on:input="update" is inside more than one setup (dirty-form, theme-preview); write the setup name, for example bq-on:input="dirty-form.update""#
        ]
    );

    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component] pub fn dirty_form(attrs: Attrs) -> Markup { html! { form bq-setup (..attrs) {} } }
        #[component(setup = "theme-preview")]
        fn view() -> Markup { html! { DirtyForm bq-setup bq-on:input="theme-preview.refresh" {} } }
    "##,
    )]);
    assert_eq!(
        setup(&types, "ThemePreview"),
        r#"Setup<"form", {}, { refresh: { event: "input"; tag: "form" } }>"#
    );
    assert_eq!(setup(&types, "DirtyForm"), r#"Setup<"form">"#);
}

#[test]
fn handlers_outside_a_setup_are_errors() {
    let errors = run(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "dirty-form")]
        fn view() -> Markup { html! { div bq-setup {} button bq-on:click="save" {} button bq-on:click="dirty-form.save" {} } }
    "##,
    )])
    .unwrap_err();
    assert_eq!(
        errors,
        [
            r#"a.rs:3: bq-on:click="save" is not inside an element with bq-setup"#,
            r#"a.rs:3: bq-on:click="dirty-form.save" is not inside the element with bare bq-setup in component `view`"#,
        ]
    );
}

#[test]
fn handler_in_a_followed_function_does_not_warn() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "menu")]
        fn view() -> Markup { html! { nav bq-setup { (item()) } } }
        fn item() -> Markup { html! { a bq-on:click|prevent="open" {} } }
    "##,
    )]);
    assert_eq!(setup(&types, "Menu"), r#"Setup<"nav", {}, { open: { event: "click"; tag: "a" } }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn call_site_handler_goes_to_the_spread_element() {
    let (types, _, warnings) = ok(&[
        (0, "card.rs", CARD),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "collapse")]
            fn view() -> Markup {
                html! {
                    div bq-setup { Card bq-on:click="toggle" {} }
                    Card bq-setup bq-on:keydown|enter="toggle" {}
                    external::Widget bq-setup bq-on:focus="toggle" {}
                }
            }
        "##,
        ),
    ]);
    assert_eq!(
        setup(&types, "Collapse"),
        r#"Setup<string, {}, { toggle: { event: "click"; tag: "section" } | { event: "focus"; tag: string } | { event: "keydown"; tag: "section" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn call_site_handler_with_the_same_attribute_merges() {
    let (types, _, warnings) = ok(&[
        (
            0,
            "form.rs",
            r##"
            #[component]
            pub fn dirty_form(attrs: Attrs, children: Markup) -> Markup {
                html! { form bq-setup bq-on:input="dirty-form.update" (..attrs) { (children) } }
            }
        "##,
        ),
        (
            0,
            "a.rs",
            r##"
            #[component(setup = "preview")]
            fn view() -> Markup {
                html! { DirtyForm bq-setup bq-on:input="preview.schedule preview.count" {} }
            }
        "##,
        ),
    ]);
    assert_eq!(
        setup(&types, "Preview"),
        r#"Setup<"form", {}, { count: { event: "input"; tag: "form" }; schedule: { event: "input"; tag: "form" } }>"#
    );
    assert_eq!(
        setup(&types, "DirtyForm"),
        r#"Setup<"form", {}, { update: { event: "input"; tag: "form" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn several_handlers_in_one_attribute() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "menu")]
        fn view() -> Markup { html! { nav bq-setup { a bq-on:click|prevent="open  track-click" {} } } }
    "##,
    )]);
    assert_eq!(
        setup(&types, "Menu"),
        r#"Setup<"nav", {}, { open: { event: "click"; tag: "a" }; trackClick: { event: "click"; tag: "a" } }>"#
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn handler_errors() {
    let cases = [
        (r#"bq-on:click|later="a""#, "`later` is not a modifier"),
        (r#"bq-on:click|enter="a""#, "`enter` cannot be used with the `click` event"),
        (r#"bq-on:input|s="a""#, "`s` cannot be used with the `input` event"),
        (r#"bq-on:focus|ctrl="a""#, "`ctrl` cannot be used with the `focus` event"),
        (r#"bq-on:keydown|middle="a""#, "`middle` cannot be used with the `keydown` event"),
        (
            r#"bq-on:touchstart|passive|prevent="a""#,
            "`passive` and `prevent` cannot be used together",
        ),
        (r#"bq-on:click|window|document="a""#, "use only one of `window` and `document`"),
        (
            r#"bq-on:click|self|document="a""#,
            "`self` cannot be used with `window` or `document`",
        ),
        (r#"bq-on:click|once|once="a""#, "the modifier `once` is given two times"),
        (r#"bq-on:htmx:afterSwap="a""#, "event names must be lowercase"),
        (
            r#"bq-on:click="saveIt""#,
            r#"the handler must be kebab-case, for example bq-on:click="save-it""#,
        ),
        (r#"bq-on:click="menu.saveIt""#, r#"for example bq-on:click="menu.save-it""#),
        (r#"bq-on:click="save_it""#, "the handler must be kebab-case"),
        (r#"bq-on:click="save-""#, "the handler must be kebab-case"),
        (r#"bq-on:click="DirtyForm.save""#, "the setup name must be kebab-case"),
        (r#"bq-on:click=(name)"#, "bq-on:click needs a fixed string"),
        (r#"bq-on:click="disconnect""#, "`disconnect` is reserved"),
        (r#"bq-on:click="save Bad""#, r#"bq-on:click="Bad": the handler must be kebab-case"#),
        (r#"bq-on:click=" ""#, "bq-on:click needs a handler"),
    ];
    for (attribute, expected) in cases {
        let source =
            format!("#[component(setup = \"menu\")]\nfn view() -> Markup {{ html! {{ div bq-setup {{ button {attribute} {{}} }} }} }}");
        let errors = run(&[(0, "a.rs", &source)]).unwrap_err();
        assert_eq!(errors.len(), 1, "{attribute}: {errors:?}");
        assert!(errors[0].contains(expected), "{attribute}: {errors:?}");
    }
}

#[test]
fn kebab_handlers_become_camel_case() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "editor")]
        fn view() -> Markup { html! { div bq-setup { input bq-on:change="files-picked" bq-on:input="editor.mark-dirty-2"; } } }
    "##,
    )]);
    assert_eq!(
        setup(&types, "Editor"),
        r#"Setup<"div", {}, { filesPicked: { event: "change"; tag: "input" }; markDirty2: { event: "input"; tag: "input" } }>"#
    );
}

#[test]
fn allowed_modifier_combinations() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "keys")]
        fn view() -> Markup {
            html! {
                div bq-setup {
                    input bq-on:keydown|mod|exact|s|prevent|window="save" bq-on:keyup|left|right="move";
                    a bq-on:click|right|ctrl|stop="menu" bq-on:mousedown|middle="wheel" bq-on:wheel|shift|passive="scroll" {}
                    p bq-on:htmx:before:request|document|once="flush" bq-on:click|self|capture="self" {}
                }
            }
        }
    "##,
    )]);
    assert!(
        types.contains(r#"flush: { event: "htmx:before:request"; tag: "p"; target: "document" }"#),
        "{types}"
    );
    assert!(
        types.contains(r#"save: { event: "keydown"; tag: "input"; target: "window" }"#),
        "{types}"
    );
}

#[test]
fn signal_and_props_are_normal_ref_names() {
    let (types, ..) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "menu")]
        fn view() -> Markup { html! { nav bq-setup { i bq-ref="signal props" {} } } }
    "##,
    )]);
    assert_eq!(setup(&types, "Menu"), r#"Setup<"nav", { props: "i"; signal: "i" }>"#);
}

#[test]
fn wrong_prefix_in_a_helper_called_inside_a_setup_is_an_error() {
    let errors = run(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "menu")]
        fn view() -> Markup { html! { nav bq-setup { (item()) } } }
        fn item() -> Markup { html! { a bq-ref="toolbar.link" {} } }
    "##,
    )])
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with(r#"a.rs:4: bq-ref="toolbar.link" names the setup "toolbar", but no #[component] has that setup"#),
        "{errors:?}"
    );
}

#[test]
fn typed_let_bindings_and_impl_method_helpers_are_followed() {
    let (types, _, warnings) = ok(&[(
        0,
        "a.rs",
        r##"
        #[component(setup = "card")]
        fn view() -> Markup {
            let title: Markup = html! { h2 bq-ref="title" {} };
            html! { section bq-setup { (title) (Row::render()) } }
        }
        struct Row;
        impl Row {
            fn render() -> Markup { html! { p bq-ref="row" {} } }
        }
    "##,
    )]);
    assert_eq!(setup(&types, "Card"), r#"Setup<"section", { row: "p"; title: "h2" }>"#);
    assert!(warnings.is_empty(), "{warnings:?}");
}
