use bq_components::{component, html, maud::Markup};

#[component]
fn divider() -> Markup {
    html! { hr; }
}

fn main() {
    html! {
        Divider .wide;
    };
}
