use boutique::{
    components::{Button, Input, Kind},
    prelude::{Markup, html},
};

use crate::views::layouts::{head, shell};

pub fn sign_up() -> Markup {
    form_page(
        "Sign up",
        "/sign-up",
        "new-password",
        html! { "Already have an account? " a href="/sign-in" { "Sign in" } },
    )
}

pub fn sign_in() -> Markup {
    form_page(
        "Sign in",
        "/sign-in",
        "current-password",
        html! { "No account yet? " a href="/sign-up" { "Sign up" } },
    )
}

fn form_page(title: &str, action: &str, password_autocomplete: &str, switch: Markup) -> Markup {
    shell(
        head(title).htmx(),
        None,
        html! {
            section.auth.flow {
                h1 { (title) }
                form.flow method="POST" action=(action) hx-post=(action) hx-target="#form-error" hx-swap="innerHTML" {
                    Input("email", kind = Kind::Email, label = "Email") required;
                    Input("password", kind = Kind::Password, label = "Password") autocomplete=(password_autocomplete) required;
                    div.error #form-error aria-live="polite" {}
                    Button(submit = true) .primary { (title) }
                }
                p { (switch) }
            }
        },
    )
}
