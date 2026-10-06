use bq_components::attrs;

fn main() {
    let _ = attrs!(type = "submit", class = "a", class = "b", r#type = "button");
}
