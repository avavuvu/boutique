use std::{
    cell::Cell,
    collections::{BTreeSet, HashSet},
};

use bq_html::{
    ast::{
        Attribute, AttributeType, ControlFlowKind, DiagnosticParse, Element, ElementBody, ElementName, IfOrBlock,
        Markup, Markups, NoElement,
    },
    proc_macro2_diagnostics::Diagnostic,
};
use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Expr, Lit, Stmt, parse::Parser, spanned::Spanned, visit::Visit};

use crate::index::Index;

#[derive(Clone, Debug)]
pub enum Tags {
    Known(BTreeSet<String>),
    Unknown,
}

impl Tags {
    fn one(tag: impl Into<String>) -> Self {
        Self::Known(BTreeSet::from([tag.into()]))
    }
}

pub struct Usage {
    pub name: String,
    pub tags: Tags,
    pub refs: Vec<RefUse>,
    pub location: String,
}

pub struct RefUse {
    pub name: String,
    pub tags: Tags,
    pub optional: bool,
    pub many: bool,
    pub location: String,
}

#[derive(Clone, Default)]
struct Scope {
    owners: Vec<usize>,
    optional: bool,
    many: bool,
}

#[derive(Clone)]
struct Cx<'c> {
    scope: Scope,
    frame: Option<&'c Frame<'c>>,
    file: usize,
    function: Option<usize>,
}

#[derive(Clone, Default)]
struct CallAttrs {
    setups: Vec<(String, String)>,
    reference: Option<(String, String)>,
}


struct Frame<'c> {
    attrs: CallAttrs,
    children: Option<&'c Markups<Element>>,
    caller: Cx<'c>,
    attrs_used: Cell<bool>,
    children_used: Cell<bool>,
}

pub fn parse(tokens: TokenStream) -> Option<Markups<Element>> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    Parser::parse2(|input: syn::parse::ParseStream| Markups::diagnostic_parse(input, &mut diagnostics), tokens).ok()
}

pub fn html_blocks(tokens: TokenStream, out: &mut Vec<TokenStream>) {
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

pub struct Walker<'a> {
    index: &'a Index,
    pub usages: Vec<Usage>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub reached: HashSet<usize>,
    followed: HashSet<usize>,
    orphans: Vec<(Option<usize>, String)>,
    stack: Vec<usize>,
}

