use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    Failure,
    fs::{io, is_rust, rust_files},
};

const BOUTIQUE_IMPORT: &str = "./boutique";

pub enum Root {
    Dir(PathBuf),
    Embedded,
}

pub struct Source {
    pub root: usize,
    pub name: String,
    pub module: Vec<String>,
    pub import: String,
    pub text: String,
}

pub struct Sources {
    pub sources: Vec<Source>,
    pub embedded: Option<usize>,
}

pub fn load(out: &Path, roots: &[Root]) -> Result<Sources, Failure> {
    let mut sources = Vec::new();
    let mut embedded = None;
    for (index, root) in roots.iter().enumerate() {
        match root {
            Root::Embedded => {
                embedded = Some(index);
                for (path, text) in bq_assets::FILES.iter().filter(|(path, _)| is_rust(path)) {
                    let path = Path::new(path);
                    sources.push(Source {
                        root: index,
                        name: format!("bq_assets/files/{}", path.display()),
                        module: module_path(path),
                        import: join_import(BOUTIQUE_IMPORT, path.parent().unwrap_or(Path::new(""))),
                        text: text.to_string(),
                    });
                }
            }
            Root::Dir(dir) => {
                let import = relative(out, dir)?;
                let mut files = Vec::new();
                rust_files(dir, &mut files)?;
                files.sort();
                for file in files {
                    let text = fs::read_to_string(&file).map_err(io(&file))?;
                    let relative = file.strip_prefix(dir).unwrap_or(&file);
                    sources.push(Source {
                        root: index,
                        name: file.display().to_string(),
                        module: module_path(relative),
                        import: join_import(&import, relative.parent().unwrap_or(Path::new(""))),
                        text,
                    });
                }
            }
        }
    }
    Ok(Sources { sources, embedded })
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

fn join_import(base: &str, folder: &Path) -> String {
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

fn relative(from: &Path, to: &Path) -> Result<String, Failure> {
    let from = fs::canonicalize(from).map_err(io(from))?;
    let to = fs::canonicalize(to).map_err(io(to))?;

    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(left, right)| left == right).count();

    let mut parts: Vec<String> = vec!["..".into(); from.len() - common];
    parts.extend(to[common..].iter().map(|part| part.as_os_str().to_string_lossy().into_owned()));

    Ok(if parts.is_empty() {
        ".".into()
    } else if parts[0] == ".." {
        parts.join("/")
    } else {
        format!("./{}", parts.join("/"))
    })
}
