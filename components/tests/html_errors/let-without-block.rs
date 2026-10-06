use bq_components::html;

fn main() {
    html! {
        p.@let x = 1; {
            (x)
        }
    };
}
