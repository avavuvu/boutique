use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashSet},
    rc::Rc,
};

use bq_html::{
    ast::{
        Attribute, AttributeType, ControlFlowKind, DiagnosticParse, Element, ElementBody, ElementName, IfOrBlock,
        Markup, Markups, NoElement,
    },
    proc_macro2_diagnostics::Diagnostic,
};
use heck::{ToKebabCase, ToLowerCamelCase, ToUpperCamelCase};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Expr, Lit, Stmt, parse::Parser, spanned::Spanned, visit::Visit};

use crate::{
    index::Index,
    on::{self, On, Target},
};

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
    pub handlers: Vec<HandlerUse>,
    pub location: String,
}

pub struct HandlerUse {
    pub name: String,
    pub event: String,
    pub target: Target,
    pub tags: Tags,
}

pub struct RefUse {
    pub name: String,
    pub tags: Tags,
    pub optional: bool,
    pub many: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ref,
    Handler,
}

pub struct Claim {
    pub kind: Kind,
    pub name: String,
    pub owner: usize,
    pub between: Vec<usize>,
    pub text: String,
    pub location: String,
}

#[derive(Clone, Copy)]
struct Owner {
    usage: usize,
    optional: bool,
    many: bool,
}

#[derive(Clone, Default)]
struct Scope {
    lexical: Vec<usize>,
    path: Vec<Vec<Owner>>,
}

impl Scope {
    fn flagged(&self, optional: bool, many: bool) -> Scope {
        let mut scope = self.clone();
        for owner in scope.path.iter_mut().flatten() {
            owner.optional |= optional;
            owner.many |= many;
        }
        scope
    }

    fn owner(&self, usage: usize) -> Option<Owner> {
        self.path.iter().flatten().find(|owner| owner.usage == usage).copied()
    }

    fn between(&self, usage: usize) -> Vec<usize> {
        let mut between = Vec::new();
        for element in &self.path {
            between.extend(element.iter().filter(|owner| owner.usage != usage).map(|owner| owner.usage));
            if element.iter().any(|owner| owner.usage == usage) {
                break;
            }
        }
        between
    }
}

#[derive(Clone, Debug)]
struct RefName {
    setup: Option<String>,
    name: String,
    text: String,
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
    refs: Vec<(RefName, String)>,
    ons: Vec<(On, String)>,
}

#[derive(Clone)]
struct Outside {
    attrs: CallAttrs,
    lexical: Vec<usize>,
    function: Option<usize>,
    created: Rc<RefCell<Vec<usize>>>,
}

struct Frame<'c> {
    outside: Vec<Outside>,
    created: Rc<RefCell<Vec<usize>>>,
    children: Option<&'c Markups<Element>>,
    caller: Cx<'c>,
    attrs_used: Cell<bool>,
    children_used: Cell<bool>,
}

fn owners_or(own: &[usize], outer: &[usize]) -> Vec<usize> {
    if own.is_empty() { outer.to_vec() } else { own.to_vec() }
}

