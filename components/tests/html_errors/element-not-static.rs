use bq_components::html;

fn main() {
    let tag = String::from("p");
    html! {
        @element(tag) {}
    };
}
