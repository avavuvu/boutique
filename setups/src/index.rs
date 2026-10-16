use std::collections::{HashMap, HashSet};

use bq_html::{
    ast::{Element, Markups},
    conventions::{ComponentOptions, component_alias, is_html_macro},
    setup::has_bare_setup,
};
use syn::{ext::IdentExt, spanned::Spanned, visit::Visit};

use crate::walk::{Flags, html_blocks, parse, unparsed};

pub struct File {
    pub name: String,
    pub root: usize,
    pub import: String,
    pub module: usize,
}

pub struct Module {
    pub root: usize,
    pub path: Vec<String>,
    pub scope: usize,
    pub uses: HashMap<String, Vec<String>>,
    pub globs: Vec<Vec<String>>,
}

pub struct Function {
    pub name: String,
    pub file: usize,
    pub module: usize,
    pub component: bool,
    pub alias: String,
    pub setup: Option<String>,
    pub blocks: Vec<usize>,
    pub bindings: HashMap<String, usize>,
    pub unparsed: bool,
}

pub struct SetupDeclaration {
    pub name: String,
    pub function: String,
    pub function_id: usize,
    pub eager: bool,
    pub file: usize,
    pub line: usize,
}

pub struct Block {
    pub file: usize,
    pub function: Option<usize>,
    pub markups: Markups<Element>,
    pub binding: Option<String>,
    pub optional: bool,
    pub many: bool,
}

#[derive(Default)]
pub struct Index {
    pub files: Vec<File>,
    pub modules: Vec<Module>,
    pub functions: Vec<Function>,
    pub blocks: Vec<Block>,
    pub by_name: HashMap<String, Vec<usize>>,
    pub components: HashMap<String, Vec<usize>>,
    named: HashMap<String, Vec<usize>>,
    pub setups: Vec<SetupDeclaration>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub embedded: Option<usize>,
    places: HashMap<(usize, Vec<String>), Vec<usize>>,
    known: HashSet<(usize, Vec<String>)>,
}

pub enum Resolved {
    Found(usize),
    Ambiguous(Vec<usize>),
    Missing,
}

enum Located {
    Ours(usize, Vec<String>),
    Extern,
}

const ALIAS_DEPTH: usize = 8;

impl Index {
    pub fn new(embedded: Option<usize>) -> Self {
        Self {
            embedded,
            ..Self::default()
        }
    }

    pub fn add_file(&mut self, name: String, root: usize, path: Vec<String>, import: String, parsed: &syn::File) {
        let file = self.files.len();
        let module = self.module(root, path, None);
        self.files.push(File {
            name,
            root,
            import,
            module,
        });
        let mut collector = Collector {
            index: self,
            file,
            module,
            local: HashMap::new(),
            current: None,
            flags: Flags::default(),
            binding: None,
        };
        collector.visit_file(parsed);
    }

    fn module(&mut self, root: usize, path: Vec<String>, scope: Option<usize>) -> usize {
        let module = self.modules.len();
        for length in 0..=path.len() {
            self.known.insert((root, path[..length].to_vec()));
        }
        self.places.entry((root, path.clone())).or_default().push(module);
        self.modules.push(Module {
            root,
            path,
            scope: scope.unwrap_or(module),
            uses: HashMap::new(),
            globs: Vec::new(),
        });
        module
    }

    fn function(&mut self, file: usize, module: usize, local: &mut HashMap<(usize, String), usize>, sig: &syn::Signature) -> usize {
        let name = sig.ident.to_string();
        let method = self.modules[module].scope != module;
        *local.entry((module, name.clone())).or_insert_with(|| {
            self.functions.push(Function {
                name: name.clone(),
                file,
                module,
                component: false,
                alias: component_alias(&sig.ident),
                setup: None,
                blocks: Vec::new(),
                bindings: HashMap::new(),
                unparsed: false,
            });
            let index = self.functions.len() - 1;
            self.named.entry(name.clone()).or_default().push(index);
            if !method {
                self.by_name.entry(name).or_default().push(index);
            }
            index
        })
    }

    fn component(&mut self, file: usize, function: usize, item: &syn::ItemFn, attribute: &syn::Attribute) {
        if !self.functions[function].component {
            self.functions[function].component = true;
            let alias = self.functions[function].alias.clone();
            self.components.entry(alias).or_default().push(function);
        }

        let line = item.sig.ident.span().start().line;
        let options = match &attribute.meta {
            syn::Meta::Path(_) => Ok(ComponentOptions::default()),
            syn::Meta::List(list) => syn::parse2::<ComponentOptions>(list.tokens.clone()),
            syn::Meta::NameValue(pair) => Err(syn::Error::new_spanned(pair, "expected #[component] or #[component(..)]")),
        };
        let options = options.unwrap_or_else(|error| {
            self.errors.push(format!("{}:{line}: {error}", self.files[file].name));
            ComponentOptions::default()
        });

        if !has_bare_setup(&item.block) {
            return;
        }
        let name = options.name(&item.sig.ident);
        self.functions[function].setup.get_or_insert_with(|| name.clone());
        self.setups.push(SetupDeclaration {
            name,
            function: item.sig.ident.unraw().to_string(),
            function_id: function,
            eager: options.eager.is_some(),
            file,
            line,
        });
    }

