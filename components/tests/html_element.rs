use bq_components::{attrs, html};

#[test]
fn variable_tag() {
    let tag = "button";
    let result = html! { @element(tag) .button { "Save" } };
    assert_eq!(result.into_string(), r#"<button class="button">Save</button>"#);
}

#[test]
fn conditional_tag() {
    for (link, expected) in [(true, r#"<a class="button" href="/x">Save</a>"#), (false, r#"<button class="button">Save</button>"#)] {
        let href = link.then_some("/x");
        let result = html! { @element(if link { "a" } else { "button" }) .button href=[href] { "Save" } };
        assert_eq!(result.into_string(), expected);
    }
}

#[test]
fn match_tag() {
    let level = 2;
    let result = html! {
        @element(match level { 1 => "h1", 2 => "h2", _ => "h3" }) #title { "Hi" }
    };
    assert_eq!(result.into_string(), r#"<h2 id="title">Hi</h2>"#);
}

#[test]
fn void_tag() {
    let tag = "input";
    let result = html! { @element(tag) type="text" required; };
    assert_eq!(result.into_string(), r#"<input type="text" required>"#);
}

#[test]
fn tag_with_spread_and_class_merge() {
    let extra = attrs!(type = "submit", class = "primary", disabled);
    let result = html! { @element("button") .button type="button" (..extra) { "Go" } };
    assert_eq!(result.into_string(), r#"<button class="button primary" type="button" disabled>Go</button>"#);
}

#[test]
fn nested_dynamic_tags() {
    let outer = "section";
    let inner = "p";
    let result = html! { @element(outer) { @element(inner) { "a" } @element(inner) { "b" } } };
    assert_eq!(result.into_string(), "<section><p>a</p><p>b</p></section>");
}

#[test]
fn tag_in_control_flow() {
    let tags = ["li", "dt"];
    let result = html! { @for tag in tags { @element(tag) { (tag) } } };
    assert_eq!(result.into_string(), "<li>li</li><dt>dt</dt>");
}

#[test]
fn tag_expression_is_evaluated_once() {
    let mut calls = 0;
    let mut next = || {
        calls += 1;
        "span"
    };
    let result = html! { @element(next()) { "x" } };
    assert_eq!(result.into_string(), "<span>x</span>");
    assert_eq!(calls, 1);
}

#[test]
fn custom_element_tag() {
    let tag = "my-widget";
    let result = html! { @element(tag) {} };
    assert_eq!(result.into_string(), "<my-widget></my-widget>");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "is not a valid tag name for @element")]
fn invalid_tag_panics_in_debug() {
    let tag = "div onclick=x";
    let _ = html! { @element(tag) {} };
}
