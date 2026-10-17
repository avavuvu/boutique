use std::{collections::HashMap, sync::Arc};

use bq_html::{
    conventions::{ComponentOptions, component_alias},
    setup::has_bare_setup,
};
use syn::{ext::IdentExt, visit::Visit};

use super::{Block, Function, Index, Module};
use crate::{
    diagnostics::{Loc, Sink},
    html::{Found, HtmlFinder, parse},
    model::Declaration,
    sources::Source,
};

pub fn file(index: &mut Index, name: Arc<str>, source: &Source, parsed: &syn::File, sink: &mut Sink) {
    let file = index.files.len();
    let module = index.module(source.root, source.module.clone(), None);
    index.files.push(super::File {
        name,
        root: source.root,
        import: source.import.clone(),
        module,
    });
    let mut collector = Collector {
        index,
        sink,
        file,
        module,
        local: HashMap::new(),
    };
    collector.visit_file(parsed);
}

impl Index {
    pub(super) fn module(&mut self, root: usize, path: Vec<String>, scope: Option<usize>) -> usize {
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
            let function = self.functions.len();
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
            self.callers.push(Default::default());
            self.callees.push(Default::default());
            self.named.entry(name.clone()).or_default().push(function);
            if !method {
                self.by_name.entry(name).or_default().push(function);
            }
            function
        })
    }

    fn component(&mut self, file: usize, function: usize, item: &syn::ItemFn, attribute: &syn::Attribute, sink: &mut Sink) {
        if !self.functions[function].component {
            self.functions[function].component = true;
            let alias = self.functions[function].alias.clone();
            self.components.entry(alias).or_default().push(function);
        }

        let loc = Loc::new(&self.files[file].name, item.sig.ident.span());
        let options = match &attribute.meta {
            syn::Meta::Path(_) => Ok(ComponentOptions::default()),
            syn::Meta::List(list) => syn::parse2::<ComponentOptions>(list.tokens.clone()),
            syn::Meta::NameValue(pair) => Err(syn::Error::new_spanned(pair, "expected #[component] or #[component(..)]")),
        };
        let options = options.unwrap_or_else(|error| {
            sink.error(&loc, error.to_string(), None);
            ComponentOptions::default()
        });

        if !has_bare_setup(&item.block) {
            return;
        }
        let name = options.name(&item.sig.ident);
        self.functions[function].setup.get_or_insert_with(|| name.clone());
        self.declarations.push(Declaration {
            name,
            function: item.sig.ident.unraw().to_string(),
            function_id: function,
            eager: options.eager.is_some(),
            import: self.files[file].import.clone(),
            loc,
        });
    }

    fn block(&mut self, file: usize, function: Option<usize>, found: Found, sink: &mut Sink) {
        let binding = found.binding.clone().filter(|_| function.is_some());
        let flags = found.flags;
        let Some(markups) = parse(found, &self.files[file].name, sink) else {
            if let Some(function) = function {
                self.functions[function].unparsed = true;
            }
            return;
        };
        let block = self.blocks.len();
        self.blocks.push(Block {
            file,
            function,
            flags,
            binding: binding.clone(),
            spliced: false,
            region: 0,
            markups: Some(markups),
        });
        match function {
            Some(function) => {
                self.functions[function].blocks.push(block);
                if let Some(binding) = binding {
                    self.functions[function].bindings.insert(binding, block);
                }
            }
            None => self.loose.push(block),
        }
    }
}

struct Collector<'i> {
    index: &'i mut Index,
    sink: &'i mut Sink,
    file: usize,
    module: usize,
    local: HashMap<(usize, String), usize>,
}

impl Collector<'_> {
    fn blocks(&mut self, function: Option<usize>, found: Vec<Found>) {
        for found in found {
            self.index.block(self.file, function, found, self.sink);
        }
    }

    fn inside(&mut self, module: usize, visit: impl FnOnce(&mut Self)) {
        let previous = std::mem::replace(&mut self.module, module);
        visit(self);
        self.module = previous;
    }

    fn child(&mut self, name: String, scope: Option<usize>) -> usize {
        let parent = &self.index.modules[scope.unwrap_or(self.module)];
        let (root, mut path) = (parent.root, parent.path.clone());
        path.push(name);
        self.index.module(root, path, scope)
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
        if item.content.is_some() {
            let module = self.child(item.ident.to_string(), None);
            self.inside(module, |collector| syn::visit::visit_item_mod(collector, item));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let name = match item.self_ty.as_ref() {
            syn::Type::Path(ty) => ty.path.segments.last().map(|segment| segment.ident.to_string()),
            _ => None,
        };
        let Some(name) = name else {
            return syn::visit::visit_item_impl(self, item);
        };
        let scope = self.index.modules[self.module].scope;
        let module = self.child(name, Some(scope));
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
            self.index.component(self.file, function, item, attribute, self.sink);
        }
        self.blocks(Some(function), HtmlFinder::block(&item.block));
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let function = self.index.function(self.file, self.module, &mut self.local, &item.sig);
        self.blocks(Some(function), HtmlFinder::block(&item.block));
        syn::visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(block) = &item.default {
            self.blocks(None, HtmlFinder::block(block));
        }
        syn::visit::visit_trait_item_fn(self, item);
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.blocks(None, HtmlFinder::expr(&item.expr));
        syn::visit::visit_item_const(self, item);
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.blocks(None, HtmlFinder::expr(&item.expr));
        syn::visit::visit_item_static(self, item);
    }

    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        self.blocks(None, HtmlFinder::expr(&item.expr));
        syn::visit::visit_impl_item_const(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        self.blocks(None, HtmlFinder::mac(&item.mac));
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
