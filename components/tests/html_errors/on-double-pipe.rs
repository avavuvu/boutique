use bq_components::html;

fn main() {
    html! {
        button bq-on:click||prevent="save" {}
    };
}
