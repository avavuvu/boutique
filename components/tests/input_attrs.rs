use bq_components::{Kind, attrs, button, input, textarea};
use maud::{Render, html};

#[test]
fn input_renders_extra_attrs_and_escapes_values() {
    let html = input("q")
        .placeholder("a \"quoted\" <thing>")
        .attrs(attrs!(x_model = "query", "hx-get" = "/search?a=1&b=2"))
        .required()
        .build()
        .into_string();

    assert!(html.contains(r#"<input id="q" name="q" type="text" autocomplete="q""#), "{html}");
    assert!(html.contains(r#"placeholder="a &quot;quoted&quot; &lt;thing&gt;""#), "{html}");
    assert!(html.contains(r#"x-model="query""#), "{html}");
    assert!(html.contains(r#"hx-get="/search?a=1&amp;b=2""#), "{html}");
    assert!(html.contains(" required>"), "{html}");
}

#[test]
fn input_named_validation_options() {
    let html = input("slug").pattern("[a-z]+").min_length(3).max_length(40).render().into_string();
    assert!(html.contains(r#"pattern="[a-z]+" minlength="3" maxlength="40""#), "{html}");

    let file = input("file").kind(Kind::File).accept(".csv").render().into_string();
    assert!(file.contains(r#"type="file""#) && file.contains(r#"accept=".csv""#), "{file}");
}

#[test]
fn password_keeps_toggle_and_extra_attrs() {
    let html = input("password").kind(Kind::Password).attrs(attrs!(data_test = "pw")).render().into_string();
    assert!(html.contains(r#"class="password-input""#), "{html}");
    assert!(html.contains(r#"autocomplete="current-password""#), "{html}");
    assert!(html.contains(r#"<div class="shell password" bq-setup="password-toggle">"#), "{html}");
    assert!(html.contains(r#"<button type="button" tabindex="-1" bq-ref="toggle">Show</button>"#), "{html}");
    assert!(html.contains(r#"data-test="pw""#), "{html}");
}

#[test]
fn input_suffix_disabled_and_hint() {
    let html = input("slug").label("Address").suffix(".example.com").hint("Lowercase").disabled().render().into_string();
    assert!(html.contains(r#"<label for="slug">Address</label>"#), "{html}");
    assert!(html.contains(r#"<div class="shell prefixed">"#), "{html}");
    assert!(html.contains(r#"<span class="suffix">.example.com</span>"#), "{html}");
    assert!(html.contains(" disabled>"), "{html}");
    assert!(html.contains(r#"<p class="hint">Lowercase</p>"#), "{html}");
}

#[test]
fn textarea_escapes_its_value() {
    let html = textarea("bio").rows(3).max_length(10).value("</textarea><b>x</b>").render().into_string();
    assert!(html.contains(r#"<textarea id="bio" name="bio" rows="3" maxlength="10">&lt;/textarea&gt;&lt;b&gt;x&lt;/b&gt;</textarea>"#), "{html}");
}

#[test]
fn button_variants_and_attributes() {
    let link = button(html! { "Edit" }).href("/edit?a=1&b=2").ghost().small().render().into_string();
    assert_eq!(link, r#"<a class="button ghost small" href="/edit?a=1&amp;b=2">Edit</a>"#);

    let post = button(html! { "Log out" }).post("/logout").render().into_string();
    assert_eq!(post, r#"<form class="button-form" method="POST" action="/logout"><button class="button" type="submit">Log out</button></form>"#);

    let url = String::from("a\"b");
    let action = button(html! { "Delete" })
        .danger()
        .hx_delete("/x")
        .hx_status_4xx("swap:none")
        .class("\" onclick=\"x")
        .attrs(attrs!(data_copy = &url, data_share, r#type_hint = "t"))
        .render()
        .into_string();
    assert_eq!(
        action,
        r#"<button class="button danger &quot; onclick=&quot;x" type="button" hx-delete="/x" hx-status:4xx="swap:none" data-copy="a&quot;b" data-share type-hint="t">Delete</button>"#
    );
}
