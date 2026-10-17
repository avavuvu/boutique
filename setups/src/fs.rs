use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use crate::Failure;

const RUNTIME_FILE: &str = "setups.ts";
const CONFIG_DIR: &str = "config/";

pub fn io(path: &Path) -> impl FnOnce(io::Error) -> Failure + '_ {
    move |error| Failure::Io {
        path: path.to_path_buf(),
        error,
    }
}

pub fn create_dir(path: &Path) -> Result<(), Failure> {
    fs::create_dir_all(path).map_err(io(path))
}

pub fn write_if_changed(path: &Path, content: &str) -> Result<(), Failure> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }
    fs::write(path, content).map_err(io(path))
}

pub fn write_boutique(dir: &Path, kit: bool) -> Result<(), Failure> {
    let mut keep = BTreeSet::new();
    let files = bq_assets::FILES
        .iter()
        .filter(|(path, _)| !is_rust(path) && (kit || *path == RUNTIME_FILE || path.starts_with(CONFIG_DIR)));
    for (path, text) in files {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            create_dir(parent)?;
        }
        write_if_changed(&target, text)?;
        keep.insert(target);
    }
    remove_stale(dir, &keep)
}

pub fn is_rust(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|extension| extension == "rs")
}

pub fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Failure> {
    for entry in fs::read_dir(dir).map_err(io(dir))? {
        let path = entry.map_err(io(dir))?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if is_rust(&path.to_string_lossy()) {
            out.push(path);
        }
    }
    Ok(())
}

fn remove_stale(dir: &Path, keep: &BTreeSet<PathBuf>) -> Result<(), Failure> {
    for entry in fs::read_dir(dir).map_err(io(dir))? {
        let entry = entry.map_err(io(dir))?;
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            remove_stale(&path, keep)?;
            if fs::read_dir(&path).is_ok_and(|mut inside| inside.next().is_none()) {
                fs::remove_dir(&path).map_err(io(&path))?;
            }
        } else if !keep.contains(&path) {
            fs::remove_file(&path).map_err(io(&path))?;
        }
    }
    Ok(())
}
