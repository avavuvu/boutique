use boutique::prelude::{Markup, component, html};

#[component(setup = "counter")]
pub fn counter(#[default] start: i32) -> Markup {
    html! {
        div.counter bq-setup data-start=(start) {
            button type="button" aria-label="Decrease" bq-on:click="decrement" { "−" }
            output bq-ref="count" { (start) }
            button type="button" aria-label="Increase" bq-on:click="increment" { "+" }
        }
    }
}