    fn block(
        &mut self,
        file: usize,
        function: Option<usize>,
        span: proc_macro2::Span,
        tokens: proc_macro2::TokenStream,
        flags: Flags,
        binding: Option<String>,
    ) {
        let markups = match parse(tokens) {
            Ok(markups) => markups,
            Err(error) => {
                let location = format!("{}:{}", self.files[file].name, span.start().line);
                self.warnings.push(unparsed(&location, &error));
                if let Some(function) = function {
                    self.functions[function].unparsed = true;
                }
                return;
            }
        };
        let binding = binding.filter(|_| function.is_some());
        self.blocks.push(Block {
            file,
            function,
            markups,
            binding: binding.clone(),
            optional: flags.optional,
            many: flags.many,
        });
        let block = self.blocks.len() - 1;
        if let Some(function) = function {
            self.functions[function].blocks.push(block);
            if let Some(binding) = binding {
                self.functions[function].bindings.insert(binding, block);
            }
        }
    }

    pub fn module_of(&self, file: usize, function: Option<usize>) -> usize {
        function.map_or(self.files[file].module, |function| self.functions[function].module)
    }

    pub fn resolve(&self, module: usize, file: usize, path: &[String], component: bool) -> Resolved {
        let Some((name, qualifier)) = path.split_last() else {
            return Resolved::Missing;
        };
        if !qualifier.is_empty() {
            return match self.locate(module, qualifier, 0) {
                Some(Located::Ours(root, place)) => self.item(root, &place, name, component, file, 0),
                _ => Resolved::Missing,
            };
        }

        let scope = self.modules[module].scope;
        if let Some(alias) = self.modules[scope].uses.get(name) {
            match self.alias(scope, alias, component, file, 0) {
                None => return Resolved::Missing,
                Some(Resolved::Missing) => {}
                Some(resolved) => return resolved,
            }
        }
        if let Some(&function) = self
            .table(component)
            .get(name)
            .and_then(|candidates| candidates.iter().find(|&&candidate| self.functions[candidate].module == scope))
        {
            return Resolved::Found(function);
        }
        for glob in &self.modules[scope].globs {
            if let Some(Located::Ours(root, place)) = self.locate(scope, glob, 0) {
                let resolved = self.item(root, &place, name, component, file, 0);
                if !matches!(resolved, Resolved::Missing) {
                    return resolved;
                }
            }
        }
        self.by_bare_name(name, file, component)
    }

    pub fn method(&self, module: usize, file: usize, name: &str) -> Resolved {
        let here = &self.modules[module];
        if here.scope == module {
            return Resolved::Missing;
        }
        self.item(here.root, &here.path, name, false, file, 0)
    }

    fn table(&self, component: bool) -> &HashMap<String, Vec<usize>> {
        if component { &self.components } else { &self.by_name }
    }

