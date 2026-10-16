use bq_components::{
    Attrs, attrs, component, html,
    maud::{Markup, Render},
};

#[component]
fn labelled<T>(#[positional] label: &str, value: T) -> Markup
where
    T: Render,
{
    html! { p { (label) ": " (value) } }
}

#[component]
fn root(attrs: Attrs) -> Markup {
    html! { b (..attrs) {} }
}

#[test]
fn elements_call_sites_and_runtime_attrs_merge_the_same_way() {
    let refs = attrs!("bq-ref" = "spread");
    let title = attrs!(title = "t");
    let result = html! {
        b bq-ref=[Some("own")] (..refs) {}
        Root bq-ref=[Some("own")] (..refs);
        b (..attrs!("bq-ref" = "own")) (..refs) {}
        b class (..title) {}
        Root class (..title);
        b (..attrs!(class)) (..title) {}
        b class="" (..title) {}
        Root class="" (..title);
        b (..attrs!(class = "")) (..title) {}
    };
    assert_eq!(
        result.into_string(),
        concat!(
            r#"<b bq-ref="own spread"></b><b bq-ref="own spread"></b><b bq-ref="own spread"></b>"#,
            r#"<b class title="t"></b><b class title="t"></b><b class title="t"></b>"#,
            r#"<b title="t"></b><b title="t"></b><b title="t"></b>"#,
        )
    );
}

#[test]
fn generic_component_with_where_clause_and_str_param() {
    let result = html! { Labelled("Count", value = 3); };
    assert_eq!(result.into_string(), "<p>Count: 3</p>");
}