fn children_lexical(created: &[usize], caller: &[usize], path: &[Vec<Owner>]) -> Vec<usize> {
    let lexical = owners_or(created, caller);
    if !lexical.is_empty() {
        return lexical;
    }
    path.first().map(|element| element.iter().map(|owner| owner.usage).collect()).unwrap_or_default()
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
    pub claims: Vec<Claim>,
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
            claims: Vec::new(),
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

    pub fn finish(mut self) -> (Vec<Usage>, Vec<Claim>, Vec<String>, Vec<String>) {
        for (function, message) in std::mem::take(&mut self.orphans) {
            if !function.is_some_and(|function| self.followed.contains(&function)) {
                self.warnings.push(message);
            }
        }
        (self.usages, self.claims, unique(self.errors), unique(self.warnings))
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
                    let branch = Cx { scope: cx.scope.flagged(true, false), ..cx.clone() };
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
                    let each = Cx { scope: cx.scope.flagged(false, true), ..cx.clone() };
                    self.markups(&for_.body.markups, &each);
                }
                ControlFlowKind::While(while_) => {
                    let branch = Cx { scope: cx.scope.flagged(true, false), ..cx.clone() };
                    self.markups(&while_.body.markups, &branch);
                }
                ControlFlowKind::Match(match_) => {
                    let arm = Cx { scope: cx.scope.flagged(true, false), ..cx.clone() };
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
                let lexical = children_lexical(&frame.created.borrow(), &frame.caller.scope.lexical, &cx.scope.path);
                let inner = Cx { scope: Scope { lexical, path: cx.scope.path.clone() }, ..frame.caller.clone() };
                self.markups(children, &inner);
            }
            return;
        }

        if !cx.scope.path.is_empty()
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
        let outside = match cx.frame {
            Some(frame) if element.attrs.iter().any(|attr| matches!(attr, Attribute::Spread { .. })) => {
                frame.attrs_used.set(true);
                frame.outside.clone()
            }
            _ => Vec::new(),
        };
        let inner = self.place(&tags, own, &outside, cx);

        if let ElementBody::Block(block) = &element.body {
            self.markups(&block.markups, &Cx { scope: inner, ..cx.clone() });
        }
    }

    fn component(&mut self, name: &str, element: &Element, location: &str, cx: &Cx) {
        let created = Rc::new(RefCell::new(Vec::new()));
        let mut outside = vec![Outside {
            attrs: self.call_attrs(&element.attrs, location),
            lexical: cx.scope.lexical.clone(),
            function: cx.function,
            created: created.clone(),
        }];
        if element.attrs.iter().any(|attr| matches!(attr, Attribute::Spread { .. }))
            && let Some(frame) = cx.frame
        {
            frame.attrs_used.set(true);
            outside.extend(frame.outside.iter().cloned());
        }
        let children = match &element.body {
            ElementBody::Block(block) => Some(&block.markups),
            ElementBody::Void(_) => None,
        };

        let function = self.lookup(name, cx.file, true, location).filter(|function| !self.stack.contains(function));
        let Some(function) = function else {
            let inner = self.place(&Tags::Unknown, CallAttrs::default(), &outside, cx);
            if let Some(children) = children {
                let lexical = children_lexical(&created.borrow(), &cx.scope.lexical, &inner.path);
                self.markups(children, &Cx { scope: Scope { lexical, path: inner.path }, ..cx.clone() });
            }
            return;
        };

        self.reached.insert(function);
        if !cx.scope.path.is_empty() {
            self.followed.insert(function);
        }

        let frame = Frame {
            outside,
            created: created.clone(),
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

        let path = if frame.attrs_used.get() {
            cx.scope.path.clone()
        } else {
            self.place(&Tags::Unknown, CallAttrs::default(), &frame.outside, cx).path
        };
        if !frame.children_used.get()
            && let Some(children) = children
        {
            let lexical = children_lexical(&created.borrow(), &cx.scope.lexical, &path);
            self.markups(children, &Cx { scope: Scope { lexical, path }, ..cx.clone() });
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
                    for reference in self.ref_names(attr_type, location) {
                        call.refs.push((reference, location.to_string()));
                    }
                }
                attribute if attribute.starts_with("bq-on:") => {
                    let value = match attr_type {
                        AttributeType::Normal { value: Markup::Lit(lit), .. } => lit.lit.value(),
                        AttributeType::Empty(None) => String::new(),
                        _ => {
                            self.errors.push(format!(
                                "{location}: {attribute} needs a fixed string, for example {attribute}=\"save\""
                            ));
                            continue;
                        }
                    };
                    match on::parse(attribute, &value) {
                        Ok(ons) => call.ons.extend(ons.into_iter().map(|on| (on, location.to_string()))),
                        Err(message) => self.errors.push(format!("{location}: {message}")),
                    }
                }
                _ => {}
            }
        }
        call
    }

    fn place(&mut self, tags: &Tags, own: CallAttrs, outside: &[Outside], cx: &Cx) -> Scope {
        for (reference, location) in own.refs {
            self.reference(reference, tags, &cx.scope.lexical, &cx.scope, location, cx.function);
        }
        for item in outside {
            for (reference, location) in &item.attrs.refs {
                self.reference(reference.clone(), tags, &item.lexical, &cx.scope, location.clone(), item.function);
            }
        }

        let own_usages = self.create(tags, own.setups);
        let mut entry = own_usages.clone();
        let mut outside_usages = Vec::new();
        for item in outside {
            let usages = self.create(tags, item.attrs.setups.clone());
            entry.extend(&usages);
            *item.created.borrow_mut() = usages.clone();
            outside_usages.push(usages);
        }

        let mut path = cx.scope.path.clone();
        if !entry.is_empty() {
            path.insert(0, entry.into_iter().map(|usage| Owner { usage, optional: false, many: false }).collect());
        }
        let inner = Scope { lexical: owners_or(&own_usages, &cx.scope.lexical), path };

        for (on, location) in own.ons {
            self.handler(on, tags, &inner.lexical, &inner, location, cx.function);
        }
        for (item, usages) in outside.iter().zip(outside_usages) {
            let lexical = owners_or(&usages, &item.lexical);
            for (on, location) in &item.attrs.ons {
                self.handler(on.clone(), tags, &lexical, &inner, location.clone(), item.function);
            }
        }
        inner
    }

    fn create(&mut self, tags: &Tags, setups: Vec<(String, String)>) -> Vec<usize> {
        setups
            .into_iter()
            .map(|(name, location)| {
                self.usages.push(Usage { name, tags: tags.clone(), refs: Vec::new(), handlers: Vec::new(), location });
                self.usages.len() - 1
            })
            .collect()
    }

    fn owner(
        &mut self,
        setup: Option<&str>,
        lexical: &[usize],
        scope: &Scope,
        text: &str,
        location: &str,
        function: Option<usize>,
    ) -> Option<(usize, bool)> {
        if let Some(setup) = setup {
            let found = scope.path.iter().flatten().find(|owner| self.usages[owner.usage].name.to_kebab_case() == setup);
            if found.is_none() {
                let message = format!("{location}: {text} is not inside an element with bq-setup=({})", setup.to_upper_camel_case());
                self.orphans.push((function, message));
            }
            return found.map(|owner| (owner.usage, false));
        }

        let mut names: Vec<String> = lexical.iter().map(|&owner| self.usages[owner].name.to_kebab_case()).collect();
        names.dedup();
        match (lexical, names.as_slice()) {
            ([], _) => {
                self.orphans.push((function, format!("{location}: {text} is not inside an element with bq-setup")));
                None
            }
            ([owner, ..], [_]) => Some((*owner, true)),
            _ => {
                let (name, value) = text.split_once('=').unwrap_or((text, "\"\""));
                let value = value.trim_matches('"');
                self.errors.push(format!(
                    "{location}: {text} is inside more than one setup ({}); write the setup name, for example {name}=\"{}.{value}\"",
                    names.join(", "),
                    names[0]
                ));
                None
            }
        }
    }

    fn claim(&mut self, kind: Kind, name: &str, owner: usize, scope: &Scope, text: String, location: String) {
        let between = scope.between(owner);
        if !between.is_empty() {
            self.claims.push(Claim { kind, name: name.to_string(), owner, between, text, location });
        }
    }

    fn handler(&mut self, on: On, tags: &Tags, lexical: &[usize], scope: &Scope, location: String, function: Option<usize>) {
        let value = match &on.setup {
            Some(setup) => format!("{setup}.{}", on.handler),
            None => on.handler.clone(),
        };
        let text = format!("{}=\"{value}\"", on.attribute);
        let Some((owner, unprefixed)) = self.owner(on.setup.as_deref(), lexical, scope, &text, &location, function) else {
            return;
        };
        if unprefixed {
            self.claim(Kind::Handler, &on.key, owner, scope, text, location);
        }
        self.usages[owner].handlers.push(HandlerUse { name: on.key, event: on.event, target: on.target, tags: tags.clone() });
    }

    fn reference(&mut self, reference: RefName, tags: &Tags, lexical: &[usize], scope: &Scope, location: String, function: Option<usize>) {
        let text = format!("bq-ref=\"{}\"", reference.text);
        let Some((owner, unprefixed)) = self.owner(reference.setup.as_deref(), lexical, scope, &text, &location, function) else {
            return;
        };
        if unprefixed {
            self.claim(Kind::Ref, &reference.name, owner, scope, text, location);
        }
        let flags = scope.owner(owner);
        self.usages[owner].refs.push(RefUse {
            name: reference.name,
            tags: tags.clone(),
            optional: flags.is_some_and(|flags| flags.optional),
            many: flags.is_some_and(|flags| flags.many),
        });
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

    fn ref_names(&mut self, value: &AttributeType, location: &str) -> Vec<RefName> {
        let AttributeType::Normal { value: Markup::Lit(lit), .. } = value else {
            self.errors.push(format!("{location}: bq-ref needs a fixed string, for example bq-ref=\"select\""));
            return Vec::new();
        };

        let value = lit.lit.value();
        if value.split_whitespace().next().is_none() {
            self.errors.push(format!("{location}: bq-ref needs a name, for example bq-ref=\"select\""));
        }

        let mut names = Vec::new();
        for text in value.split_whitespace() {
            let (setup, name) = match text.split_once('.') {
                Some((setup, name)) => (Some(setup), name),
                None => (None, text),
            };
            if !on::kebab(name) || setup.is_some_and(|setup| !on::kebab(setup)) {
                self.errors.push(format!(
                    "{location}: bq-ref=\"{text}\" must be lowercase kebab-case, for example bq-ref=\"close-button\" or bq-ref=\"menu.close-button\""
                ));
                continue;
            }
            names.push(RefName { setup: setup.map(str::to_string), name: name.to_lower_camel_case(), text: text.to_string() });
        }
        names
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
