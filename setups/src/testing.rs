use std::path::Path;

use crate::{
    Failure, analyse_sources,
    sources::{self, module_path},
};

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
    let strings = |diagnostics: Vec<crate::Diagnostic>| diagnostics.iter().map(ToString::to_string).collect();
    match analyse_sources(runtime, &sources, None) {
        Ok(output) => Ok((output.types, output.registry, strings(output.warnings))),
        Err(Failure::Invalid { errors, .. }) => Err(strings(errors)),
        Err(failure) => Err(vec![failure.to_string()]),
    }
}
