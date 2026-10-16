mod index;
mod on;
mod sources;
mod walk;

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use heck::{ToLowerCamelCase, ToUpperCamelCase};

use index::Index;
use on::Target;
use sources::{join_import, module_path, rust_files};
#[cfg(test)]
use testing::{Source, analyse};
use walk::{Tags, Usage, Walked, Walker};

const BOUTIQUE: &str = "boutique";
const RUNTIME: &str = "./boutique/setups";
const RUNTIME_FILE: &str = "setups.ts";

struct Root {
    dir: Option<PathBuf>,
    import: String,
}

struct Declaration {
    name: String,
    function: String,
    function_id: usize,
    eager: bool,
    import: String,
    location: String,
}

#[derive(Default)]
struct Merged {
    tags: BTreeSet<String>,
    any: bool,
    refs: BTreeMap<String, MergedRef>,
    handlers: BTreeMap<String, BTreeSet<(String, Target, Option<String>)>>,
    usages: usize,
}

#[derive(Default)]
struct MergedRef {
    tags: BTreeSet<String>,
    any: bool,
    optional: bool,
    many: bool,
    usages: usize,
}

pub struct Generator {
    out: PathBuf,
    roots: Vec<Root>,
}

impl Generator {
    pub fn new(out: impl Into<PathBuf>) -> Self {
        Self {
            out: out.into(),
            roots: Vec::new(),
        }
    }

    pub fn source(mut self, dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        fs::create_dir_all(&self.out).expect("create the bindings folder");
        let import = relative(&self.out, &dir);
        self.roots.push(Root { dir: Some(dir), import });
        self
    }

    pub fn boutique(mut self) -> Self {
        self.roots.push(Root {
            dir: None,
            import: format!("./{BOUTIQUE}"),
        });
        self
    }

    pub fn write(self) {
        for dir in self.roots.iter().filter_map(|root| root.dir.as_ref()) {
            println!("cargo:rerun-if-changed={}", dir.display());
        }

        let (types, registry, warnings) = match self.generate() {
            Ok(output) => output,
            Err(errors) => panic!("\n{}\n", errors.join("\n")),
        };

        for warning in warnings {
            println!("cargo:warning={warning}");
        }

        fs::create_dir_all(&self.out).expect("create the bindings folder");
        let kit = self.roots.iter().any(|root| root.dir.is_none());
        write_boutique(&self.out.join(BOUTIQUE), kit);
        write_if_changed(&self.out.join("setups.ts"), &types);
        write_if_changed(&self.out.join("registry.ts"), &registry);
    }

    fn generate(&self) -> Result<(String, String, Vec<String>), Vec<String>> {
        let mut sources = Vec::new();
        let mut embedded = None;
        for (index, root) in self.roots.iter().enumerate() {
            let Some(dir) = &root.dir else {
                embedded = Some(index);
                for (path, text) in bq_components::FILES.iter().filter(|(path, _)| is_rust(path)) {
                    let folder = Path::new(path).parent().unwrap_or(Path::new(""));
                    sources.push(sources::Source {
                        root: index,
                        name: format!("bq_components/src/{path}"),
                        module: module_path(Path::new(path)),
                        import: join_import(&root.import, folder),
                        text: text.to_string(),
                    });
                }
                continue;
            };

            let mut files = Vec::new();
            rust_files(dir, &mut files);
            files.sort();

            for file in files {
                let text = fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
                let folder = file.parent().unwrap_or(dir).strip_prefix(dir).unwrap_or(Path::new(""));
                sources.push(sources::Source {
                    root: index,
                    name: file.display().to_string(),
                    module: module_path(file.strip_prefix(dir).unwrap_or(&file)),
                    import: join_import(&root.import, folder),
                    text,
                });
            }
        }
        analyse_sources(RUNTIME, &sources, embedded)
    }
}

