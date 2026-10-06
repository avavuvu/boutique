use bq_components::html;

fn main() {
    html! {
        p title={ @element("b") {} } {}
    };
}
