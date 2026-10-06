use bq_components::{AttrValue, Attrs, __private::write_attrs, attrs};

fn written(attrs: &Attrs, skip: &[&str]) -> String {
    let mut out = String::new();
    write_attrs(&mut out, attrs, skip);
    out
}

#[test]
fn empty_by_default() {
    let attrs = Attrs::new();
    assert!(attrs.is_empty());
    assert!(!attrs.has("class"));
    assert!(attrs.class().is_none());
    assert_eq!(written(&attrs, &[]), "");
}

#[test]
fn macro_builds_flags_and_escaped_text() {
    let url = String::from("/x?a=1&b=\"2\"");
    let attrs = attrs!(hx_get = &url, required, "data-n" = 3);

    assert!(attrs.has("hx-get") && attrs.has("required") && attrs.has("data-n"));
    assert!(!attrs.has("class"));
    assert!(matches!(attrs.iter().nth(1), Some(("required", AttrValue::Flag))));
    assert_eq!(written(&attrs, &[]), r#" hx-get="/x?a=1&amp;b=&quot;2&quot;" required data-n="3""#);
}

#[test]
fn class_is_merged_and_kept_apart() {
    let attrs = attrs!(class = "a", id = "x", class = "b <c>");
    assert!(attrs.has("class"));
    assert_eq!(attrs.class().map(|class| class.0), Some("a b &lt;c&gt;"));
    assert_eq!(written(&attrs, &[]), r#" id="x""#);
}

#[test]
fn empty_class_is_ignored() {
    let attrs = attrs!(class = "");
    assert!(!attrs.has("class"));
    assert!(attrs.is_empty());
}

#[test]
fn first_value_wins_when_extended() {
    let mut first = attrs!(type = "submit", class = "primary");
    first.__extend(&attrs!(type = "button", disabled, class = "small"));

    assert_eq!(first.class().map(|class| class.0), Some("primary small"));
    assert_eq!(written(&first, &[]), r#" type="submit" disabled"#);
}

#[test]
fn skipped_names_are_not_written() {
    let attrs = attrs!(id = "x", type = "button", hidden);
    assert_eq!(written(&attrs, &["type", "hidden"]), r#" id="x""#);
}