fn analyse_sources(
    runtime: &str,
    sources: &[sources::Source],
    embedded: Option<usize>,
) -> Result<(String, String, Vec<String>), Vec<String>> {
    let mut index = Index::new(embedded);
    let mut errors = Vec::new();

    for source in sources {
        let parsed = match syn::parse_file(&source.text) {
            Ok(parsed) => parsed,
            Err(error) => {
                errors.push(format!("{}: {error}", source.name));
                continue;
            }
        };
        index.add_file(
            source.name.clone(),
            source.root,
            source.module.clone(),
            source.import.clone(),
            &parsed,
        );
    }
    errors.append(&mut index.errors);

    let walked = walk(&index);
    errors.extend(walked.errors);
    let mut warnings = std::mem::take(&mut index.warnings);
    warnings.extend(walked.warnings);

    let by_name = declarations(&index, &mut errors);
    let merged = merge(&walked.usages, &by_name);

    for (name, declaration) in &by_name {
        let function = declaration.function_id;
        let unparsed = index.functions[function].unparsed || walked.unparsed.contains(&function);
        if !merged.contains_key(name) && !unparsed {
            warnings.push(format!(
                "{}: the bare bq-setup of `{}` is not reached by bq_setups, so the setup \"{}\" has no tags, refs or handlers",
                declaration.location, declaration.function, declaration.name
            ));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok((types(runtime, &by_name, &merged), registry(runtime, &by_name, &merged), warnings))
}

fn walk(index: &Index) -> Walked {
    let components: Vec<usize> = index
        .functions
        .iter()
        .enumerate()
        .filter(|(_, function)| function.component)
        .map(|(function, _)| function)
        .collect();

    let called = {
        let mut probe = Walker::new(index);
        walk_blocks(index, &mut probe);
        for &function in &components {
            probe.component_alone(function);
        }
        probe.reached
    };

    let mut walker = Walker::new(index);
    walk_blocks(index, &mut walker);
    for &function in &components {
        if !called.contains(&function) {
            walker.component_alone(function);
        }
    }
    for &function in &components {
        if called.contains(&function) && !walker.reached.contains(&function) {
            walker.component_alone(function);
        }
    }
    walker.finish()
}

fn walk_blocks(index: &Index, walker: &mut Walker) {
    for (block, html) in index.blocks.iter().enumerate() {
        let inside = html.function.is_some_and(|function| index.functions[function].component);
        if !inside && html.binding.is_none() {
            walker.block(block);
        }
    }
    for (function, item) in index.functions.iter().enumerate() {
        if !item.component {
            walker.unspliced_alone(function);
        }
    }
}

fn declarations(index: &Index, errors: &mut Vec<String>) -> BTreeMap<String, Declaration> {
    let mut by_type: BTreeMap<String, Declaration> = BTreeMap::new();
    for setup in &index.setups {
        let file = &index.files[setup.file];
        let declaration = Declaration {
            name: setup.name.clone(),
            function: setup.function.clone(),
            function_id: setup.function_id,
            eager: setup.eager,
            import: file.import.clone(),
            location: format!("{}:{}", file.name, setup.line),
        };
        let key = setup.name.to_upper_camel_case();
        if let Some(existing) = by_type.get(&key) {
            let clash = if existing.name == declaration.name {
                format!("two setups are named \"{}\"", declaration.name)
            } else {
                format!(
                    "the setups \"{}\" and \"{}\" both have the TypeScript type {key}",
                    existing.name, declaration.name
                )
            };
            errors.push(format!(
                "{}: {clash}: `{}` at {} and `{}` at {}; rename one with #[component(setup = \"…\")]",
                declaration.location, existing.function, existing.location, declaration.function, declaration.location
            ));
            continue;
        }
        by_type.insert(key, declaration);
    }
    by_type
}

fn types(runtime: &str, declarations: &BTreeMap<String, Declaration>, merged: &BTreeMap<String, Merged>) -> String {
    let mut out = format!("import type {{ Setup }} from {};\n\n", ts_str(runtime));

    for name in declarations.keys() {
        let empty = Merged::default();
        let setup = merged.get(name).unwrap_or(&empty);
        let tags = tag_union(&setup.tags, setup.any);

        if setup.refs.is_empty() && setup.handlers.is_empty() {
            out.push_str(&format!("export type {name} = Setup<{tags}>;\n"));
            continue;
        }

        let refs: Vec<String> = setup
            .refs
            .iter()
            .map(|(key, reference)| {
                let tags = tag_union(&reference.tags, reference.any);
                if reference.many {
                    format!("{key}: ({tags})[]")
                } else if reference.optional {
                    format!("{key}?: {tags}")
                } else {
                    format!("{key}: {tags}")
                }
            })
            .collect();
        let refs = if refs.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {} }}", refs.join("; "))
        };

        if setup.handlers.is_empty() {
            out.push_str(&format!("export type {name} = Setup<{tags}, {refs}>;\n"));
            continue;
        }

        let handlers: Vec<String> = setup
            .handlers
            .iter()
            .map(|(handler, uses)| {
                let uses: Vec<String> = uses
                    .iter()
                    .map(|(event, target, tag)| {
                        let tag = tag.as_deref().map(ts_str).unwrap_or_else(|| "string".into());
                        let target = target
                            .name()
                            .map(|target| format!("; target: {}", ts_str(target)))
                            .unwrap_or_default();
                        format!("{{ event: {}; tag: {tag}{target} }}", ts_str(event))
                    })
                    .collect();
                format!("{handler}: {}", uses.join(" | "))
            })
            .collect();
        out.push_str(&format!(
            "export type {name} = Setup<{tags}, {refs}, {{ {} }}>;\n",
            handlers.join("; ")
        ));
    }

    out
}

