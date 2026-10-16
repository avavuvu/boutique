use crate::server::{HTMX_PATH, manifest_entry};

#[must_use]
pub struct Head {
    pub(super) title: String,
    pub(super) scripts: Vec<String>,
    pub(super) preloads: Vec<(String, Option<&'static str>)>,
    pub(super) stylesheets: Vec<String>,
    pub(super) favicon: Option<String>,
    pub(super) metadata: Option<Metadata>,
    pub(super) class: Option<String>,
    pub(super) lang: String,
    pub(super) theme: Option<String>,
    pub(super) viewport: String,
    pub(super) manifest: Option<String>,
    pub(super) theme_color: Option<String>,
}

impl Head {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            scripts: Vec::new(),
            stylesheets: Vec::new(),
            preloads: Vec::new(),
            favicon: None,
            metadata: None,
            class: None,
            lang: "en".into(),
            theme: None,
            viewport: "width=device-width, initial-scale=1.0".into(),
            manifest: None,
            theme_color: None,
        }
    }

    pub fn viewport(mut self, content: impl Into<String>) -> Self {
        self.viewport = content.into();
        self
    }

    pub fn manifest(mut self, href: impl Into<String>) -> Self {
        self.manifest = Some(href.into());
        self
    }

    pub fn theme_color(mut self, color: impl Into<String>) -> Self {
        self.theme_color = Some(color.into());
        self
    }

    pub fn preload_font(mut self, href: impl Into<String>) -> Self {
        let href = href.into();
        let mime = font_type(&href);
        self.preloads.push((href, mime));
        self
    }

    pub fn module(mut self, src: impl Into<String>) -> Self {
        self.scripts.push(src.into());
        self
    }

    pub fn stylesheet(mut self, href: impl Into<String>) -> Self {
        self.stylesheets.push(href.into());
        self
    }

    pub fn entry(mut self, name: &str) -> Self {
        let Some(asset) = manifest_entry(name) else {
            return self;
        };
        self.stylesheets.extend(asset.styles);
        self.module(asset.file)
    }

    pub fn entry_css(self, name: &str) -> Self {
        match manifest_entry(name) {
            Some(asset) => self.stylesheet(asset.file),
            None => self,
        }
    }

    pub fn favicon(mut self, href: impl Into<String>) -> Self {
        self.favicon = Some(href.into());
        self
    }

    pub fn htmx(self) -> Self {
        self.module(HTMX_PATH)
    }

    pub fn seo(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn class(mut self, name: impl Into<String>) -> Self {
        self.class = Some(name.into());
        self
    }

    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self
    }

    /// rendered as `data-theme` on `<html>`, for css theme blocks
    pub fn theme(mut self, name: impl Into<String>) -> Self {
        self.theme = Some(name.into());
        self
    }
}

fn font_type(href: &str) -> Option<&'static str> {
    let path = href.split(['?', '#']).next().unwrap_or(href);
    let (_, extension) = path.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "woff2" => Some("font/woff2"),
        "woff" => Some("font/woff"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        _ => None,
    }
}

pub enum OgType {
    Website,
    Article,
}

impl OgType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OgType::Website => "website",
            OgType::Article => "article",
        }
    }
}

#[must_use]
pub struct Metadata {
    pub(super) description: String,
    pub(super) og_type: OgType,
    pub(super) image: Option<String>,
    pub(super) author: Option<String>,
    pub(super) url: Option<String>,
    pub(super) modified_time: Option<String>,
}

impl Metadata {
    pub fn website(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            og_type: OgType::Website,
            image: None,
            author: None,
            url: None,
            modified_time: None,
        }
    }

    pub fn article(description: impl Into<String>, author: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            og_type: OgType::Article,
            author: Some(author.into()),
            url: Some(url.into()),
            image: None,
            modified_time: None,
        }
    }

    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image = Some(url.into());
        self
    }

    pub fn modified_time(mut self, time: impl Into<String>) -> Self {
        self.modified_time = Some(time.into());
        self
    }
}
