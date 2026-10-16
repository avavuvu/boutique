use std::{
    collections::HashMap,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde::Deserialize;

#[derive(Deserialize)]
struct Chunk {
    file: String,
    name: Option<String>,
    #[serde(default)]
    css: Vec<String>,
    #[serde(default, rename = "isEntry")]
    is_entry: bool,
}

pub struct Entry {
    pub file: String,
    pub styles: Vec<String>,
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

pub fn init(route: &str, dir: &str, live: bool) -> Result<(), ManifestError> {
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

fn lookup<T>(name: &str, pick: impl FnOnce(&str, &Entry) -> T) -> Option<T> {
    let Some(manifest) = MANIFEST.get() else {
        tracing::warn!(entry = name, "asset requested before Server::router initialised the vite manifest");
        return None;
    };

    let found = match manifest {
        Manifest::Live { route, path } => match read(path) {
            Ok(entries) => entries.get(name).map(|entry| pick(route, entry)),
            Err(error) => {
                tracing::warn!(%error, "vite manifest not available");
                return None;
            }
        },
        Manifest::Loaded { route, entries } => entries.get(name).map(|entry| pick(route, entry)),
    };
    if found.is_none() {
        tracing::warn!(entry = name, "no entry with this name in the vite manifest");
    }
    found
}

pub fn url(name: &str) -> Option<String> {
    lookup(name, |route, entry| format!("{route}/{}", entry.file))
}

pub fn styles(name: &str) -> Vec<String> {
    lookup(name, |route, entry| {
        entry.styles.iter().map(|file| format!("{route}/{file}")).collect()
    })
    .unwrap_or_default()
}
