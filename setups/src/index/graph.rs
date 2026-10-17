use std::collections::BTreeSet;

use bq_html::ast::{
    Attribute, ComponentArg, ControlFlowKind, Element as HtmlElement, ElementBody, ElementName, IfOrBlock, Markup, Markups,
};
use syn::{Lit, Stmt, spanned::Spanned};

use super::{Call, Element, Expr, Index, Node, Splice};
use crate::{
    attrs::{self, ElementAttrs, Lexical},
    diagnostics::{Loc, Sink},
    html::{Flags, HtmlFinder, parse},
    model::TagSet,
};

const ALIAS_DEPTH: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Callee {
    Function(usize),
    Ambiguous { name: String, files: Vec<String> },
    Missing,
}

impl Callee {
    pub fn ambiguity(&self) -> Option<String> {
        match self {
            Callee::Ambiguous { name, files } => Some(format!(
                "`{name}` matches more than one function ({}), so bq_setups does not follow it",
                files.join(", ")
            )),
            _ => None,
        }
    }
}

enum Resolved {
    Found(usize),
    Ambiguous(Vec<usize>),
    Missing,
}

enum Located {
    Ours(usize, Vec<String>),
    Extern,
}

enum Path {
    Segments(Vec<String>),
    SelfMethod(String),
}

pub fn compile(index: &mut Index, sink: &mut Sink) {
    for block in 0..index.blocks.len() {
        let Some(markups) = index.blocks[block].markups.take() else {
            continue;
        };
        let file = index.blocks[block].file;
        let function = index.blocks[block].function;
        let module = function.map_or(index.files[file].module, |function| index.functions[function].module);
        let region = Compiler {
            index,
            sink,
            file,
            function,
            module,
        }
        .markups(&markups);
        index.blocks[block].region = region;
    }
}

struct Compiler<'i> {
    index: &'i mut Index,
    sink: &'i mut Sink,
    file: usize,
    function: Option<usize>,
    module: usize,
}

