use std::{collections::HashMap, fs, sync::OnceLock};

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

struct Config {
    route: String,
    path: String,
    cache: OnceLock<Entries>,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn read(path: &str) -> Option<Entries> {
    let text = fs::read_to_string(path).ok()?;
    let chunks: HashMap<String, Chunk> = serde_json::from_str(&text).ok()?;
    Some(
        chunks
            .into_values()
            .filter(|chunk| chunk.is_entry)
            .filter_map(|chunk| {
                let entry = Entry { file: chunk.file, styles: chunk.css };
                chunk.name.map(|name| (name, entry))
            })
            .collect(),
    )
}

/// `route` is the public URL prefix, `dir` the folder Vite writes to.
/// In debug the manifest is re-read on every lookup; in release it is read once here.
pub fn init(route: &str, dir: &str) {
    let path = format!("{dir}/.vite/manifest.json");
    let config = Config { route: route.trim_end_matches('/').to_string(), path, cache: OnceLock::new() };

    if cfg!(debug_assertions) {
        if read(&config.path).is_none() {
            eprintln!("[assets] {} not found, run the vite build first", config.path);
        }
    } else {
        let entries = read(&config.path).unwrap_or_else(|| panic!("{} not found, run the vite build before starting", config.path));
        config.cache.set(entries).ok();
    }

    CONFIG.set(config).ok();
}

fn lookup<T>(name: &str, pick: impl Fn(&str, &Entry) -> T) -> Option<T> {
    let Some(config) = CONFIG.get() else {
        eprintln!("[assets] manifest::init was not called");
        return None;
    };

    let found = if cfg!(debug_assertions) {
        read(&config.path).and_then(|entries| entries.get(name).map(|entry| pick(&config.route, entry)))
    } else {
        config.cache.get().and_then(|entries| entries.get(name).map(|entry| pick(&config.route, entry)))
    };
    if found.is_none() {
        eprintln!("[assets] no entry named {name:?} in {}", config.path);
    }
    found
}

pub fn url(name: &str) -> Option<String> {
    lookup(name, |route, entry| format!("{route}/{}", entry.file))
}

pub fn styles(name: &str) -> Vec<String> {
    lookup(name, |route, entry| entry.styles.iter().map(|file| format!("{route}/{file}")).collect()).unwrap_or_default()
}
