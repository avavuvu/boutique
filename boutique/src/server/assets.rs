use std::{
    collections::HashMap,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use axum::{Router, http::header, response::IntoResponse, routing::get};
use serde::Deserialize;

pub(crate) const HTMX_PATH: &str = concat!("/boutique/htmx-", env!("BOUTIQUE_HTMX_HASH"), ".js");
pub(crate) const BUILD_ROUTE: &str = "/build";
pub(crate) const BUILD_DIR: &str = "public/build";

fn script(body: &'static str) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        body,
    )
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route(HTMX_PATH, get(|| async { script(include_str!("../../assets/htmx.js")) }))
}

#[derive(Deserialize)]
struct Chunk {
    file: String,
    name: Option<String>,
    #[serde(default)]
    css: Vec<String>,
    #[serde(default, rename = "isEntry")]
    is_entry: bool,
}

struct Entry {
    file: String,
    styles: Vec<String>,
}

pub(crate) struct Asset {
    pub(crate) file: String,
    pub(crate) styles: Vec<String>,
}

type Entries = HashMap<String, Entry>;

enum Manifest {
    Live { route: String, path: PathBuf },
    Loaded { route: String, entries: Entries },
}

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

#[derive(Debug)]
pub enum ManifestError {
    Read { path: PathBuf, source: io::Error },
    Parse { path: PathBuf, source: serde_json::Error },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Read { path, .. } => write!(f, "could not read {}, run the vite build first", path.display()),
            ManifestError::Parse { path, .. } => write!(f, "could not parse {}", path.display()),
        }
    }
}

impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ManifestError::Read { source, .. } => Some(source),
            ManifestError::Parse { source, .. } => Some(source),
        }
    }
}

fn read(path: &Path) -> Result<Entries, ManifestError> {
    let text = fs::read_to_string(path).map_err(|source| ManifestError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let chunks: HashMap<String, Chunk> = serde_json::from_str(&text).map_err(|source| ManifestError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(chunks
        .into_values()
        .filter(|chunk| chunk.is_entry)
        .filter_map(|chunk| {
            let entry = Entry {
                file: chunk.file,
                styles: chunk.css,
            };
            chunk.name.map(|name| (name, entry))
        })
        .collect())
}

pub(crate) fn init(route: &str, dir: &str, live: bool) -> Result<(), ManifestError> {
    let route = route.trim_end_matches('/').to_string();
    let path = Path::new(dir).join(".vite/manifest.json");

    let manifest = if live {
        if let Err(error) = read(&path) {
            tracing::warn!(%error, "vite manifest not available yet");
        }
        Manifest::Live { route, path }
    } else {
        let entries = read(&path)?;
        Manifest::Loaded { route, entries }
    };

    if MANIFEST.set(manifest).is_err() {
        tracing::debug!("vite manifest already initialised");
    }
    Ok(())
}

fn asset(route: &str, entry: &Entry) -> Asset {
    Asset {
        file: format!("{route}/{}", entry.file),
        styles: entry.styles.iter().map(|file| format!("{route}/{file}")).collect(),
    }
}

pub(crate) fn entry(name: &str) -> Option<Asset> {
    let Some(manifest) = MANIFEST.get() else {
        tracing::warn!(entry = name, "asset requested before Server::router initialised the vite manifest");
        return None;
    };

    let found = match manifest {
        Manifest::Live { route, path } => match read(path) {
            Ok(entries) => entries.get(name).map(|entry| asset(route, entry)),
            Err(error) => {
                tracing::warn!(%error, "vite manifest not available");
                return None;
            }
        },
        Manifest::Loaded { route, entries } => entries.get(name).map(|entry| asset(route, entry)),
    };
    if found.is_none() {
        tracing::warn!(entry = name, "no entry with this name in the vite manifest");
    }
    found
}