    fn by_bare_name(&self, name: &str, file: usize, component: bool) -> Resolved {
        let Some(candidates) = self.table(component).get(name) else {
            return Resolved::Missing;
        };
        if let Some(&first) = candidates.iter().find(|&&candidate| self.functions[candidate].file == file) {
            return Resolved::Found(first);
        }
        let root = self.files[file].root;
        let in_root: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&candidate| self.files[self.functions[candidate].file].root == root)
            .collect();
        match (in_root.as_slice(), candidates.as_slice()) {
            ([one], _) | ([], [one]) => Resolved::Found(*one),
            _ => Resolved::Ambiguous(candidates.clone()),
        }
    }

    fn alias(&self, scope: usize, alias: &[String], component: bool, file: usize, depth: usize) -> Option<Resolved> {
        let Some((name, qualifier)) = alias.split_last() else {
            return Some(Resolved::Missing);
        };
        if qualifier.is_empty() {
            return None;
        }
        match self.locate(scope, qualifier, depth + 1) {
            Some(Located::Ours(root, place)) => Some(self.item(root, &place, name, component, file, depth + 1)),
            Some(Located::Extern) => None,
            None => Some(Resolved::Missing),
        }
    }

    fn locate(&self, module: usize, segments: &[String], depth: usize) -> Option<Located> {
        if depth > ALIAS_DEPTH {
            return None;
        }
        let scope = self.modules[module].scope;
        let here = &self.modules[scope];
        let Some((first, rest)) = segments.split_first() else {
            return Some(Located::Ours(here.root, here.path.clone()));
        };

        let (root, mut place, rest) = match first.as_str() {
            "crate" => (here.root, Vec::new(), rest),
            "self" => (here.root, here.path.clone(), rest),
            "super" => {
                let supers = segments.iter().take_while(|segment| *segment == "super").count();
                let kept = here.path.len().checked_sub(supers)?;
                (here.root, here.path[..kept].to_vec(), &segments[supers..])
            }
            "Self" => (self.modules[module].root, self.modules[module].path.clone(), rest),
            "bq_components" => (self.embedded?, Vec::new(), rest),
            "boutique" if rest.first().is_some_and(|segment| segment == "components") => (self.embedded?, Vec::new(), &rest[1..]),
            _ => {
                if let Some(alias) = here.uses.get(first).filter(|alias| alias.as_slice() != [first.clone()]) {
                    let mut expanded = alias.clone();
                    expanded.extend(rest.iter().cloned());
                    return self.locate(scope, &expanded, depth + 1);
                }
                let mut child = here.path.clone();
                child.push(first.clone());
                if self.known.contains(&(here.root, child.clone())) {
                    (here.root, child, rest)
                } else if let Some(place) = self.through_globs(scope, first, depth) {
                    (here.root, place, rest)
                } else {
                    return Some(Located::Extern);
                }
            }
        };

        for segment in rest {
            place = self.step(root, place, segment, depth)?;
        }
        Some(Located::Ours(root, place))
    }

    fn through_globs(&self, scope: usize, name: &str, depth: usize) -> Option<Vec<String>> {
        self.modules[scope].globs.iter().find_map(|glob| {
            let mut path = glob.clone();
            path.push(name.to_string());
            match self.locate(scope, &path, depth + 1)? {
                Located::Ours(root, place) if root == self.modules[scope].root && self.known.contains(&(root, place.clone())) => {
                    Some(place)
                }
                _ => None,
            }
        })
    }

    fn step(&self, root: usize, mut place: Vec<String>, segment: &str, depth: usize) -> Option<Vec<String>> {
        let modules = self.places.get(&(root, place.clone())).cloned().unwrap_or_default();
        place.push(segment.to_string());
        if self.known.contains(&(root, place.clone())) {
            return Some(place);
        }
        for module in modules {
            if let Some(alias) = self.modules[module].uses.get(segment)
                && let Some(Located::Ours(alias_root, target)) = self.locate(module, alias, depth + 1)
                && alias_root == root
            {
                return Some(target);
            }
        }
        None
    }

    fn item(&self, root: usize, place: &[String], name: &str, component: bool, file: usize, depth: usize) -> Resolved {
        if depth > ALIAS_DEPTH {
            return Resolved::Missing;
        }
        let modules = self.places.get(&(root, place.to_vec())).cloned().unwrap_or_default();
        let candidates = if component {
            self.components.get(name)
        } else {
            self.named.get(name)
        };
        let candidates = candidates.map(Vec::as_slice).unwrap_or_default();

        let defined: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&function| modules.contains(&self.functions[function].module))
            .collect();
        if !defined.is_empty() {
            return self.pick(defined, file);
        }

        for &module in &modules {
            if let Some(alias) = self.modules[module].uses.get(name)
                && let Some(resolved) = self.alias(module, alias, component, file, depth)
                && !matches!(resolved, Resolved::Missing)
            {
                return resolved;
            }
        }
        for &module in &modules {
            for glob in &self.modules[module].globs {
                if let Some(Located::Ours(glob_root, target)) = self.locate(module, glob, depth + 1) {
                    let resolved = self.item(glob_root, &target, name, component, file, depth + 1);
                    if !matches!(resolved, Resolved::Missing) {
                        return resolved;
                    }
                }
            }
        }

        let nested: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&function| {
                let module = &self.modules[self.functions[function].module];
                module.root == root && module.scope == self.functions[function].module && module.path.starts_with(place)
            })
            .collect();
        if nested.is_empty() {
            Resolved::Missing
        } else {
            self.pick(nested, file)
        }
    }

    fn pick(&self, candidates: Vec<usize>, file: usize) -> Resolved {
        match candidates.as_slice() {
            [one] => Resolved::Found(*one),
            _ => match candidates.iter().find(|&&candidate| self.functions[candidate].file == file) {
                Some(&first) => Resolved::Found(first),
                None => Resolved::Ambiguous(candidates),
            },
        }
    }
}

struct Collector<'i> {
    index: &'i mut Index,
    file: usize,
    module: usize,
    local: HashMap<(usize, String), usize>,
    current: Option<usize>,
    flags: Flags,
    binding: Option<String>,
}

