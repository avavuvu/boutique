use bq_components::html;

fn main() {
    let missing = 1;
    html! {
        p { Missing("x") {} }
    };
}
