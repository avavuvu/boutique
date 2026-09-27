use maud::{DOCTYPE, Markup, html};

use crate::views::head::{Metadata, Head};

pub fn base(context: &Head, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" class=[context.class.as_deref()] data-theme=[context.theme.as_deref()] {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                @if let Some(favicon) = &context.favicon {
                    link rel="icon" type="image/x-icon" href=(favicon);
                }
                title { (context.title) }
                @for (href, mime) in &context.preloads {
                    link rel="preload" as="font" type=(mime) href=(href) crossorigin;
                }
                @for href in &context.stylesheets {
                    link rel="stylesheet" href=(href);
                }
                @if let Some(meta) = &context.metadata {
                    (seo_tags(&context.title, meta))
                }
            }
            body {
                (content)
                @for src in &context.scripts {
                    script type="module" src=(src) {}
                }
            }
        }
    }
}

fn seo_tags(title: &str, meta: &Metadata) -> Markup {
    let twitter_card = if meta.image.is_some() { "summary_large_image" } else { "summary" };

    html! {
        meta name="description" content=(meta.description);
        meta property="og:type" content=(meta.og_type.as_str());
        meta property="og:title" content=(title);
        meta property="og:description" content=(meta.description);
        meta name="twitter:card" content=(twitter_card);
        meta name="twitter:title" content=(title);
        meta name="twitter:description" content=(meta.description);
        @if let Some(image) = &meta.image {
            meta property="og:image" content=(image);
            meta name="twitter:image" content=(image);
        }
        @if let Some(author) = &meta.author {
            meta name="author" content=(author);
        }
        @if let Some(url) = &meta.url {
            meta property="og:url" content=(url);
            link rel="canonical" href=(url);
        }
        @if let Some(time) = &meta.modified_time {
            meta property="article:modified_time" content=(time);
        }
    }
}
