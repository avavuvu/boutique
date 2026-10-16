use std::collections::HashMap;

use bq_html::{
    ast::{Element, Markups},
    setup::{ComponentOptions, mark},
};
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span};
use syn::{ext::IdentExt, visit::Visit};

use crate::walk::{Flags, html_blocks, parse};

pub struct File {
    pub name: String,
    pub root: usize,
    pub import: String,
}

pub struct Function {
    pub name: String,
    pub file: usize,
    pub component: bool,
    pub setup: Option<String>,
    pub blocks: Vec<usize>,
    pub bindings: HashMap<String, usize>,
}

pub struct SetupDeclaration {
    pub name: String,
    pub function: String,
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
    pub functions: Vec<Function>,
    pub blocks: Vec<Block>,
    pub by_name: HashMap<String, Vec<usize>>,
    pub components: HashMap<String, Vec<usize>>,
    pub setups: Vec<SetupDeclaration>,
    pub errors: Vec<String>,
}

impl Index {
    pub fn add_file(&mut self, name: String, root: usize, import: String, parsed: &syn::File) {
        let file = self.files.len();
        self.files.push(File { name, root, import });
        let mut collector = Collector {
            index: self,
            file,
            local: HashMap::new(),
            current: None,
            flags: Flags::default(),
            binding: None,
        };
        collector.visit_file(parsed);
    }

    fn function(&mut self, file: usize, local: &mut HashMap<String, usize>, item: &syn::ItemFn) -> usize {
        let name = item.sig.ident.to_string();
        let attribute = item
            .attrs
            .iter()
            .find(|attr| attr.path().segments.last().is_some_and(|segment| segment.ident == "component"));
        let component = attribute.is_some();

        let index = *local.entry(name.clone()).or_insert_with(|| {
            self.functions.push(Function {
                name: name.clone(),
                file,
                component: false,
                setup: None,
                blocks: Vec::new(),
                bindings: HashMap::new(),
            });
            let index = self.functions.len() - 1;
            self.by_name.entry(name.clone()).or_default().push(index);
            index
        });

        if component && !self.functions[index].component {
            self.functions[index].component = true;
            let alias = if name.starts_with(|first: char| first.is_ascii_uppercase()) {
                name
            } else {
                name.to_upper_camel_case()
            };
            self.components.entry(alias).or_default().push(index);
        }
        if let Some(attribute) = attribute {
            self.setup(file, index, item, attribute);
        }
        index
    }

    fn setup(&mut self, file: usize, function: usize, item: &syn::ItemFn, attribute: &syn::Attribute) {
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

        let mut block = (*item.block).clone();
        if !mark(&mut block, &Ident::new("__BqSetup", Span::call_site())) {
            return;
        }
        let name = options.name(&item.sig.ident);
        self.functions[function].setup.get_or_insert_with(|| name.clone());
        self.setups.push(SetupDeclaration {
            name,
            function: item.sig.ident.unraw().to_string(),
            eager: options.eager.is_some(),
            file,
            line,
        });
    }

    fn block(&mut self, file: usize, function: Option<usize>, tokens: proc_macro2::TokenStream, flags: Flags, binding: Option<String>) {
        let Some(markups) = parse(tokens) else { return };
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
}

struct Collector<'i> {
    index: &'i mut Index,
    file: usize,
    local: HashMap<String, usize>,
    current: Option<usize>,
    flags: Flags,
    binding: Option<String>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let function = self.index.function(self.file, &mut self.local, item);
        let previous = self.current.replace(function);
        syn::visit::visit_item_fn(self, item);
        self.current = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let previous = self.current.take();
        syn::visit::visit_impl_item_fn(self, item);
        self.current = previous;
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let name = match (&local.pat, &local.init) {
            (syn::Pat::Ident(pat), Some(init)) if is_html(&init.expr) => Some(pat.ident.to_string()),
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
        if mac.path.segments.last().is_some_and(|segment| segment.ident == "html") {
            self.index.block(self.file, self.current, mac.tokens.clone(), self.flags, binding);
        } else {
            let mut blocks = Vec::new();
            html_blocks(mac.tokens.clone(), &mut blocks);
            for tokens in blocks {
                self.index.block(self.file, self.current, tokens, self.flags, None);
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn is_html(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Macro(mac) if mac.mac.path.segments.last().is_some_and(|segment| segment.ident == "html"))
}
