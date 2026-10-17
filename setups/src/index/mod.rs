mod collect;
mod graph;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Arc,
};

use bq_html::ast::{Element as HtmlElement, Markups};

use crate::{
    attrs::ElementAttrs,
    diagnostics::{Loc, Sink},
    html::Flags,
    model::{Declaration, TagSet},
    sources::Source,
};

pub use graph::Callee;

pub struct File {
    pub name: Arc<str>,
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

pub struct Block {
    pub file: usize,
    pub function: Option<usize>,
    pub flags: Flags,
    pub binding: Option<String>,
    pub spliced: bool,
    pub region: usize,
    markups: Option<Markups<HtmlElement>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    Element(Element),
    Splice(Splice),
    Branch { flags: Flags, regions: Vec<usize> },
    Expr(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Element {
    pub tags: TagSet,
    pub attrs: usize,
    pub spread: bool,
    pub call: Option<Call>,
    pub body: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Call {
    pub callee: Callee,
    pub loc: Loc,
    pub positional: Vec<usize>,
    pub named: Vec<(String, usize)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Splice {
    pub children: bool,
    pub name: Option<String>,
    pub callee: Option<(Callee, Loc)>,
    pub expr: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Expr {
    pub binding: Option<usize>,
    pub nested: Vec<(Flags, usize)>,
}

pub const NO_ATTRS: usize = 0;

pub struct Index {
    pub files: Vec<File>,
    pub modules: Vec<Module>,
    pub functions: Vec<Function>,
    pub blocks: Vec<Block>,
    pub declarations: Vec<Declaration>,
    pub regions: Vec<Vec<Node>>,
    pub exprs: Vec<Expr>,
    pub attrs: Vec<ElementAttrs>,
    pub loose: Vec<usize>,
    pub callers: Vec<BTreeSet<Option<usize>>>,
    pub callees: Vec<BTreeSet<usize>>,
    embedded: Option<usize>,
    by_name: HashMap<String, Vec<usize>>,
    named: HashMap<String, Vec<usize>>,
    components: HashMap<String, Vec<usize>>,
    places: HashMap<(usize, Vec<String>), Vec<usize>>,
    known: HashSet<(usize, Vec<String>)>,
    region_ids: HashMap<Vec<Node>, usize>,
    expr_ids: HashMap<Expr, usize>,
    attr_ids: HashMap<ElementAttrs, usize>,
}

impl Index {
    pub fn build(sources: &[Source], embedded: Option<usize>, sink: &mut Sink) -> Self {
        let mut index = Self {
            files: Vec::new(),
            modules: Vec::new(),
            functions: Vec::new(),
            blocks: Vec::new(),
            declarations: Vec::new(),
            regions: Vec::new(),
            exprs: Vec::new(),
            attrs: vec![ElementAttrs::default()],
            loose: Vec::new(),
            callers: Vec::new(),
            callees: Vec::new(),
            embedded,
            by_name: HashMap::new(),
            named: HashMap::new(),
            components: HashMap::new(),
            places: HashMap::new(),
            known: HashSet::new(),
            region_ids: HashMap::new(),
            expr_ids: HashMap::new(),
            attr_ids: HashMap::from([(ElementAttrs::default(), NO_ATTRS)]),
        };
        for source in sources {
            let name: Arc<str> = source.name.as_str().into();
            match syn::parse_file(&source.text) {
                Ok(parsed) => collect::file(&mut index, name, source, &parsed, sink),
                Err(error) => sink.error(&Loc::new(&name, error.span()), error.to_string(), None),
            }
        }
        graph::compile(&mut index, sink);
        index
    }

    pub fn roots(&self) -> Vec<usize> {
        let count = self.functions.len();
        let mut seen = vec![false; count];
        let loose: Vec<usize> = (0..count).filter(|&function| self.callers[function].contains(&None)).collect();
        self.reach(&loose, &mut seen);

        let mut roots = Vec::new();
        let entries = (0..count).filter(|&function| !self.functions[function].component || self.callers[function].is_empty());
        for function in entries.clone() {
            roots.push(function);
            self.reach(&[function], &mut seen);
        }
        for function in 0..count {
            if !seen[function] {
                roots.push(function);
                self.reach(&[function], &mut seen);
            }
        }
        roots
    }

    fn reach(&self, from: &[usize], seen: &mut [bool]) {
        let mut stack = from.to_vec();
        while let Some(function) = stack.pop() {
            if !std::mem::replace(&mut seen[function], true) {
                stack.extend(self.callees[function].iter().copied());
            }
        }
    }
}
