use crate::assets;

pub struct Head {
    pub title: String,
    pub scripts: Vec<String>,
    pub preloads: Vec<(String, &'static str)>,
    pub stylesheets: Vec<String>,
    pub favicon: Option<String>,
    pub metadata: Option<Metadata>,
    pub class: Option<String>,
    pub theme: Option<String>,
    alpine_entry: String,
    islands_entry: String,
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
            theme: None,
            alpine_entry: "/assets/alpine.js".into(),
            islands_entry: "/assets/islands.js".into(),
        }
    }

    pub fn preload_font(mut self, href: impl Into<String>) -> Self {
        self.preloads.push((href.into(), "font/woff2"));
        self
    }

    pub fn alpine_entry(mut self, src: impl Into<String>) -> Self {
        self.alpine_entry = src.into();
        self
    }

    pub fn islands_entry(mut self, src: impl Into<String>) -> Self {
        self.islands_entry = src.into();
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

    pub fn favicon(mut self, href: impl Into<String>) -> Self {
        self.favicon = Some(href.into());
        self
    }

    pub fn htmx(self) -> Self {
        self.module(assets::HTMX_PATH)
    }

    /// the compat extension must load after htmx and before alpine
    pub fn alpine(self) -> Self {
        let entry = self.alpine_entry.clone();
        self.module(assets::ALPINE_PATH).module(entry)
    }

    /// the loader must come before the app entry that calls `mountIslands`
    pub fn islands(self) -> Self {
        let entry = self.islands_entry.clone();
        self.module(assets::ISLANDS_PATH).module(entry)
    }

    pub fn seo(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn class(mut self, name: String) -> Self {
        self.class = Some(name);
        self
    }

    /// rendered as `data-theme` on `<html>`, for css theme blocks
    pub fn theme(mut self, name: impl Into<String>) -> Self {
        self.theme = Some(name.into());
        self
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

pub struct Metadata {
    pub description: String,
    pub og_type: OgType,
    pub image: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub modified_time: Option<String>,
}

impl Metadata {
    pub fn website(description: &str) -> Self {
        Self {
            description: description.to_string(),
            og_type: OgType::Website,
            image: None,
            author: None,
            url: None,
            modified_time: None,
        }
    }

    pub fn article(description: &str, author: &str, url: &str) -> Self {
        Self {
            description: description.to_string(),
            og_type: OgType::Article,
            author: Some(author.to_string()),
            url: Some(url.to_string()),
            image: None,
            modified_time: None,
        }
    }

    pub fn with_image(mut self, url: &str) -> Self {
        self.image = Some(url.to_string());
        self
    }

    pub fn with_modified_time(mut self, time: &str) -> Self {
        self.modified_time = Some(time.to_string());
        self
    }
}