impl<'a> Walker<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self {
            index,
            usages: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            reached: HashSet::new(),
            followed: HashSet::new(),
            orphans: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn block(&mut self, block: usize) {
        let block = &self.index.blocks[block];
        let cx = Cx { scope: Scope::default(), frame: None, file: block.file, function: block.function };
        self.markups(&block.markups, &cx);
    }

    pub fn finish(mut self) -> (Vec<Usage>, Vec<String>, Vec<String>) {
        for (function, message) in std::mem::take(&mut self.orphans) {
            if !function.is_some_and(|function| self.followed.contains(&function)) {
                self.warnings.push(message);
            }
        }
        (self.usages, unique(self.errors), unique(self.warnings))
    }

    fn location(&self, file: usize, span: Span) -> String {
        format!("{}:{}", self.index.files[file].name, span.start().line)
    }

    fn markups(&mut self, markups: &Markups<Element>, cx: &Cx) {
        for markup in &markups.markups {
            self.markup(markup, cx);
        }
    }

    fn markup(&mut self, markup: &Markup<Element>, cx: &Cx) {
        match markup {
            Markup::Block(block) => self.markups(&block.markups, cx),
            Markup::Lit(_) | Markup::Semi(_) => {}
            Markup::Splice { expr, .. } => self.splice(expr, cx),
            Markup::Element(element) => self.element(element, cx),
            Markup::ControlFlow(control_flow) => match &control_flow.kind {
                ControlFlowKind::If(if_) => {
                    let branch = Cx { scope: Scope { optional: true, ..cx.scope.clone() }, ..cx.clone() };
                    let mut next = Some(if_);
                    while let Some(if_) = next.take() {
                        self.markups(&if_.then_branch.markups, &branch);
                        match if_.else_branch.as_ref().map(|(_, _, else_)| else_.as_ref()) {
                            Some(IfOrBlock::If(else_if)) => next = Some(else_if),
                            Some(IfOrBlock::Block(block)) => self.markups(&block.markups, &branch),
                            None => {}
                        }
                    }
                }
                ControlFlowKind::For(for_) => {
                    let each = Cx { scope: Scope { many: true, ..cx.scope.clone() }, ..cx.clone() };
                    self.markups(&for_.body.markups, &each);
                }
                ControlFlowKind::While(while_) => {
                    let branch = Cx { scope: Scope { optional: true, ..cx.scope.clone() }, ..cx.clone() };
                    self.markups(&while_.body.markups, &branch);
                }
                ControlFlowKind::Match(match_) => {
                    let arm = Cx { scope: Scope { optional: true, ..cx.scope.clone() }, ..cx.clone() };
                    for match_arm in &match_.arms {
                        self.markup(&match_arm.body, &arm);
                    }
                }
                ControlFlowKind::Let(local) => {
                    if let Some(init) = &local.init {
                        self.nested(&init.expr, cx);
                    }
                }
            },
        }
    }

    fn splice(&mut self, expr: &Expr, cx: &Cx) {
        if let Some(frame) = cx.frame.filter(|_| is_children(expr)) {
            frame.children_used.set(true);
            if let Some(children) = frame.children {
                let inner = Cx { scope: cx.scope.clone(), ..frame.caller.clone() };
                self.markups(children, &inner);
            }
            return;
        }

        if !cx.scope.owners.is_empty()
            && let Some(name) = called_function(expr)
            && let location = self.location(cx.file, expr.span())
            && let Some(function) = self.lookup(&name, cx.file, false, &location)
            && !self.stack.contains(&function)
        {
            self.followed.insert(function);
            self.stack.push(function);
            let followed = Cx { frame: None, file: self.index.functions[function].file, ..cx.clone() };
            for &block in &self.index.functions[function].blocks {
                self.markups(&self.index.blocks[block].markups, &followed);
            }
            self.stack.pop();
        }

        self.nested(expr, cx);
    }

    fn nested(&mut self, expr: &Expr, cx: &Cx) {
        let mut finder = MacroFinder::default();
        finder.visit_expr(expr);
        for tokens in finder.blocks {
            if let Some(markups) = parse(tokens) {
                self.markups(&markups, cx);
            }
        }
    }

    fn element(&mut self, element: &Element, cx: &Cx) {
        let location = self.location(cx.file, element.span());
        let tags = match &element.name {
            None => Tags::one("div"),
            Some(ElementName::Static(name)) => Tags::one(name.to_string()),
            Some(ElementName::Dynamic { expr, .. }) => literal_tags(expr),
            Some(ElementName::Component { path, .. }) => {
                let name = path.segments.last().map(|segment| segment.ident.to_string()).unwrap_or_default();
                return self.component(&name, element, &location, cx);
            }
        };

        let own = self.call_attrs(&element.attrs, &location);
        let forwarded = cx.frame.filter(|_| element.attrs.iter().any(|attr| matches!(attr, Attribute::Spread { .. })));

        if let Some((reference, location)) = own.reference {
            self.reference(reference, &tags, &cx.scope, location, cx.function);
        }
        let mut setups = own.setups;
        if let Some(frame) = forwarded {
            frame.attrs_used.set(true);
            if let Some((reference, location)) = frame.attrs.reference.clone() {
                self.reference(reference, &tags, &cx.scope, location, frame.caller.function);
            }
            setups.extend(frame.attrs.setups.iter().cloned());
        }
        let inner = self.owners(&tags, setups, cx.scope.clone());

        if let ElementBody::Block(block) = &element.body {
            self.markups(&block.markups, &Cx { scope: inner, ..cx.clone() });
        }
    }

    fn component(&mut self, name: &str, element: &Element, location: &str, cx: &Cx) {
        let mut attrs = self.call_attrs(&element.attrs, location);
        if element.attrs.iter().any(|attr| matches!(attr, Attribute::Spread { .. }))
            && let Some(frame) = cx.frame
        {
            frame.attrs_used.set(true);
            if attrs.reference.is_none() {
                attrs.reference = frame.attrs.reference.clone();
            }
            attrs.setups.extend(frame.attrs.setups.iter().cloned());
        }
        let children = match &element.body {
            ElementBody::Block(block) => Some(&block.markups),
            ElementBody::Void(_) => None,
        };

        let function = self.lookup(name, cx.file, true, location).filter(|function| !self.stack.contains(function));
        let Some(function) = function else {
            let inner = self.apply(&Tags::Unknown, attrs, cx);
            if let Some(children) = children {
                self.markups(children, &Cx { scope: inner, ..cx.clone() });
            }
            return;
        };

        self.reached.insert(function);
        if !cx.scope.owners.is_empty() {
            self.followed.insert(function);
        }

        let frame = Frame {
            attrs,
            children,
            caller: cx.clone(),
            attrs_used: Cell::new(false),
            children_used: Cell::new(false),
        };
        let body = Cx {
            scope: cx.scope.clone(),
            frame: Some(&frame),
            file: self.index.functions[function].file,
            function: Some(function),
        };

        self.stack.push(function);
        for &block in &self.index.functions[function].blocks {
            self.markups(&self.index.blocks[block].markups, &body);
        }
        self.stack.pop();

        let inner = if frame.attrs_used.get() {
            cx.scope.clone()
        } else {
            self.apply(&Tags::Unknown, frame.attrs.clone(), cx)
        };
        if !frame.children_used.get()
            && let Some(children) = children
        {
            self.markups(children, &Cx { scope: inner, ..cx.clone() });
        }
    }

    fn call_attrs(&mut self, attrs: &[Attribute], location: &str) -> CallAttrs {
        let mut call = CallAttrs::default();
        for attr in attrs {
            let Attribute::Named { name, attr_type } = attr else { continue };
            match name.to_string().as_str() {
                "bq-setup" => {
                    for setup in self.setup_names(attr_type, location) {
                        call.setups.push((setup, location.to_string()));
                    }
                }
                "bq-ref" => {
                    if let Some(reference) = self.ref_name(attr_type, location) {
                        call.reference = Some((reference, location.to_string()));
                    }
                }
                _ => {}
            }
        }
        call
    }

    fn apply(&mut self, tags: &Tags, attrs: CallAttrs, cx: &Cx) -> Scope {
        if let Some((reference, location)) = attrs.reference {
            self.reference(reference, tags, &cx.scope, location, cx.function);
        }
        self.owners(tags, attrs.setups, cx.scope.clone())
    }


    fn owners(&mut self, tags: &Tags, setups: Vec<(String, String)>, outer: Scope) -> Scope {
        if setups.is_empty() {
            return outer;
        }
        let mut owners = Vec::new();
        for (name, location) in setups {
            self.usages.push(Usage { name, tags: tags.clone(), refs: Vec::new(), location });
            owners.push(self.usages.len() - 1);
        }
        Scope { owners, optional: false, many: false }
    }

    fn reference(&mut self, name: String, tags: &Tags, scope: &Scope, location: String, function: Option<usize>) {
        if scope.owners.is_empty() {
            let message = format!("{location}: bq-ref=\"{name}\" is not inside an element with bq-setup");
            self.orphans.push((function, message));
        }
        for &owner in &scope.owners {
            self.usages[owner].refs.push(RefUse {
                name: name.clone(),
                tags: tags.clone(),
                optional: scope.optional,
                many: scope.many,
                location: location.clone(),
            });
        }
    }

    fn lookup(&mut self, name: &str, file: usize, component: bool, location: &str) -> Option<usize> {
        let table = if component { &self.index.components } else { &self.index.by_name };
        let candidates = table.get(name)?;
        let root = self.index.files[file].root;

        let in_file: Vec<usize> = candidates.iter().copied().filter(|&c| self.index.functions[c].file == file).collect();
        if let Some(&first) = in_file.first() {
            return Some(first);
        }
        let in_root: Vec<usize> =
            candidates.iter().copied().filter(|&c| self.index.files[self.index.functions[c].file].root == root).collect();
        match (in_root.as_slice(), candidates.as_slice()) {
            ([one], _) | ([], [one]) => Some(*one),
            _ => {
                let places: Vec<&str> =
                    candidates.iter().map(|&c| self.index.files[self.index.functions[c].file].name.as_str()).collect();
                self.warnings.push(format!(
                    "{location}: `{name}` matches more than one function ({}), so bq_setups does not follow it",
                    places.join(", ")
                ));
                None
            }
        }
    }

    fn setup_names(&mut self, value: &AttributeType, location: &str) -> Vec<String> {
        let message = || format!("{location}: bq-setup needs a name from setup!(..), for example bq-setup=(ThemeSelect)");

        let AttributeType::Normal { value, .. } = value else {
            self.errors.push(message());
            return Vec::new();
        };

        let mut names = Vec::new();
        let parts: Vec<&Markup<NoElement>> = match value {
            Markup::Block(block) => block.markups.markups.iter().collect(),
            other => vec![other],
        };
        for part in parts {
            match part {
                Markup::Splice { expr, .. } => match path_name(expr) {
                    Some(name) => names.push(name),
                    None => self.errors.push(message()),
                },
                Markup::Lit(lit) if lit.lit.value().trim().is_empty() => {}
                _ => self.errors.push(message()),
            }
        }
        names
    }

    fn ref_name(&mut self, value: &AttributeType, location: &str) -> Option<String> {
        let text = match value {
            AttributeType::Normal { value: Markup::Lit(lit), .. } => Some(lit.lit.value()),
            _ => None,
        };

        let Some(text) = text else {
            self.errors.push(format!("{location}: bq-ref needs a fixed string, for example bq-ref=\"select\""));
            return None;
        };

        let valid = text.chars().next().is_some_and(|first| first.is_ascii_lowercase())
            && !text.ends_with('-')
            && !text.contains("--")
            && text.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-');
        if !valid {
            self.errors.push(format!("{location}: bq-ref=\"{text}\" must be lowercase kebab-case, for example bq-ref=\"close-button\""));
            return None;
        }
        Some(text.to_lower_camel_case())
    }
}

