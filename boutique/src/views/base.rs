use maud::{DOCTYPE, Markup};

use crate::{
    html,
    views::head::{Head, Metadata},
};

pub fn base(head: &Head, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang=(head.lang) class=[head.class.as_deref()] data-theme=[head.theme.as_deref()] {
            head {
                meta charset="UTF-8";
                meta name="viewport" content=(head.viewport);
                @if let Some(manifest) = &head.manifest {
                    link rel="manifest" href=(manifest);
                }
                @if let Some(color) = &head.theme_color {
                    meta name="theme-color" content=(color);
                }
                @if let Some(favicon) = &head.favicon {
                    link rel="icon" href=(favicon);
                }
                title { (head.title) }
                @for (href, mime) in &head.preloads {
                    link rel="preload" as="font" type=[mime] href=(href) crossorigin;
                }
                @for href in &head.stylesheets {
                    link rel="stylesheet" href=(href);
                }
                @if let Some(meta) = &head.metadata {
                    (seo_tags(&head.title, meta))
                }
                @for markup in &head.extra {
                    (markup)
                }
            }
            body {
                (content)
                @for src in &head.scripts {
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