fn registry(runtime: &str, declarations: &BTreeMap<String, Declaration>, merged: &BTreeMap<String, Merged>) -> String {
    let mut out = format!(
        "import type {{ Registry }} from {};\nimport type * as setups from {};\n",
        ts_str(runtime),
        ts_str("./setups")
    );

    let mut eager: Vec<(&str, Vec<String>)> = Vec::new();
    for declaration in declarations.values().filter(|declaration| declaration.eager) {
        let export = declaration.name.to_lower_camel_case();
        match eager.iter_mut().find(|(import, _)| *import == declaration.import) {
            Some((_, exports)) => exports.push(export),
            None => eager.push((&declaration.import, vec![export])),
        }
    }
    for (import, exports) in eager {
        out.push_str(&format!("import {{ {} }} from {};\n", exports.join(", "), ts_str(import)));
    }

    out.push_str("\nexport const registry: Registry = {\n");
    for (name, declaration) in declarations {
        let empty = Merged::default();
        let setup = merged.get(name).unwrap_or(&empty);
        let export = declaration.name.to_lower_camel_case();
        let tags = if setup.any { String::new() } else { quoted_list(setup.tags.iter()) };
        let many = quoted_list(setup.refs.iter().filter(|(_, reference)| reference.many).map(|(key, _)| key));

        out.push_str(&format!("    {}: {{\n", ts_str(&declaration.name)));
        out.push_str(&format!("        tags: [{tags}],\n"));
        out.push_str(&format!("        many: [{many}],\n"));
        if declaration.eager {
            out.push_str(&format!("        setup: {export} satisfies setups.{name},\n"));
        } else {
            out.push_str(&format!(
                "        load: () => import({}).then((module) => module.{export} satisfies setups.{name}),\n",
                ts_str(&declaration.import)
            ));
        }
        out.push_str("    },\n");
    }
    out.push_str("};\n");

    out
}