impl Compiler<'_> {
    fn loc(&self, span: proc_macro2::Span) -> Loc {
        Loc::new(&self.index.files[self.file].name, span)
    }

    fn markups(&mut self, markups: &Markups<HtmlElement>) -> usize {
        let mut nodes = Vec::new();
        for markup in &markups.markups {
            self.markup(markup, &mut nodes);
        }
        self.index.region(nodes)
    }

    fn single(&mut self, markup: &Markup<HtmlElement>) -> usize {
        let mut nodes = Vec::new();
        self.markup(markup, &mut nodes);
        self.index.region(nodes)
    }

    fn markup(&mut self, markup: &Markup<HtmlElement>, nodes: &mut Vec<Node>) {
        match markup {
            Markup::Block(block) => {
                for markup in &block.markups.markups {
                    self.markup(markup, nodes);
                }
            }
            Markup::Lit(_) | Markup::Semi(_) => {}
            Markup::Splice { expr, .. } => nodes.push(Node::Splice(self.splice(expr))),
            Markup::Element(element) => nodes.push(Node::Element(self.element(element))),
            Markup::ControlFlow(control_flow) => match &control_flow.kind {
                ControlFlowKind::If(if_) => {
                    let mut regions = Vec::new();
                    let mut next = Some(if_);
                    while let Some(if_) = next.take() {
                        regions.push(self.markups(&if_.then_branch.markups));
                        match if_.else_branch.as_ref().map(|(_, _, else_)| else_.as_ref()) {
                            Some(IfOrBlock::If(else_if)) => next = Some(else_if),
                            Some(IfOrBlock::Block(block)) => regions.push(self.markups(&block.markups)),
                            None => {}
                        }
                    }
                    nodes.push(Node::Branch {
                        flags: Flags::OPTIONAL,
                        regions,
                    });
                }
                ControlFlowKind::For(for_) => {
                    let regions = vec![self.markups(&for_.body.markups)];
                    nodes.push(Node::Branch {
                        flags: Flags::MANY,
                        regions,
                    });
                }
                ControlFlowKind::While(while_) => {
                    let regions = vec![self.markups(&while_.body.markups)];
                    nodes.push(Node::Branch {
                        flags: Flags::OPTIONAL,
                        regions,
                    });
                }
                ControlFlowKind::Match(match_) => {
                    let regions = match_.arms.iter().map(|arm| self.single(&arm.body)).collect();
                    nodes.push(Node::Branch {
                        flags: Flags::OPTIONAL,
                        regions,
                    });
                }
                ControlFlowKind::Let(local) => {
                    if let Some(init) = &local.init {
                        nodes.push(Node::Expr(self.expr(&init.expr)));
                    }
                }
            },
        }
    }

    fn expr(&mut self, expr: &syn::Expr) -> usize {
        if let syn::Expr::Path(path) = expr
            && let Some(name) = path.path.get_ident()
            && let Some(function) = self.function
            && let Some(&block) = self.index.functions[function].bindings.get(&name.to_string())
        {
            self.index.blocks[block].spliced = true;
            return self.index.expr(Expr {
                binding: Some(block),
                nested: Vec::new(),
            });
        }

        let mut nested = Vec::new();
        for found in HtmlFinder::expr(expr) {
            let flags = found.flags;
            match parse(found, &self.index.files[self.file].name, self.sink) {
                Some(markups) => nested.push((flags, self.markups(&markups))),
                None => {
                    if let Some(function) = self.function {
                        self.index.functions[function].unparsed = true;
                    }
                }
            }
        }
        self.index.expr(Expr { binding: None, nested })
    }

    fn splice(&mut self, expr: &syn::Expr) -> Splice {
        let callee = called(expr).map(|path| {
            let loc = self.loc(expr.span());
            (self.callee(&path, false), loc)
        });
        Splice {
            children: is_children(expr),
            name: spliced_name(expr),
            callee,
            expr: self.expr(expr),
        }
    }

    fn element(&mut self, element: &HtmlElement) -> Element {
        let loc = self.loc(element.span());
        let lexical = self.function.map(|function| {
            let function = &self.index.functions[function];
            (function.name.clone(), function.component, function.setup.clone())
        });
        let parsed = attrs::element(
            &element.attrs,
            &loc,
            lexical.as_ref().map(|(name, component, setup)| Lexical {
                name,
                component: *component,
                setup: setup.as_deref(),
            }),
            self.sink,
        );
        let attrs = self.index.element_attrs(parsed);
        let spread = element.attrs.iter().any(|attr| matches!(attr, Attribute::Spread { .. }));

        let (tags, call) = match &element.name {
            None => (TagSet::one("div"), None),
            Some(ElementName::Static(name)) => (TagSet::one(name.to_string()), None),
            Some(ElementName::Dynamic { expr, .. }) => (literal_tags(expr), None),
            Some(ElementName::Component { path, args, .. }) => {
                let mut positional = Vec::new();
                let mut named = Vec::new();
                for arg in args {
                    match arg {
                        ComponentArg::Positional(expr) => positional.push(self.expr(expr)),
                        ComponentArg::Named { name, value, .. } => named.push((name.to_string(), self.expr(value))),
                    }
                }
                let segments = path.segments.iter().map(|segment| segment.ident.to_string()).collect();
                let callee = self.callee(&Path::Segments(segments), true);
                (
                    TagSet::any(),
                    Some(Call {
                        callee,
                        loc,
                        positional,
                        named,
                    }),
                )
            }
        };

        let body = match &element.body {
            ElementBody::Block(block) => Some(self.markups(&block.markups)),
            ElementBody::Void(_) => None,
        };
        Element {
            tags,
            attrs,
            spread,
            call,
            body,
        }
    }

    fn callee(&mut self, path: &Path, component: bool) -> Callee {
        let resolved = match path {
            Path::Segments(segments) => self.index.resolve(self.module, self.file, segments, component),
            Path::SelfMethod(name) => self.index.method(self.module, self.file, name),
        };
        match resolved {
            Resolved::Found(function) => {
                if component {
                    self.index.callers[function].insert(self.function);
                    if let Some(caller) = self.function {
                        self.index.callees[caller].insert(function);
                    }
                }
                Callee::Function(function)
            }
            Resolved::Missing => Callee::Missing,
            Resolved::Ambiguous(candidates) => Callee::Ambiguous {
                name: match path {
                    Path::Segments(segments) => segments.last().cloned().unwrap_or_default(),
                    Path::SelfMethod(name) => name.clone(),
                },
                files: candidates
                    .iter()
                    .map(|&candidate| self.index.files[self.index.functions[candidate].file].name.to_string())
                    .collect(),
            },
        }
    }
}

