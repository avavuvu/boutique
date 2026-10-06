use std::collections::HashMap;

use bq_html::ast::{Element, Markups};
use heck::ToUpperCamelCase;
use syn::visit::Visit;

use crate::walk::{html_blocks, parse};

pub struct File {
    pub name: String,
    pub root: usize,
}

pub struct Function {
    pub file: usize,
    pub component: bool,
    pub blocks: Vec<usize>,
}

pub struct Block {
    pub file: usize,
    pub function: Option<usize>,
    pub markups: Markups<Element>,
}

#[derive(Default)]
pub struct Index {
    pub files: Vec<File>,
    pub functions: Vec<Function>,
    pub blocks: Vec<Block>,
    pub by_name: HashMap<String, Vec<usize>>,
    pub components: HashMap<String, Vec<usize>>,
}

impl Index {
    pub fn add_file(&mut self, name: String, root: usize, parsed: &syn::File) {
        let file = self.files.len();
        self.files.push(File { name, root });
        let mut collector = Collector { index: self, file, local: HashMap::new(), current: None };
        collector.visit_file(parsed);
    }

    fn function(&mut self, file: usize, local: &mut HashMap<String, usize>, item: &syn::ItemFn) -> usize {
        let name = item.sig.ident.to_string();
        let component = item.attrs.iter().any(|attr| attr.path().segments.last().is_some_and(|segment| segment.ident == "component"));

        let index = *local.entry(name.clone()).or_insert_with(|| {
            self.functions.push(Function { file, component: false, blocks: Vec::new() });
            let index = self.functions.len() - 1;
            self.by_name.entry(name.clone()).or_default().push(index);
            index
        });

        if component && !self.functions[index].component {
            self.functions[index].component = true;
            let alias = if name.starts_with(|first: char| first.is_ascii_uppercase()) { name } else { name.to_upper_camel_case() };
            self.components.entry(alias).or_default().push(index);
        }
        index
    }

    fn block(&mut self, file: usize, function: Option<usize>, tokens: proc_macro2::TokenStream) {
        let Some(markups) = parse(tokens) else { return };
        self.blocks.push(Block { file, function, markups });
        let block = self.blocks.len() - 1;
        if let Some(function) = function {
            self.functions[function].blocks.push(block);
        }
    }
}

struct Collector<'i> {
    index: &'i mut Index,
    file: usize,
    local: HashMap<String, usize>,
    current: Option<usize>,
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

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.segments.last().is_some_and(|segment| segment.ident == "html") {
            self.index.block(self.file, self.current, mac.tokens.clone());
        } else {
            let mut blocks = Vec::new();
            html_blocks(mac.tokens.clone(), &mut blocks);
            for tokens in blocks {
                self.index.block(self.file, self.current, tokens);
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}