impl Collector<'_> {
    fn inside(&mut self, module: usize, visit: impl FnOnce(&mut Self)) {
        let previous = std::mem::replace(&mut self.module, module);
        visit(self);
        self.module = previous;
    }

    fn inside_function(&mut self, function: usize, visit: impl FnOnce(&mut Self)) {
        let previous = self.current.replace(function);
        visit(self);
        self.current = previous;
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !test_only(item_attrs(item)) {
            syn::visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let attrs = match item {
            syn::ImplItem::Fn(item) => &item.attrs,
            syn::ImplItem::Const(item) => &item.attrs,
            syn::ImplItem::Type(item) => &item.attrs,
            syn::ImplItem::Macro(item) => &item.attrs,
            _ => return syn::visit::visit_impl_item(self, item),
        };
        if !test_only(attrs) {
            syn::visit::visit_impl_item(self, item);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if item.content.is_none() {
            return;
        }
        let parent = &self.index.modules[self.module];
        let (root, mut path) = (parent.root, parent.path.clone());
        path.push(item.ident.to_string());
        let module = self.index.module(root, path, None);
        self.inside(module, |collector| syn::visit::visit_item_mod(collector, item));
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let syn::Type::Path(ty) = item.self_ty.as_ref() else {
            return syn::visit::visit_item_impl(self, item);
        };
        let Some(name) = ty.path.segments.last().map(|segment| segment.ident.to_string()) else {
            return;
        };
        let scope = self.index.modules[self.module].scope;
        let parent = &self.index.modules[scope];
        let (root, mut path) = (parent.root, parent.path.clone());
        path.push(name);
        let module = self.index.module(root, path, Some(scope));
        self.inside(module, |collector| syn::visit::visit_item_impl(collector, item));
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let scope = self.index.modules[self.module].scope;
        let prefix = if item.leading_colon.is_some() {
            vec![String::from("::")]
        } else {
            Vec::new()
        };
        uses(&item.tree, prefix, &mut self.index.modules[scope]);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let function = self.index.function(self.file, self.module, &mut self.local, &item.sig);
        if let Some(attribute) = item
            .attrs
            .iter()
            .find(|attr| attr.path().segments.last().is_some_and(|segment| segment.ident == "component"))
        {
            self.index.component(self.file, function, item, attribute);
        }
        self.inside_function(function, |collector| syn::visit::visit_item_fn(collector, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let function = self.index.function(self.file, self.module, &mut self.local, &item.sig);
        self.inside_function(function, |collector| syn::visit::visit_impl_item_fn(collector, item));
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let name = match (binding_name(&local.pat), &local.init) {
            (Some(name), Some(init)) if is_html(&init.expr) => Some(name),
            _ => None,
        };
        let previous = std::mem::replace(&mut self.binding, name);
        syn::visit::visit_local(self, local);
        self.binding = previous;
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        let saved = self.flags;
        self.flags.enter(expr);
        syn::visit::visit_expr(self, expr);
        self.flags = saved;
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let binding = self.binding.take();
        if is_html_macro(&mac.path) {
            self.index
                .block(self.file, self.current, mac.path.span(), mac.tokens.clone(), self.flags, binding);
        } else {
            let mut blocks = Vec::new();
            html_blocks(mac.tokens.clone(), &mut blocks);
            for (span, tokens) in blocks {
                self.index.block(self.file, self.current, span, tokens, self.flags, None);
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn uses(tree: &syn::UseTree, mut prefix: Vec<String>, module: &mut Module) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            uses(&path.tree, prefix, module);
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(last) = prefix.last().cloned() {
                module.uses.insert(last, prefix);
            }
        }
        syn::UseTree::Name(name) => {
            let alias = name.ident.to_string();
            prefix.push(alias.clone());
            module.uses.insert(alias, prefix);
        }
        syn::UseTree::Rename(rename) => {
            if rename.ident != "self" {
                prefix.push(rename.ident.to_string());
            }
            if rename.rename != "_" {
                module.uses.insert(rename.rename.to_string(), prefix);
            }
        }
        syn::UseTree::Glob(_) => module.globs.push(prefix),
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                uses(tree, prefix.clone(), module);
            }
        }
    }
}

fn binding_name(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(pat) => Some(pat.ident.to_string()),
        syn::Pat::Type(pat) => binding_name(&pat.pat),
        _ => None,
    }
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| attr.parse_args::<syn::Meta>().is_ok_and(|meta| requires_test(&meta)))
}

fn requires_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) if list.path.is_ident("all") => list
            .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            .is_ok_and(|metas| metas.iter().any(requires_test)),
        _ => false,
    }
}

fn is_html(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Macro(mac) if is_html_macro(&mac.mac.path))
}
