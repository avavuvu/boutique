use bq_components::html;

fn main() {
    let name = "href";
    html! {
        a (name)="about:blank" {}
    };
}
