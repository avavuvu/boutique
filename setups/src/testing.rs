use std::path::Path;

use crate::sources::{self, module_path};

pub struct Source {
    pub root: usize,
    pub name: String,
    pub import: String,
    pub text: String,
}

pub fn analyse(runtime: &str, sources: &[Source]) -> Result<(String, String, Vec<String>), Vec<String>> {
    let sources: Vec<sources::Source> = sources
        .iter()
        .map(|source| sources::Source {
            root: source.root,
            name: source.name.clone(),
            module: module_path(Path::new(&source.name)),
            import: source.import.clone(),
            text: source.text.clone(),
        })
        .collect();
    crate::analyse_sources(runtime, &sources, None)
}
