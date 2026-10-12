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
    let result = html! { Button(href = &format!("/items/{id}")) { "Open" } };
    assert_eq!(result.into_string(), r#"<a class="button" href="/items/7">Open</a>"#);
}

#[test]
fn disabled_button_and_disabled_link() {
    let locked = true;
    let result = html! { Button(disabled = locked) { "A" } Button(href = "/x", disabled = locked) { "B" } };
    assert_eq!(
        result.into_string(),
        r#"<button class="button" type="button" disabled>A</button><a class="button" aria-disabled="true">B</a>"#
    );
}

#[test]
fn disabled_link_ignores_a_spread_href() {
    let result = html! { Button(href = "/x", disabled = true) href="/y" { "B" } };
    assert_eq!(result.into_string(), r#"<a class="button" aria-disabled="true">B</a>"#);
}

#[test]
fn enabled_link_keeps_its_href() {
    let result = html! { Button(href = "/x", disabled = false) { "B" } };
    assert_eq!(result.into_string(), r#"<a class="button" href="/x">B</a>"#);
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
#[cfg(debug_assertions)]
#[should_panic(expected = "both `href` and `post`")]
fn link_with_post_panics_in_debug_builds() {
    let _ = html! { Button(href = "/a", post = "/b") { "A" } };
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
        Input("q", label = "Search") placeholder="a \"quoted\" <thing>" x-model="query" hx-get="/search?a=1&b=2" required;
    }
    .into_string();
    assert!(
        result.contains(
            r#"<input id="q" name="q" type="text" aria-describedby="q-error" placeholder="a &quot;quoted&quot; &lt;thing&gt;" x-model="query" hx-get="/search?a=1&amp;b=2" required>"#
        ),
        "{result}"
    );
    assert!(result.contains(r#"<label for="q">Search</label>"#), "{result}");
    assert!(result.contains(r#"<div class="shell">"#), "{result}");
    assert!(result.contains(r#"<p class="error" id="q-error" aria-live="polite"></p>"#), "{result}");
}

#[test]
fn input_autocomplete_can_be_replaced() {
    let result = html! { Input("handle") aria-label="Handle" autocomplete="username"; }.into_string();
    assert!(result.contains(r#"autocomplete="username">"#), "{result}");
    assert_eq!(result.matches("autocomplete").count(), 1, "{result}");

    let none = html! { Input("n", kind = Kind::Number, label = "N"); }.into_string();
    assert!(!none.contains("autocomplete"), "{none}");
}

#[test]
fn input_autocomplete_only_uses_valid_tokens() {
    for name in ["slug", "address", "description", "first_name", "q", "greeting"] {
        let result = html! { Input(name, label = "Field"); }.into_string();
        assert!(!result.contains("autocomplete"), "{result}");
    }
    for name in ["name", "username", "email", "given-name", "new-password"] {
        let result = html! { Input(name, label = "Field"); }.into_string();
        assert!(result.contains(&format!(r#"autocomplete="{name}">"#)), "{result}");
    }

    let email = html! { Input("contact", kind = Kind::Email, label = "Email"); }.into_string();
    assert!(email.contains(r#"autocomplete="email">"#), "{email}");

    let current = html! { Input("password", kind = Kind::Password, label = "Password"); }.into_string();
    assert!(current.contains(r#"autocomplete="current-password">"#), "{current}");

    let new = html! { Input("password", kind = Kind::Password, label = "Password") autocomplete="new-password"; }.into_string();
    assert!(new.contains(r#"autocomplete="new-password">"#), "{new}");
    assert_eq!(new.matches("autocomplete").count(), 1, "{new}");
}

#[test]
fn input_validation_attributes() {
    let result = html! { Input("slug", label = "Slug") pattern="[a-z]+" minlength=(3) maxlength=(40); }.into_string();
    assert!(result.contains(r#"pattern="[a-z]+" minlength="3" maxlength="40""#), "{result}");

    let file = html! { Input("file", kind = Kind::File, label = "File") accept=".csv"; }.into_string();
    assert!(file.contains(r#"type="file" aria-describedby="file-error" accept=".csv""#), "{file}");
}

#[test]
fn password_keeps_the_toggle_and_extra_attributes() {
    let result = html! { Input("password", kind = Kind::Password, label = "Password") .wide data-test="pw" required; }.into_string();
    assert!(result.contains(r#"<label for="password">Password</label>"#), "{result}");
    assert!(result.contains(r#"<div class="shell password" bq-setup="password-toggle">"#), "{result}");
    assert!(
        result.contains(
            r#"<input class="password-input wide" id="password" name="password" type="password" aria-describedby="password-error" data-test="pw" required autocomplete="current-password">"#
        ),
        "{result}"
    );
    assert!(
        result.contains(
            r#"<button type="button" aria-label="Show password" aria-pressed="false" aria-controls="password" bq-on:click="toggle">Show</button>"#
        ),
        "{result}"
    );
    assert!(!result.contains("tabindex"), "{result}");
}

#[test]
fn password_toggle_controls_the_custom_id() {
    let result = html! { Input("password", id = "signup-password", kind = Kind::Password, label = "Password"); }.into_string();
    assert!(result.contains(r#"id="signup-password" name="password""#), "{result}");
    assert!(result.contains(r#"aria-controls="signup-password""#), "{result}");
}

#[test]
fn input_suffix_hint_and_disabled() {
    let locked = true;
    let result = html! { Input("slug", label = "Address", suffix = ".example.com", hint = "Lowercase") disabled[locked]; }.into_string();
    assert!(result.contains(r#"<label for="slug">Address</label>"#), "{result}");
    assert!(result.contains(r#"<div class="shell prefixed">"#), "{result}");
    assert!(result.contains(r#"<span class="suffix">.example.com</span>"#), "{result}");
    assert!(result.contains(r#" aria-describedby="slug-hint slug-error" disabled>"#), "{result}");
    assert!(result.contains(r#"<p class="hint" id="slug-hint">Lowercase</p>"#), "{result}");
}

#[test]
fn input_error_marks_the_field_invalid() {
    let result = html! { Input("email", kind = Kind::Email, label = "Email", hint = "We never share it", error = "Enter a valid email"); }
        .into_string();
    assert!(
        result.contains(
            r#"<input id="email" name="email" type="email" aria-describedby="email-hint email-error" aria-invalid="true" autocomplete="email">"#
        ),
        "{result}"
    );
    assert!(result.contains(r#"<p class="error" id="email-error" aria-live="polite">Enter a valid email</p>"#), "{result}");

    let valid = html! { Input("email", kind = Kind::Email, label = "Email"); }.into_string();
    assert!(!valid.contains("aria-invalid"), "{valid}");
    assert!(valid.contains(r#"<p class="error" id="email-error" aria-live="polite"></p>"#), "{valid}");
}

#[test]
fn input_custom_id_keeps_ids_unique() {
    let result = html! {
        Input("q", id = "top-q", label = "Search", hint = "Top");
        Input("q", id = "side-q", label = "Search", hint = "Side");
    }
    .into_string();
    for id in ["top-q", "side-q"] {
        assert!(result.contains(&format!(r#"<label for="{id}">"#)), "{result}");
        assert!(result.contains(&format!(r#"<input id="{id}" name="q""#)), "{result}");
        assert!(result.contains(&format!(r#"aria-describedby="{id}-hint {id}-error""#)), "{result}");
        assert!(result.contains(&format!(r#"<p class="hint" id="{id}-hint">"#)), "{result}");
        assert!(result.contains(&format!(r#"<p class="error" id="{id}-error""#)), "{result}");
    }
    assert!(!result.contains(r#"id="q""#), "{result}");
}

#[test]
fn input_accepts_aria_label_or_aria_labelledby() {
    let labelled = html! { Input("q") aria-label="Search"; }.into_string();
    assert!(labelled.contains(r#"aria-label="Search""#), "{labelled}");
    assert!(!labelled.contains("<label"), "{labelled}");

    let by = html! { Input("q") aria-labelledby="search-heading"; }.into_string();
    assert!(by.contains(r#"aria-labelledby="search-heading""#), "{by}");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "has no accessible name")]
fn input_without_a_name_panics_in_debug_builds() {
    let _ = html! { Input("q") placeholder="Search"; };
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "has no accessible name")]
fn textarea_without_a_name_panics_in_debug_builds() {
    let _ = html! { Textarea("bio"); };
}

#[test]
fn input_value_from_a_variable() {
    let name = String::from("Ava \"A\"");
    let result = html! { Input("name", label = "Name") value=(name); }.into_string();
    assert!(result.contains(r#"value="Ava &quot;A&quot;""#), "{result}");
}

#[test]
fn input_builder_outside_html() {
    let result = input("q").label("Search").attrs(attrs!(required)).build().into_string();
    assert!(result.contains(r#" aria-describedby="q-error" required>"#), "{result}");
}

#[test]
fn textarea_escapes_its_value() {
    let result = html! { Textarea("bio", label = "Bio", value = "</textarea><b>x</b>") rows=(3) maxlength=(10); }.into_string();
    assert!(
        result.contains(
            r#"<textarea id="bio" name="bio" aria-describedby="bio-error" rows="3" maxlength="10">&lt;/textarea&gt;&lt;b&gt;x&lt;/b&gt;</textarea>"#
        ),
        "{result}"
    );
}

#[test]
fn textarea_hint_error_and_custom_id() {
    let result = html! { Textarea("bio", id = "about", label = "Bio", hint = "Short", error = "Too long"); }.into_string();
    assert!(result.contains(r#"<label for="about">Bio</label>"#), "{result}");
    assert!(
        result.contains(r#"<textarea id="about" name="bio" aria-describedby="about-hint about-error" aria-invalid="true">"#),
        "{result}"
    );
    assert!(result.contains(r#"<p class="hint" id="about-hint">Short</p>"#), "{result}");
    assert!(result.contains(r#"<p class="error" id="about-error" aria-live="polite">Too long</p>"#), "{result}");
}
