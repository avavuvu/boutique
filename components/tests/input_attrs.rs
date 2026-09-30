use bq_components::Input;
use maud::Render;

#[test]
fn renders_extra_attrs_and_escapes_values() {
    let html = Input::text("q")
        .placeholder("a \"quoted\" <thing>")
        .attr("x-model", "query")
        .attr("hx-get", "/search?a=1&b=2")
        .required()
        .render()
        .into_string();

    assert!(html.contains(r#"<input id="q" name="q" type="text" autocomplete="q""#), "{html}");
    assert!(html.contains(r#"placeholder="a &quot;quoted&quot; &lt;thing&gt;""#), "{html}");
    assert!(html.contains(r#"x-model="query""#), "{html}");
    assert!(html.contains(r#"hx-get="/search?a=1&amp;b=2""#), "{html}");
    assert!(html.contains(" required>"), "{html}");
}

#[test]
fn password_keeps_toggle_and_extra_attrs() {
    let html = Input::password("password").attr("data-test", "pw").render().into_string();
    assert!(html.contains(r#"class="password-input""#), "{html}");
    assert!(html.contains(r#"type="password""#), "{html}");
    assert!(html.contains(r#"<div class="shell password" bq-setup="password-toggle">"#), "{html}");
    assert!(html.contains(r#"<button type="button" tabindex="-1" bq-ref="toggle">Show</button>"#), "{html}");
    assert!(html.contains(r#"data-test="pw""#), "{html}");
}