impl Index {
    fn region(&mut self, nodes: Vec<Node>) -> usize {
        if let Some(&region) = self.region_ids.get(&nodes) {
            return region;
        }
        self.regions.push(nodes.clone());
        self.region_ids.insert(nodes, self.regions.len() - 1);
        self.regions.len() - 1
    }

    fn expr(&mut self, expr: Expr) -> usize {
        if let Some(&id) = self.expr_ids.get(&expr) {
            return id;
        }
        self.exprs.push(expr.clone());
        self.expr_ids.insert(expr, self.exprs.len() - 1);
        self.exprs.len() - 1
    }

    fn element_attrs(&mut self, attrs: ElementAttrs) -> usize {
        if let Some(&id) = self.attr_ids.get(&attrs) {
            return id;
        }
        self.attrs.push(attrs.clone());
        self.attr_ids.insert(attrs, self.attrs.len() - 1);
        self.attrs.len() - 1
    }

    fn resolve(&self, module: usize, file: usize, path: &[String], component: bool) -> Resolved {
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

    fn method(&self, module: usize, file: usize, name: &str) -> Resolved {
        let here = &self.modules[module];
        if here.scope == module {
            return Resolved::Missing;
        }
        self.item(here.root, &here.path, name, false, file, 0)
    }

    fn table(&self, component: bool) -> &std::collections::HashMap<String, Vec<usize>> {
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
        modules.into_iter().find_map(|module| {
            let alias = self.modules[module].uses.get(segment)?;
            match self.locate(module, alias, depth + 1)? {
                Located::Ours(alias_root, target) if alias_root == root => Some(target),
                _ => None,
            }
        })
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

fn called(expr: &syn::Expr) -> Option<Path> {
    match expr {
        syn::Expr::Call(call) => match call.func.as_ref() {
            syn::Expr::Path(path) => Some(Path::Segments(
                path.path.segments.iter().map(|segment| segment.ident.to_string()).collect(),
            )),
            _ => None,
        },
        syn::Expr::MethodCall(method) if matches!(method.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self")) => {
            Some(Path::SelfMethod(method.method.to_string()))
        }
        syn::Expr::MethodCall(method) => called(&method.receiver),
        syn::Expr::Reference(reference) => called(&reference.expr),
        syn::Expr::Paren(paren) => called(&paren.expr),
        _ => None,
    }
}

fn spliced_name(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) if path.qself.is_none() => path.path.get_ident().map(ToString::to_string),
        syn::Expr::Reference(reference) => spliced_name(&reference.expr),
        syn::Expr::Paren(paren) => spliced_name(&paren.expr),
        _ => None,
    }
}

fn is_children(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(path) => path.qself.is_none() && path.path.is_ident(bq_html::conventions::CHILDREN),
        syn::Expr::Reference(reference) => is_children(&reference.expr),
        syn::Expr::Paren(paren) => is_children(&paren.expr),
        _ => false,
    }
}

fn literal_tags(expr: &syn::Expr) -> TagSet {
    fn collect(expr: &syn::Expr, out: &mut BTreeSet<String>) -> bool {
        match expr {
            syn::Expr::Lit(lit) => match &lit.lit {
                Lit::Str(text) => out.insert(text.value()) || true,
                _ => false,
            },
            syn::Expr::Paren(paren) => collect(&paren.expr, out),
            syn::Expr::Group(group) => collect(&group.expr, out),
            syn::Expr::Block(block) => tail(&block.block, out),
            syn::Expr::If(if_) => tail(&if_.then_branch, out) && if_.else_branch.as_ref().is_some_and(|(_, else_)| collect(else_, out)),
            syn::Expr::Match(match_) => match_.arms.iter().all(|arm| collect(&arm.body, out)),
            _ => false,
        }
    }

    fn tail(block: &syn::Block, out: &mut BTreeSet<String>) -> bool {
        matches!(block.stmts.as_slice(), [Stmt::Expr(expr, None)] if collect(expr, out))
    }

    let mut tags = BTreeSet::new();
    if collect(expr, &mut tags) {
        TagSet::known(tags)
    } else {
        TagSet::any()
    }
}
