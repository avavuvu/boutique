use bq_components::html;

fn main() {
    let extra = bq_components::Attrs::new();
    html! {
        (..extra)
    };
}