#[derive(Default)]
struct MacroFinder {
    blocks: Vec<TokenStream>,
}

impl<'ast> Visit<'ast> for MacroFinder {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.segments.last().is_some_and(|segment| segment.ident == "html") {
            self.blocks.push(mac.tokens.clone());
        } else {
            html_blocks(mac.tokens.clone(), &mut self.blocks);
        }
    }
}

fn is_children(expr: &Expr) -> bool {
    match expr {
        Expr::Path(path) => path.qself.is_none() && path.path.is_ident("children"),
        Expr::Reference(reference) => is_children(&reference.expr),
        Expr::Paren(paren) => is_children(&paren.expr),
        _ => false,
    }
}

fn called_function(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Call(call) => match call.func.as_ref() {
            Expr::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
            _ => None,
        },
        Expr::MethodCall(method) => called_function(&method.receiver),
        Expr::Reference(reference) => called_function(&reference.expr),
        Expr::Paren(paren) => called_function(&paren.expr),
        _ => None,
    }
}

fn path_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(path) if path.qself.is_none() => path.path.segments.last().map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

fn literal_tags(expr: &Expr) -> Tags {
    fn collect(expr: &Expr, out: &mut BTreeSet<String>) -> bool {
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Str(text) => out.insert(text.value()) || true,
                _ => false,
            },
            Expr::Paren(paren) => collect(&paren.expr, out),
            Expr::Group(group) => collect(&group.expr, out),
            Expr::Block(block) => tail(&block.block, out),
            Expr::If(if_) => {
                tail(&if_.then_branch, out) && if_.else_branch.as_ref().is_some_and(|(_, else_)| collect(else_, out))
            }
            Expr::Match(match_) => match_.arms.iter().all(|arm| collect(&arm.body, out)),
            _ => false,
        }
    }

    fn tail(block: &syn::Block, out: &mut BTreeSet<String>) -> bool {
        matches!(block.stmts.as_slice(), [Stmt::Expr(expr, None)] if collect(expr, out))
    }

    let mut tags = BTreeSet::new();
    if collect(expr, &mut tags) && !tags.is_empty() { Tags::Known(tags) } else { Tags::Unknown }
}

fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    items.into_iter().filter(|item| seen.insert(item.clone())).collect()
}
