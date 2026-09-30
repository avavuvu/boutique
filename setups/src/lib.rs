mod markup;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Component, Path, PathBuf},
};

use heck::{ToKebabCase, ToLowerCamelCase};
use proc_macro2::{TokenStream, TokenTree};
use syn::visit::Visit;

use markup::{Usage, Walker};

const RESERVED: [&str; 2] = ["signal", "props"];

struct Root {
    dir: PathBuf,
    import: String,
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
    refs: BTreeMap<String, MergedRef>,
    usages: usize,
}

#[derive(Default)]
struct MergedRef {
    tags: BTreeSet<String>,
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
        let mut walker = Walker::default();
        let mut declarations = Vec::new();
        let mut errors = Vec::new();

        for root in &self.roots {
            let mut files = Vec::new();
            rust_files(&root.dir, &mut files);
            files.sort();

            for file in files {
                let source = fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
                let parsed = match syn::parse_file(&source) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        errors.push(format!("{}: {error}", file.display()));
                        continue;
                    }
                };

                let folder = file.parent().unwrap_or(&root.dir).strip_prefix(&root.dir).unwrap_or(Path::new(""));
                let import = join_import(&root.import, folder);

                let mut collector = Collector::default();
                collector.visit_file(&parsed);

                walker.start_file(file.display().to_string(), collector.functions);
                let mut visitor = Visitor { walker: &mut walker, declarations: &mut declarations, errors: &mut errors, import };
                visitor.visit_file(&parsed);
                walker.finish_file();
            }
        }

        errors.append(&mut walker.errors);

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

        let merged = merge(&walker.usages, &by_name, &mut errors);

        let mut warnings = std::mem::take(&mut walker.warnings);
        for (name, declaration) in &by_name {
            if !merged.contains_key(name) {
                warnings.push(format!("{}: setup!({name}) is not used in any bq-setup", declaration.location));
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok((self.types(&by_name, &merged), self.registry(&by_name, &merged), warnings))
    }

    fn types(&self, declarations: &BTreeMap<String, Declaration>, merged: &BTreeMap<String, Merged>) -> String {
        let mut out = format!("import type {{ Setup }} from \"{}\";\n\n", self.runtime);

        for name in declarations.keys() {
            let empty = Merged::default();
            let setup = merged.get(name).unwrap_or(&empty);
            let tags = tag_union(&setup.tags);

            if setup.refs.is_empty() {
                out.push_str(&format!("export type {name} = Setup<{tags}>;\n"));
                continue;
            }

            let refs: Vec<String> = setup
                .refs
                .iter()
                .map(|(key, reference)| {
                    let tags = tag_union(&reference.tags);
                    if reference.many {
                        format!("{key}: ({tags})[]")
                    } else if reference.optional {
                        format!("{key}?: {tags}")
                    } else {
                        format!("{key}: {tags}")
                    }
                })
                .collect();
            out.push_str(&format!("export type {name} = Setup<{tags}, {{ {} }}>;\n", refs.join("; ")));
        }

        out
    }

    fn registry(&self, declarations: &BTreeMap<String, Declaration>, merged: &BTreeMap<String, Merged>) -> String {
        let mut out = format!("import type {{ Registry }} from \"{}\";\nimport type * as setups from \"./setups\";\n", self.runtime);

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
            let tags = quoted_list(setup.tags.iter());
            let many = quoted_list(setup.refs.iter().filter(|(_, reference)| reference.many).map(|(key, _)| key));

            out.push_str(&format!("    \"{}\": {{\n", name.to_kebab_case()));
            out.push_str(&format!("        tags: [{tags}],\n"));
            out.push_str(&format!("        many: [{many}],\n"));
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
}

#[derive(Default)]
struct Collector {
    functions: HashMap<String, Vec<TokenStream>>,
    current: Option<String>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let previous = self.current.replace(item.sig.ident.to_string());
        syn::visit::visit_item_fn(self, item);
        self.current = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let previous = self.current.take();
        syn::visit::visit_impl_item_fn(self, item);
        self.current = previous;
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(function) = &self.current {
            let blocks = self.functions.entry(function.clone()).or_default();
            if mac.path.segments.last().is_some_and(|segment| segment.ident == "html") {
                blocks.push(mac.tokens.clone());
            } else {
                html_blocks(mac.tokens.clone(), blocks);
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn html_blocks(tokens: TokenStream, out: &mut Vec<TokenStream>) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        let TokenTree::Group(group) = token else { continue };
        let is_html = index >= 2
            && matches!(&tokens[index - 2], TokenTree::Ident(ident) if ident == "html")
            && matches!(&tokens[index - 1], TokenTree::Punct(punct) if punct.as_char() == '!');
        if is_html {
            out.push(group.stream());
        } else {
            html_blocks(group.stream(), out);
        }
    }
}

struct Visitor<'a> {
    walker: &'a mut Walker,
    declarations: &'a mut Vec<Declaration>,
    errors: &'a mut Vec<String>,
    import: String,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let previous = self.walker.function.replace(item.sig.ident.to_string());
        syn::visit::visit_item_fn(self, item);
        self.walker.function = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let previous = self.walker.function.take();
        syn::visit::visit_impl_item_fn(self, item);
        self.walker.function = previous;
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = mac.path.segments.last().map(|segment| segment.ident.to_string());
        match name.as_deref() {
            Some("html") => self.walker.html(mac.tokens.clone()),
            Some("setup") => {
                let line = mac.path.segments.last().map(|segment| segment.ident.span().start().line).unwrap_or(0);
                let location = format!("{}:{line}", self.walker.file);
                match declaration(mac.tokens.clone()) {
                    Ok((name, eager)) => {
                        self.declarations.push(Declaration { name, eager, import: self.import.clone(), location });
                    }
                    Err(message) => self.errors.push(format!("{location}: {message}")),
                }
            }
            _ => self.walker.nested(mac.tokens.clone()),
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
        setup.tags.insert(usage.tag.clone());

        let mut seen = BTreeSet::new();
        for reference in &usage.refs {
            if RESERVED.contains(&reference.name.as_str()) {
                errors.push(format!("{}: bq-ref=\"{}\" is a reserved name", reference.location, reference.name));
                continue;
            }

            let first = seen.insert(reference.name.clone());
            let entry = setup.refs.entry(reference.name.clone()).or_default();
            entry.tags.insert(reference.tag.clone());
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

fn tag_union(tags: &BTreeSet<String>) -> String {
    if tags.is_empty() {
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
