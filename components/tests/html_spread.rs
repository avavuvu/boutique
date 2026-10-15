use bq_components::{Attrs, attrs, html};

#[test]
fn spread_writes_entries() {
    let extra = attrs!(id = "x", hidden, "data-n" = 3);
    let result = html! { div (..extra) {} };
    assert_eq!(result.into_string(), r#"<div id="x" hidden data-n="3"></div>"#);
}

#[test]
fn spread_on_void_element() {
    let result = html! { input type="text" (..attrs!(required));  };
    assert_eq!(result.into_string(), r#"<input type="text" required>"#);
}

#[test]
fn spread_by_reference_keeps_the_value() {
    let extra = attrs!(title = "t");
    let by_value = html! { p (..extra) {} };
    let by_reference = html! { p (..&extra) {} };
    assert_eq!(by_value.into_string(), by_reference.into_string());
    assert!(extra.has("title"));
}

#[test]
fn empty_spread_writes_nothing() {
    let result = html! { p (..Attrs::new()) { "x" } };
    assert_eq!(result.into_string(), "<p>x</p>");
}

#[test]
fn static_value_wins() {
    let result = html! { button type="button" (..attrs!(type = "submit", disabled)) {} };
    assert_eq!(result.into_string(), r#"<button type="button" disabled></button>"#);
}

#[test]
fn static_value_wins_when_spread_comes_first() {
    let result = html! { button (..attrs!(type = "submit")) type="button" {} };
    assert_eq!(result.into_string(), r#"<button type="button"></button>"#);
}

#[test]
fn static_id_wins() {
    let result = html! { p #me (..attrs!(id = "other", title = "t")) {} };
    assert_eq!(result.into_string(), r#"<p id="me" title="t"></p>"#);
}

#[test]
fn absent_optional_attribute_still_owns_its_name() {
    let href: Option<&str> = None;
    let result = html! { a href=[href] (..attrs!(href = "/x")) {} };
    assert_eq!(result.into_string(), "<a></a>");
}

#[test]
fn false_toggle_still_owns_its_name() {
    let result = html! { input disabled[false] (..attrs!(disabled));  };
    assert_eq!(result.into_string(), "<input>");
}

#[test]
fn classes_merge_in_order() {
    let result = html! { div.a.b[true].c[false] class="d" (..attrs!(class = "e f")) {} };
    assert_eq!(result.into_string(), r#"<div class="a b d e f"></div>"#);
}

#[test]
fn spread_class_alone() {
    let result = html! { div (..attrs!(class = "e")) {} };
    assert_eq!(result.into_string(), r#"<div class="e"></div>"#);
}

#[test]
fn no_class_when_every_part_is_empty() {
    let empty = "";
    let result = html! { div.a[false] class=(empty) (..attrs!(id = "x")) {} };
    assert_eq!(result.into_string(), r#"<div id="x"></div>"#);
}

#[test]
fn optional_class_attribute_merges() {
    let some = Some("s");
    let none: Option<&str> = None;
    let result = html! { div.a class=[some] (..attrs!(class = "z")) {} div.a class=[none] (..Attrs::new()) {} };
    assert_eq!(result.into_string(), r#"<div class="a s z"></div><div class="a"></div>"#);
}

#[test]
fn dynamic_class_parts_are_escaped() {
    let name = "<x>";
    let result = html! { div.(name).{ "p-" (name) } (..attrs!(class = "\"q\"")) {} };
    assert_eq!(result.into_string(), r#"<div class="&lt;x&gt; p-&lt;x&gt; &quot;q&quot;"></div>"#);
}

#[test]
fn attributes_keep_their_order() {
    let result = html! { a.button #go href="/a" (..attrs!(rel = "next", class = "big")) title="t" {} };
    assert_eq!(result.into_string(), r#"<a class="button big" id="go" href="/a" title="t" rel="next"></a>"#);
}

#[test]
fn first_spread_wins_and_classes_merge() {
    let first = attrs!(title = "one", class = "a");
    let second = attrs!(title = "two", lang = "en", class = "b");
    let result = html! { p (..first) (..second) {} };
    assert_eq!(result.into_string(), r#"<p class="a b" title="one" lang="en"></p>"#);
}

#[test]
fn spread_on_div_shorthand_and_in_control_flow() {
    let items = ["x", "y"];
    let result = html! {
        @for item in &items {
            .row (..attrs!("data-item" = item)) {}
        }
    };
    assert_eq!(result.into_string(), r#"<div class="row" data-item="x"></div><div class="row" data-item="y"></div>"#);
}

#[test]
fn classes_merge_without_spread() {
    let toggle = false;
    let extra = Some("c");
    let result = html! { div.a class="b" {} p.x[toggle] class=[extra] class="d" {} };
    assert_eq!(result.into_string(), r#"<div class="a b"></div><p class="c d"></p>"#);
}

#[test]
fn single_class_attribute_is_unchanged() {
    let result = html! { div class="" {} p class="b" title="t" {} };
    assert_eq!(result.into_string(), r#"<div class=""></div><p class="b" title="t"></p>"#);
}