fn merge(usages: &[Usage], declarations: &BTreeMap<String, Declaration>) -> BTreeMap<String, Merged> {
    let types: HashMap<&str, &str> = declarations
        .iter()
        .map(|(key, declaration)| (declaration.name.as_str(), key.as_str()))
        .collect();
    let mut merged: BTreeMap<String, Merged> = BTreeMap::new();

    for usage in usages {
        let Some(&key) = types.get(usage.name.as_str()) else { continue };
        let setup = merged.entry(key.to_string()).or_default();
        setup.usages += 1;
        add_tags(&usage.tags, &mut setup.tags, &mut setup.any);

        for handler in &usage.handlers {
            let uses = setup.handlers.entry(handler.name.clone()).or_default();
            match &handler.tags {
                Tags::Known(tags) => {
                    for tag in tags {
                        uses.insert((handler.event.clone(), handler.target.clone(), Some(tag.clone())));
                    }
                }
                Tags::Unknown => {
                    uses.insert((handler.event.clone(), handler.target.clone(), None));
                }
            }
        }

        let mut seen = BTreeSet::new();
        for reference in &usage.refs {
            let first = seen.insert(reference.name.clone());
            let entry = setup.refs.entry(reference.name.clone()).or_default();
            add_tags(&reference.tags, &mut entry.tags, &mut entry.any);
            entry.optional |= reference.optional;
            entry.many |= reference.many || !first;
            if first {
                entry.usages += 1;
            }
        }
    }

    for setup in merged.values_mut() {
        for reference in setup.refs.values_mut() {
            if reference.usages < setup.usages {
                reference.optional = true;
            }
        }
    }

    merged
}

fn add_tags(tags: &Tags, into: &mut BTreeSet<String>, any: &mut bool) {
    match tags {
        Tags::Known(tags) => into.extend(tags.iter().cloned()),
        Tags::Unknown => *any = true,
    }
}

fn tag_union(tags: &BTreeSet<String>, any: bool) -> String {
    if any || tags.is_empty() {
        return "string".into();
    }
    tags.iter().map(|tag| ts_str(tag)).collect::<Vec<_>>().join(" | ")
}

fn quoted_list<'a>(items: impl Iterator<Item = &'a String>) -> String {
    items.map(|item| ts_str(item)).collect::<Vec<_>>().join(", ")
}

fn ts_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' | '\u{2029}' => out.push_str(&format!("\\u{:04x}", u32::from(character))),
            character if character.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(character))),
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

fn relative(from: &Path, to: &Path) -> String {
    let from = fs::canonicalize(from).unwrap_or_else(|error| panic!("{}: {error}", from.display()));
    let to = fs::canonicalize(to).unwrap_or_else(|error| panic!("{}: {error}", to.display()));

    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(left, right)| left == right).count();

    let mut parts: Vec<String> = vec!["..".into(); from.len() - common];
    parts.extend(to[common..].iter().map(|part| part.as_os_str().to_string_lossy().into_owned()));

    if parts.is_empty() {
        ".".into()
    } else if parts[0] == ".." {
        parts.join("/")
    } else {
        format!("./{}", parts.join("/"))
    }
}

fn is_rust(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|extension| extension == "rs")
}

fn write_boutique(dir: &Path, kit: bool) {
    let mut keep = BTreeSet::new();
    for (path, text) in bq_components::FILES
        .iter()
        .filter(|(path, _)| !is_rust(path) && (kit || *path == RUNTIME_FILE))
    {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
        }
        write_if_changed(&target, text);
        keep.insert(target);
    }
    remove_stale(dir, &keep);
}

fn remove_stale(dir: &Path, keep: &BTreeSet<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_dir {
            remove_stale(&path, keep);
            if fs::read_dir(&path).is_ok_and(|mut inside| inside.next().is_none()) {
                fs::remove_dir(&path).unwrap_or_else(|error| panic!("remove {}: {error}", path.display()));
            }
        } else if !keep.contains(&path) {
            fs::remove_file(&path).unwrap_or_else(|error| panic!("remove {}: {error}", path.display()));
        }
    }
}

fn write_if_changed(path: &Path, content: &str) {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return;
    }
    fs::write(path, content).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}
