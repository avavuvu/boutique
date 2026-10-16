use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub struct Source {
    pub root: usize,
    pub name: String,
    pub module: Vec<String>,
    pub import: String,
    pub text: String,
}

pub fn module_path(relative: &Path) -> Vec<String> {
    let mut parts: Vec<String> = relative
        .components()
        .filter_map(|part| match part {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let Some(last) = parts.pop() else { return parts };
    let stem = Path::new(&last)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let crate_root = parts.is_empty() && (stem == "lib" || stem == "main");
    if stem != "mod" && !crate_root {
        parts.push(stem);
    }
    parts
}

pub fn join_import(base: &str, folder: &Path) -> String {
    let mut import = base.trim_end_matches('/').to_string();
    for part in folder.components() {
        if let Component::Normal(part) = part {
            import.push('/');
            import.push_str(&part.to_string_lossy());
        }
    }
    import.push_str("/index");
    import
}

pub fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        panic!("read {}", dir.display());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}
