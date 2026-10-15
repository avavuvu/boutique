use bq_components::{
    Attrs, attrs, component, html,
    maud::{Markup, Render},
};

#[component]
fn card(
    #[positional] title: &str,
    subtitle: Option<&str>,
    wide: bool,
    attrs: Attrs,
    children: Markup,
) -> Markup {
    html! {
        section.card.wide[wide] (..attrs) {
            h2 { (title) }
            @if let Some(subtitle) = subtitle { p { (subtitle) } }
            (children)
        }
    }
}

#[component]
fn post_button(#[positional] action: &str, attrs: Attrs, children: Markup) -> Markup {
    html! {
        form method="POST" action=(action) {
            button.button type="submit" (..attrs) { (children) }
        }
    }
}

#[component]
fn divider() -> Markup {
    html! { hr; }
}

#[allow(non_snake_case)]
#[component]
fn Plain() -> Markup {
    html! { br; }
}

mod ui {
    use bq_components::{Attrs, component, html, maud::Markup};

    #[component]
    pub fn badge(#[default] attrs: Attrs, children: Markup) -> Markup {
        html! { span.badge (..attrs) { (children) } }
    }
}

#[test]
fn positional_and_children() {
    let result = html! { Card("Hello") { "body" } };
    assert_eq!(result.into_string(), r#"<section class="card"><h2>Hello</h2>body</section>"#);
}

#[test]
fn named_arguments() {
    let wide = true;
    let result = html! { Card("Hello", subtitle = "sub", wide = wide) {} };
    assert_eq!(result.into_string(), r#"<section class="card wide"><h2>Hello</h2><p>sub</p></section>"#);
}

#[test]
fn expressions_as_arguments() {
    let name = "Ava";
    let result = html! { Card(&format!("Hi {name}"), wide = name.len() > 2) {} };
    assert_eq!(result.into_string(), r#"<section class="card wide"><h2>Hi Ava</h2></section>"#);
}

#[test]
fn void_body_uses_the_default_children() {
    let result = html! { Card("Empty"); };
    assert_eq!(result.into_string(), r#"<section class="card"><h2>Empty</h2></section>"#);
}

#[test]
fn no_parentheses() {
    let result = html! { Divider; ui::Badge.new { "x" } };
    assert_eq!(result.into_string(), r#"<hr><span class="badge new">x</span>"#);
}

#[test]
fn multi_word_name() {
    let result = html! { PostButton("/logout") .danger { "Log out" } };
    assert_eq!(
        result.into_string(),
        r#"<form method="POST" action="/logout"><button class="button danger" type="submit">Log out</button></form>"#
    );
}

#[test]
fn module_path() {
    let result = html! { ui::Badge { "new" } };
    assert_eq!(result.into_string(), r#"<span class="badge">new</span>"#);
}

#[test]
fn local_variable_does_not_hide_the_component() {
    let card = 5;
    let post_button = "x";
    let result = html! { Card("A"); PostButton(post_button) { (card) } };
    assert_eq!(
        result.into_string(),
        r#"<section class="card"><h2>A</h2></section><form method="POST" action="x"><button class="button" type="submit">5</button></form>"#
    );
}

#[test]
fn call_site_attributes_reach_the_root() {
    let urgent = true;
    let id = "main";
    let result = html! {
        Card("T") .extra .urgent[urgent] .hidden[false] #(id) data-x="1" hx-post=(format!("/a?b={}", 2)) open {}
    };
    assert_eq!(
        result.into_string(),
        r#"<section class="card extra urgent" id="main" data-x="1" hx-post="/a?b=2" open><h2>T</h2></section>"#
    );
}

#[test]
fn call_site_static_attributes_win_over_spreads() {
    let extra = attrs!(title = "spread", lang = "en", class = "s");
    let none: Option<&str> = None;
    let result = html! { Card("T") .a title="static" href=[none] (..extra) (..attrs!(href = "/x", lang = "fr")) {} };
    assert_eq!(
        result.into_string(),
        r#"<section class="card a s" title="static" lang="en"><h2>T</h2></section>"#
    );
}

#[test]
fn spread_right_after_the_name_is_not_an_argument() {
    let extra = attrs!(title = "t");
    let result = html! { ui::Badge (..extra) { "a" } Card("T") (..attrs!(lang = "en")); };
    assert_eq!(
        result.into_string(),
        r#"<span class="badge" title="t">a</span><section class="card" lang="en"><h2>T</h2></section>"#
    );
}

#[test]
fn component_static_attributes_win_over_call_site() {
    let result = html! { PostButton("/x") type="button" .big { "Go" } };
    assert_eq!(
        result.into_string(),
        r#"<form method="POST" action="/x"><button class="button big" type="submit">Go</button></form>"#
    );
}

#[test]
fn optional_and_toggled_attributes() {
    let some = Some("yes");
    let result = html! { ui::Badge data-a=[some] data-b=[None::<&str>] hidden[true] disabled[false] { "x" } };
    assert_eq!(result.into_string(), r#"<span class="badge" data-a="yes" hidden>x</span>"#);
}

#[test]
fn escaping_at_the_call_site() {
    let value = "\"<x>\"";
    let result = html! { ui::Badge title=(value) .(value) { (value) } };
    assert_eq!(
        result.into_string(),
        r#"<span class="badge &quot;&lt;x&gt;&quot;" title="&quot;&lt;x&gt;&quot;">&quot;&lt;x&gt;&quot;</span>"#
    );
}

#[test]
fn nested_components_and_control_flow() {
    let items = ["a", "b"];
    let result = html! {
        Card("List") {
            @for item in &items {
                @let upper = item.to_uppercase();
                ui::Badge .item { (upper) }
            }
            @if items.len() > 1 { Divider; }
        }
    };
    assert_eq!(
        result.into_string(),
        r#"<section class="card"><h2>List</h2><span class="badge item">A</span><span class="badge item">B</span><hr></section>"#
    );
}

#[test]
fn component_inside_dynamic_element() {
    let tag = "main";
    let result = html! { @element(tag) { ui::Badge { "x" } } };
    assert_eq!(result.into_string(), r#"<main><span class="badge">x</span></main>"#);
}

#[test]
fn lowercase_names_are_still_elements() {
    let result = html! { card { "x" } Card-x {} };
    assert_eq!(result.into_string(), "<card>x</card><Card-x></Card-x>");
}

#[test]
fn capitalised_function_needs_no_alias() {
    assert_eq!(html! { Plain; }.into_string(), "<br>");
}

#[test]
fn exported_aliases() {
    let result = html! { bq_components::Input("q", label = "Search") data-x="1"; }.into_string();
    assert!(result.contains(r#"<input id="q" name="q" type="text" aria-describedby="q-error" data-x="1">"#), "{result}");
}

#[test]
fn builder_still_works_outside_html() {
    let result = card("Plain").wide(true).build();
    assert_eq!(result.into_string(), r#"<section class="card wide"><h2>Plain</h2></section>"#);
    assert_eq!(divider().render().into_string(), "<hr>");
}
