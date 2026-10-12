mod index;
mod on;
mod walk;

#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Component, Path, PathBuf},
};

use heck::{ToKebabCase, ToLowerCamelCase};
use proc_macro2::{TokenStream, TokenTree};
use syn::visit::Visit;

use index::Index;
use on::Target;
use walk::{Claim, Kind, Tags, Usage, Walker};

struct Root {
    dir: PathBuf,
    import: String,
}

struct Source {
    root: usize,
    name: String,
    import: String,
    text: String,
}

struct Declaration {
    name: String,
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
    runtime: String,
}

impl Generator {
    pub fn new(out: impl Into<PathBuf>) -> Self {
        Self { out: out.into(), roots: Vec::new(), runtime: "@bq/components/src/setups".into() }
    }

    pub fn source(mut self, dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        fs::create_dir_all(&self.out).expect("create the bindings folder");
        let import = relative(&self.out, &dir);
        self.roots.push(Root { dir, import });
        self
    }

    pub fn boutique(mut self) -> Self {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../components/src");
        self.roots.push(Root { dir, import: "@bq/components/src".into() });
        self
    }

    pub fn write(self) {
        for root in &self.roots {
            println!("cargo:rerun-if-changed={}", root.dir.display());
        }

        let (types, registry, warnings) = match self.generate() {
            Ok(output) => output,
            Err(errors) => panic!("\n{}\n", errors.join("\n")),
        };

        for warning in warnings {
            println!("cargo:warning={warning}");
        }

        fs::create_dir_all(&self.out).expect("create the bindings folder");
        write_if_changed(&self.out.join("setups.ts"), &types);
        write_if_changed(&self.out.join("registry.ts"), &registry);
    }

    fn generate(&self) -> Result<(String, String, Vec<String>), Vec<String>> {
        let mut sources = Vec::new();
        for (index, root) in self.roots.iter().enumerate() {
            let mut files = Vec::new();
            rust_files(&root.dir, &mut files);
            files.sort();

            for file in files {
                let text = fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
                let folder = file.parent().unwrap_or(&root.dir).strip_prefix(&root.dir).unwrap_or(Path::new(""));
                sources.push(Source {
                    root: index,
                    name: file.display().to_string(),
                    import: join_import(&root.import, folder),
                    text,
                });
            }
        }
        analyse(&self.runtime, &sources)
    }
}

fn analyse(runtime: &str, sources: &[Source]) -> Result<(String, String, Vec<String>), Vec<String>> {
    let mut index = Index::default();
    let mut declarations = Vec::new();
    let mut errors = Vec::new();

    for source in sources {
        let parsed = match syn::parse_file(&source.text) {
            Ok(parsed) => parsed,
            Err(error) => {
                errors.push(format!("{}: {error}", source.name));
                continue;
            }
        };
        index.add_file(source.name.clone(), source.root, &parsed);
        let mut visitor = Declarations { source, declarations: &mut declarations, errors: &mut errors };
        visitor.visit_file(&parsed);
    }

    let mut probe = Walker::new(&index);
    walk(&mut probe, &index, &HashSet::new());
    let mut walker = Walker::new(&index);
    walk(&mut walker, &index, &probe.followed);
    let (usages, claims, mut walk_errors, mut warnings) = walker.finish();
    errors.append(&mut walk_errors);

    let mut by_name: BTreeMap<String, Declaration> = BTreeMap::new();
    for declaration in declarations {
        if let Some(existing) = by_name.get(&declaration.name) {
            errors.push(format!(
                "{}: setup!({}) is declared two times, first at {}",
                declaration.location, declaration.name, existing.location
            ));
            continue;
        }
        by_name.insert(declaration.name.clone(), declaration);
    }

    let merged = merge(&usages, &by_name, &mut errors);
    collisions(&usages, &claims, &merged, &mut errors);

    for (name, declaration) in &by_name {
        if !merged.contains_key(name) {
            warnings.push(format!("{}: setup!({name}) is not used in any bq-setup", declaration.location));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok((types(runtime, &by_name, &merged), registry(runtime, &by_name, &merged), warnings))
}

fn walk(walker: &mut Walker, index: &Index, followed: &HashSet<usize>) {
    for (block, html) in index.blocks.iter().enumerate() {
        let skip = html.function.is_some_and(|function| index.functions[function].component || followed.contains(&function));
        if !skip {
            walker.block(block);
        }
    }
    for (function, item) in index.functions.iter().enumerate() {
        if item.component && !walker.reached.contains(&function) {
            for &block in &item.blocks {
                walker.block(block);
            }
        }
    }
}

fn types(runtime: &str, declarations: &BTreeMap<String, Declaration>, merged: &BTreeMap<String, Merged>) -> String {
    let mut out = format!("import type {{ Setup }} from \"{runtime}\";\n\n");

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
        let refs = if refs.is_empty() { "{}".to_string() } else { format!("{{ {} }}", refs.join("; ")) };

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
                        let tag = tag.as_ref().map(|tag| format!("\"{tag}\"")).unwrap_or_else(|| "string".into());
                        format!("{{ event: \"{event}\"; tag: {tag}{} }}", target.suffix())
                    })
                    .collect();
                format!("{handler}: {}", uses.join(" | "))
            })
            .collect();
        out.push_str(&format!("export type {name} = Setup<{tags}, {refs}, {{ {} }}>;\n", handlers.join("; ")));
    }

    out
}

