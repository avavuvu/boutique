use bq_components::{
    component, html,
    maud::{Markup, Render},
};

#[component]
fn labelled<T>(#[positional] label: &str, value: T) -> Markup
where
    T: Render,
{
    html! { p { (label) ": " (value) } }
}

#[test]
fn generic_component_with_where_clause_and_str_param() {
    let result = html! { Labelled("Count", value = 3); };
    assert_eq!(result.into_string(), "<p>Count: 3</p>");
}
