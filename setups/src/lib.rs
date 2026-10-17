mod attrs;
mod diagnostics;
mod emit;
mod fs;
mod html;
mod index;
mod model;
mod sources;
mod walk;

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, HashMap},
    error, fmt, io,
    path::PathBuf,
    process,
};

use bq_html::conventions::ts_type_name;

use diagnostics::Sink;
pub use diagnostics::{Diagnostic, Level, Loc};
use index::Index;
use model::Declaration;
use sources::Root;
#[cfg(test)]
use testing::{Source, analyse};

const BOUTIQUE: &str = "boutique";
const RUNTIME: &str = "./boutique/setups";

pub struct Generator {
    out: PathBuf,
    roots: Vec<Root>,
}

#[derive(Debug)]
pub struct Report {
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum Failure {
    Invalid {
        errors: Vec<Diagnostic>,
        warnings: Vec<Diagnostic>,
    },
    Io {
        path: PathBuf,
        error: io::Error,
    },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Invalid { errors, .. } => {
                for (index, error) in errors.iter().enumerate() {
                    if index > 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{error}")?;
                }
                Ok(())
            }
            Failure::Io { path, error } => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl error::Error for Failure {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Failure::Invalid { .. } => None,
            Failure::Io { error, .. } => Some(error),
        }
    }
}

impl Generator {
    pub fn new(out: impl Into<PathBuf>) -> Self {
        Self {
            out: out.into(),
            roots: Vec::new(),
        }
    }

    pub fn source(mut self, dir: impl Into<PathBuf>) -> Self {
        self.roots.push(Root::Dir(dir.into()));
        self
    }

    pub fn boutique(mut self) -> Self {
        self.roots.push(Root::Embedded);
        self
    }

    pub fn try_write(&self) -> Result<Report, Failure> {
        fs::create_dir(&self.out)?;
        let sources = sources::load(&self.out, &self.roots)?;
        let output = analyse_sources(RUNTIME, &sources.sources, sources.embedded)?;

        let kit = self.roots.iter().any(|root| matches!(root, Root::Embedded));
        fs::write_boutique(&self.out.join(BOUTIQUE), kit)?;
        fs::write_if_changed(&self.out.join("setups.ts"), &output.types)?;
        fs::write_if_changed(&self.out.join("registry.ts"), &output.registry)?;
        Ok(Report { warnings: output.warnings })
    }

    pub fn write(self) {
        for root in &self.roots {
            if let Root::Dir(dir) = root {
                println!("cargo:rerun-if-changed={}", dir.display());
            }
        }

        match self.try_write() {
            Ok(report) => print_warnings(&report.warnings),
            Err(Failure::Invalid { errors, warnings }) => {
                print_warnings(&warnings);
                for error in &errors {
                    eprintln!("error: {error}");
                }
                process::exit(1);
            }
            Err(failure) => {
                eprintln!("error: {failure}");
                process::exit(1);
            }
        }
    }
}

fn print_warnings(warnings: &[Diagnostic]) {
    for warning in warnings {
        println!("cargo:warning={warning}");
    }
}

struct Output {
    types: String,
    registry: String,
    warnings: Vec<Diagnostic>,
}

fn analyse_sources(runtime: &str, sources: &[sources::Source], embedded: Option<usize>) -> Result<Output, Failure> {
    let mut sink = Sink::default();
    let index = Index::build(sources, embedded, &mut sink);
    let usages = walk::walk(&index, &mut sink);
    let declarations = declarations(&index, &mut sink);
    let setups = model::merge(&usages, &declarations);

    for declaration in declarations.values() {
        if !setups.contains_key(&declaration.name) && !index.functions[declaration.function_id].unparsed {
            sink.warning(
                &declaration.loc,
                format!(
                    "the bare bq-setup of `{}` is not reached by bq_setups, so the setup \"{}\" has no tags, refs or handlers",
                    declaration.function, declaration.name
                ),
                None,
            );
        }
    }

    let failed = sink.has_errors();
    let (errors, warnings) = sink.finish();
    if failed {
        return Err(Failure::Invalid { errors, warnings });
    }

    let mut ordered: Vec<&Declaration> = declarations.into_values().collect();
    ordered.sort_by_cached_key(|declaration| ts_type_name(&declaration.name));
    Ok(Output {
        types: emit::types(runtime, &ordered, &setups),
        registry: emit::registry(runtime, &ordered, &setups),
        warnings,
    })
}

fn declarations<'i>(index: &'i Index, sink: &mut Sink) -> BTreeMap<String, &'i Declaration> {
    let mut by_name = BTreeMap::new();
    let mut by_type: HashMap<String, &Declaration> = HashMap::new();
    for declaration in &index.declarations {
        let ts_type = ts_type_name(&declaration.name);
        if let Some(existing) = by_type.get(&ts_type) {
            let clash = if existing.name == declaration.name {
                format!("two setups are named \"{}\"", declaration.name)
            } else {
                format!(
                    "the setups \"{}\" and \"{}\" both have the TypeScript type {ts_type}",
                    existing.name, declaration.name
                )
            };
            sink.error(
                &declaration.loc,
                format!(
                    "{clash}: `{}` at {} and `{}` at {}; rename one with #[component(setup = \"…\")]",
                    existing.function, existing.loc, declaration.function, declaration.loc
                ),
                None,
            );
            continue;
        }
        by_type.insert(ts_type, declaration);
        by_name.insert(declaration.name.clone(), declaration);
    }
    by_name
}