fn registry(runtime: &str, declarations: &BTreeMap<String, Declaration>, merged: &BTreeMap<String, Merged>) -> String {
    let mut out = format!("import type {{ Registry }} from \"{runtime}\";\nimport type * as setups from \"./setups\";\n");

    for (name, declaration) in declarations {
        if declaration.eager {
            out.push_str(&format!("import {{ {} }} from \"{}\";\n", name.to_lower_camel_case(), declaration.import));
        }
    }

    out.push_str("\nexport const registry: Registry = {\n");
    for (name, declaration) in declarations {
        let empty = Merged::default();
        let setup = merged.get(name).unwrap_or(&empty);
        let export = name.to_lower_camel_case();
        let tags = if setup.any { String::new() } else { quoted_list(setup.tags.iter()) };
        let refs = quoted_list(setup.refs.keys());
        let many = quoted_list(setup.refs.iter().filter(|(_, reference)| reference.many).map(|(key, _)| key));
        let handlers = quoted_list(setup.handlers.keys());

        out.push_str(&format!("    \"{}\": {{\n", name.to_kebab_case()));
        out.push_str(&format!("        tags: [{tags}],\n"));
        out.push_str(&format!("        refs: [{refs}],\n"));
        out.push_str(&format!("        many: [{many}],\n"));
        out.push_str(&format!("        handlers: [{handlers}],\n"));
        if declaration.eager {
            out.push_str(&format!("        setup: {export} satisfies setups.{name},\n"));
        } else {
            out.push_str(&format!(
                "        load: () => import(\"{}\").then((module) => module.{export} satisfies setups.{name}),\n",
                declaration.import
            ));
        }
        out.push_str("    },\n");
    }
    out.push_str("};\n");

    out
}

struct Declarations<'a> {
    source: &'a Source,
    declarations: &'a mut Vec<Declaration>,
    errors: &'a mut Vec<String>,
}

impl<'ast> Visit<'ast> for Declarations<'_> {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.segments.last().is_some_and(|segment| segment.ident == "setup") {
            let line = mac.path.segments.last().map(|segment| segment.ident.span().start().line).unwrap_or(0);
            let location = format!("{}:{line}", self.source.name);
            match declaration(mac.tokens.clone()) {
                Ok((name, eager)) => {
                    self.declarations.push(Declaration { name, eager, import: self.source.import.clone(), location });
                }
                Err(message) => self.errors.push(format!("{location}: {message}")),
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn declaration(tokens: TokenStream) -> Result<(String, bool), String> {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let mut index = 0;

    if matches!(tokens.get(index), Some(TokenTree::Ident(ident)) if ident == "pub") {
        index += 1;
        if matches!(tokens.get(index), Some(TokenTree::Group(_))) {
            index += 1;
        }
    }

    let Some(TokenTree::Ident(name)) = tokens.get(index) else {
        return Err("setup!(..) needs a name, for example setup!(ThemeSelect)".into());
    };
    index += 1;

    let mut eager = false;
    if matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == ',') {
        match tokens.get(index + 1) {
            Some(TokenTree::Ident(option)) if option == "eager" => eager = true,
            _ => return Err("the only option for setup!(..) is `eager`".into()),
        }
        index += 2;
    }

    if index < tokens.len() {
        return Err("unexpected tokens in setup!(..)".into());
    }

    Ok((name.to_string(), eager))
}

fn merge(usages: &[Usage], declarations: &BTreeMap<String, Declaration>, errors: &mut Vec<String>) -> BTreeMap<String, Merged> {
    let mut merged: BTreeMap<String, Merged> = BTreeMap::new();

    for usage in usages {
        if !declarations.contains_key(&usage.name) {
            errors.push(format!("{}: bq-setup=({}) has no setup!({}) declaration", usage.location, usage.name, usage.name));
            continue;
        }

        let setup = merged.entry(usage.name.clone()).or_default();
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

fn collisions(usages: &[Usage], claims: &[Claim], merged: &BTreeMap<String, Merged>, errors: &mut Vec<String>) {
    let uses = |setup: &str, kind: Kind, name: &str| {
        merged.get(setup).is_some_and(|merged| match kind {
            Kind::Ref => merged.refs.contains_key(name),
            Kind::Handler => merged.handlers.contains_key(name),
        })
    };
    let mut seen = BTreeSet::new();
    for claim in claims {
        let owner = &usages[claim.owner].name;
        for &other in &claim.between {
            let other = &usages[other].name;
            if uses(other, claim.kind, &claim.name) && seen.insert((&claim.location, &claim.text, other)) {
                errors.push(format!(
                    "{}: {} is written for {owner}, but {other} also uses the name `{}` and is nearer, so {other} would get it. \
                     Rename one of them, or write the setup name, for example \"{}.{}\"",
                    claim.location,
                    claim.text,
                    claim.name.to_kebab_case(),
                    owner.to_kebab_case(),
                    claim.name.to_kebab_case()
                ));
            }
        }
    }
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
    tags.iter().map(|tag| format!("\"{tag}\"")).collect::<Vec<_>>().join(" | ")
}

fn quoted_list<'a>(items: impl Iterator<Item = &'a String>) -> String {
    items.map(|item| format!("\"{item}\"")).collect::<Vec<_>>().join(", ")
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

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
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

fn write_if_changed(path: &Path, content: &str) {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return;
    }
    fs::write(path, content).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}
