use bq_components::{Button, Input, Kind, Textarea, attrs, button, html, input};

#[test]
fn button_defaults_to_type_button() {
    let result = html! { Button { "Save" } };
    assert_eq!(result.into_string(), r#"<button class="button" type="button">Save</button>"#);
}

#[test]
fn button_submit_with_styling_classes() {
    let result = html! { Button(submit = true) .primary .small { "Save" } };
    assert_eq!(result.into_string(), r#"<button class="button primary small" type="submit">Save</button>"#);
}

#[test]
fn button_link() {
    let result = html! { Button(href = "/edit?a=1&b=2") .ghost { "Edit" } };
    assert_eq!(result.into_string(), r#"<a class="button ghost" href="/edit?a=1&amp;b=2">Edit</a>"#);
}

#[test]
fn button_link_accepts_a_string() {
    let id = 7;
    let result = html! { Button(href = format!("/items/{id}")) { "Open" } };
    assert_eq!(result.into_string(), r#"<a class="button" href="/items/7">Open</a>"#);
}

#[test]
fn disabled_button_and_disabled_link() {
    let locked = true;
    let result = html! { Button(disabled = locked) { "A" } Button(href = "/x", disabled = locked) { "B" } };
    assert_eq!(
        result.into_string(),
        r#"<button class="button" type="button" disabled>A</button><a class="button" href="/x" aria-disabled="true">B</a>"#
    );
}

#[test]
fn post_button_wraps_a_form() {
    let result = html! { Button(post = "/logout") { "Log out" } };
    assert_eq!(
        result.into_string(),
        r#"<form class="button-form" method="POST" action="/logout"><button class="button" type="submit">Log out</button></form>"#
    );
}

#[test]
fn post_attributes_go_on_the_button() {
    let result = html! { Button(post = "/x") .danger data-x="1" { "Delete" } };
    assert_eq!(
        result.into_string(),
        r#"<form class="button-form" method="POST" action="/x"><button class="button danger" type="submit" data-x="1">Delete</button></form>"#
    );
}

#[test]
fn link_ignores_post() {
    let result = html! { Button(href = "/a", post = "/b") { "A" } };
    assert_eq!(result.into_string(), r#"<a class="button" href="/a">A</a>"#);
}

#[test]
fn button_htmx_and_static_type_wins() {
    let id = 3;
    let result = html! {
        Button .danger hx-delete=(format!("/x/{id}")) hx-status:4xx="swap:none" type="submit" class="\" onclick=\"x" { "Delete" }
    };
    assert_eq!(
        result.into_string(),
        r#"<button class="button danger &quot; onclick=&quot;x" type="button" hx-delete="/x/3" hx-status:4xx="swap:none">Delete</button>"#
    );
}

#[test]
fn button_without_children() {
    let result = html! { Button .icon aria-label="Close"; };
    assert_eq!(result.into_string(), r#"<button class="button icon" type="button" aria-label="Close"></button>"#);
}

#[test]
fn button_builder_outside_html() {
    let result = button().href("/a").children(html! { "A" }).build();
    assert_eq!(result.into_string(), r#"<a class="button" href="/a">A</a>"#);
}

#[test]
fn input_with_attributes_and_escaping() {
    let result = html! {
        Input("q") placeholder="a \"quoted\" <thing>" x-model="query" hx-get="/search?a=1&b=2" required;
    }
    .into_string();
    assert!(
        result.contains(
            r#"<input id="q" name="q" type="text" placeholder="a &quot;quoted&quot; &lt;thing&gt;" x-model="query" hx-get="/search?a=1&amp;b=2" required autocomplete="q">"#
        ),
        "{result}"
    );
    assert!(result.contains(r#"<div class="shell">"#), "{result}");
    assert!(result.contains(r#"<p class="error" id="q-error"></p>"#), "{result}");
}

#[test]
fn input_autocomplete_can_be_replaced() {
    let result = html! { Input("handle") autocomplete="username"; }.into_string();
    assert!(result.contains(r#"type="text" autocomplete="username">"#), "{result}");
    assert_eq!(result.matches("autocomplete").count(), 1, "{result}");

    let none = html! { Input("n", kind = Kind::Number); }.into_string();
    assert!(!none.contains("autocomplete"), "{none}");
}

#[test]
fn input_validation_attributes() {
    let result = html! { Input("slug") pattern="[a-z]+" minlength=(3) maxlength=(40); }.into_string();
    assert!(result.contains(r#"pattern="[a-z]+" minlength="3" maxlength="40""#), "{result}");

    let file = html! { Input("file", kind = Kind::File) accept=".csv"; }.into_string();
    assert!(file.contains(r#"type="file" accept=".csv""#), "{file}");
}

#[test]
fn password_keeps_the_toggle_and_extra_attributes() {
    let result = html! { Input("password", kind = Kind::Password, label = "Password") .wide data-test="pw" required; }.into_string();
    assert!(result.contains(r#"<label for="password">Password</label>"#), "{result}");
    assert!(result.contains(r#"<div class="shell password" bq-setup="password-toggle">"#), "{result}");
    assert!(
        result.contains(
            r#"<input class="password-input wide" id="password" name="password" type="password" data-test="pw" required autocomplete="current-password">"#
        ),
        "{result}"
    );
    assert!(result.contains(r#"<button type="button" tabindex="-1" bq-ref="toggle">Show</button>"#), "{result}");
}

#[test]
fn input_suffix_hint_and_disabled() {
    let locked = true;
    let result = html! { Input("slug", label = "Address", suffix = ".example.com", hint = "Lowercase") disabled[locked]; }.into_string();
    assert!(result.contains(r#"<label for="slug">Address</label>"#), "{result}");
    assert!(result.contains(r#"<div class="shell prefixed">"#), "{result}");
    assert!(result.contains(r#"<span class="suffix">.example.com</span>"#), "{result}");
    assert!(result.contains(" disabled autocomplete=\"slug\">"), "{result}");
    assert!(result.contains(r#"<p class="hint">Lowercase</p>"#), "{result}");
}

#[test]
fn input_value_from_a_variable() {
    let name = String::from("Ava \"A\"");
    let result = html! { Input("name") value=(name); }.into_string();
    assert!(result.contains(r#"value="Ava &quot;A&quot;""#), "{result}");
}

#[test]
fn input_builder_outside_html() {
    let result = input("q").attrs(attrs!(required)).build().into_string();
    assert!(result.contains(" required autocomplete=\"q\">"), "{result}");
}

#[test]
fn textarea_escapes_its_value() {
    let result = html! { Textarea("bio", value = "</textarea><b>x</b>") rows=(3) maxlength=(10); }.into_string();
    assert!(
        result.contains(r#"<textarea id="bio" name="bio" rows="3" maxlength="10">&lt;/textarea&gt;&lt;b&gt;x&lt;/b&gt;</textarea>"#),
        "{result}"
    );
}
