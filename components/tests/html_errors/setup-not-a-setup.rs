use bq_components::html;

struct NotASetup;

fn main() {
    html! {
        div bq-setup=(NotASetup) {}
    };
}
