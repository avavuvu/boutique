use std::{
    cell::Cell,
    collections::{BTreeSet, HashSet},
};

use bq_html::{
    ast::{
        Attribute, AttributeType, ComponentArg, ControlFlowKind, DiagnosticParse, Element, ElementBody, ElementName, IfOrBlock, Markup,
        Markups,
    },
    proc_macro2_diagnostics::Diagnostic,
};
use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Expr, Lit, Stmt, parse::Parser, spanned::Spanned, visit::Visit};

use crate::{
    index::{Function, Index},
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

#[derive(Clone, Copy)]
struct Owner {
    usage: usize,
    optional: bool,
    many: bool,
}

#[derive(Clone, Default)]
struct Scope {
    path: Vec<Vec<Owner>>,
}

impl Scope {
    fn nearest(&self) -> Vec<usize> {
        self.path
            .first()
            .map(|element| element.iter().map(|owner| owner.usage).collect())
            .unwrap_or_default()
    }
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
    setups: Vec<String>,
    refs: Vec<(RefName, String)>,
    ons: Vec<(On, String)>,
}

#[derive(Clone)]
struct Outside {
    attrs: CallAttrs,
    function: Option<usize>,
}

struct Frame<'c> {
    outside: Vec<Outside>,
    args: Vec<(String, &'c Expr, Cell<bool>)>,
    children: Option<&'c Markups<Element>>,
    caller: Cx<'c>,
    attrs_used: Cell<bool>,
    children_used: Cell<bool>,
}

pub fn parse(tokens: TokenStream) -> Option<Markups<Element>> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    Parser::parse2(
        |input: syn::parse::ParseStream| Markups::diagnostic_parse(input, &mut diagnostics),
        tokens,
    )
    .ok()
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
    orphans: Vec<(Option<usize>, String, Option<String>)>,
    stack: Vec<usize>,
    spliced: HashSet<usize>,
    bound: Vec<usize>,
    unspliced: Option<(usize, String)>,
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
            spliced: HashSet::new(),
            bound: Vec::new(),
            unspliced: None,
        }
    }

    pub fn block(&mut self, block: usize) {
        let block = &self.index.blocks[block];
        let cx = Cx {
            scope: Scope::default(),
            frame: None,
            file: block.file,
            function: block.function,
        };
        self.markups(&block.markups, &cx);
    }

    pub fn finish(mut self) -> (Vec<Usage>, Vec<String>, Vec<String>) {
        for (function, message, binding) in std::mem::take(&mut self.orphans) {
            if function.is_some_and(|function| self.followed.contains(&function)) {
                continue;
            }
            match binding {
                Some(binding) => self.warnings.push(format!(
                    "{message}; it is in `{binding}`, a let-bound html! that is not spliced into another html!, so bq_setups cannot tell which setup it belongs to"
                )),
                None => self.errors.push(message),
            }
        }
        (self.usages, unique(self.errors), unique(self.warnings))
    }

    pub fn component_alone(&mut self, function: usize) {
        let cx = Cx {
            scope: Scope::default(),
            frame: None,
            file: self.index.functions[function].file,
            function: Some(function),
        };
        self.function_blocks(function, &cx);
    }

    pub fn unspliced_alone(&mut self, function: usize) {
        let cx = Cx {
            scope: Scope::default(),
            frame: None,
            file: self.index.functions[function].file,
            function: Some(function),
        };
        self.unspliced_blocks(function, &cx);
    }

    fn function_blocks(&mut self, function: usize, cx: &Cx) {
        for &block in &self.index.functions[function].blocks {
            let block = &self.index.blocks[block];
            if block.binding.is_none() {
                let scope = cx.scope.flagged(block.optional, block.many);
                self.markups(&block.markups, &Cx { scope, ..cx.clone() });
            }
        }
        self.unspliced_blocks(function, cx);
    }

    fn unspliced_blocks(&mut self, function: usize, cx: &Cx) {
        let index = self.index;
        for &block in index.functions[function].blocks.iter().rev() {
            let Some(binding) = &index.blocks[block].binding else { continue };
            if self.spliced.contains(&block) {
                continue;
            }
            let previous = self.unspliced.replace((function, binding.clone()));
            self.bound_block(block, cx);
            self.unspliced = previous;
        }
    }

    fn bound_block(&mut self, block: usize, cx: &Cx) {
        if self.bound.contains(&block) {
            return;
        }
        self.bound.push(block);
        let html = &self.index.blocks[block];
        let scope = cx.scope.flagged(html.optional, html.many);
        self.markups(&html.markups, &Cx { scope, ..cx.clone() });
        self.bound.pop();
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
                    let branch = Cx {
                        scope: cx.scope.flagged(true, false),
                        ..cx.clone()
                    };
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
                    let each = Cx {
                        scope: cx.scope.flagged(false, true),
                        ..cx.clone()
                    };
                    self.markups(&for_.body.markups, &each);
                }
                ControlFlowKind::While(while_) => {
                    let branch = Cx {
                        scope: cx.scope.flagged(true, false),
                        ..cx.clone()
                    };
                    self.markups(&while_.body.markups, &branch);
                }
                ControlFlowKind::Match(match_) => {
                    let arm = Cx {
                        scope: cx.scope.flagged(true, false),
                        ..cx.clone()
                    };
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
                self.markups(
                    children,
                    &Cx {
                        scope: cx.scope.clone(),
                        ..frame.caller.clone()
                    },
                );
            }
            return;
        }

        if let Some(frame) = cx.frame
            && let Some(name) = spliced_name(expr)
            && let Some((_, value, used)) = frame.args.iter().find(|(arg, ..)| *arg == name)
        {
            used.set(true);
            self.nested(
                value,
                &Cx {
                    scope: cx.scope.clone(),
                    ..frame.caller.clone()
                },
            );
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
            let followed = Cx {
                frame: None,
                file: self.index.functions[function].file,
                function: Some(function),
                ..cx.clone()
            };
            self.function_blocks(function, &followed);
            self.stack.pop();
        }

        self.nested(expr, cx);
    }

    fn nested(&mut self, expr: &Expr, cx: &Cx) {
        if let Expr::Path(path) = expr
            && let Some(name) = path.path.get_ident()
            && let Some(function) = cx.function
            && let Some(&block) = self.index.functions[function].bindings.get(&name.to_string())
        {
            self.spliced.insert(block);
            self.bound_block(block, cx);
            return;
        }

        let mut finder = MacroFinder::default();
        finder.visit_expr(expr);
        for (tokens, optional, many) in finder.blocks {
            if let Some(markups) = parse(tokens) {
                self.markups(
                    &markups,
                    &Cx {
                        scope: cx.scope.flagged(optional, many),
                        ..cx.clone()
                    },
                );
            }
        }
    }

    fn element(&mut self, element: &Element, cx: &Cx) {
        let location = self.location(cx.file, element.span());
        let tags = match &element.name {
            None => Tags::one("div"),
            Some(ElementName::Static(name)) => Tags::one(name.to_string()),
            Some(ElementName::Dynamic { expr, .. }) => literal_tags(expr),
            Some(ElementName::Component { path, args, .. }) => {
                let mut named = Vec::new();
                for arg in args {
                    match arg {
                        ComponentArg::Positional(expr) => self.nested(expr, cx),
                        ComponentArg::Named { name, value, .. } => named.push((name.to_string(), value, Cell::new(false))),
                    }
                }
                let name = path.segments.last().map(|segment| segment.ident.to_string()).unwrap_or_default();
                return self.component(&name, element, named, &location, cx);
            }
        };

        let own = self.call_attrs(&element.attrs, &location, cx.function);
        let outside = match cx.frame {
            Some(frame) if element.attrs.iter().any(|attr| matches!(attr, Attribute::Spread { .. })) => {
                frame.attrs_used.set(true);
                frame.outside.clone()
            }
            _ => Vec::new(),
        };
        let inner = self.place(&tags, own, &outside, cx);

        if let ElementBody::Block(block) = &element.body {
            self.markups(
                &block.markups,
                &Cx {
                    scope: inner,
                    ..cx.clone()
                },
            );
        }
    }

    fn component<'e>(&mut self, name: &str, element: &'e Element, args: Vec<(String, &'e Expr, Cell<bool>)>, location: &str, cx: &Cx) {
        let mut outside = vec![Outside {
            attrs: self.call_attrs(&element.attrs, location, cx.function),
            function: cx.function,
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

        let function = self
            .lookup(name, cx.file, true, location)
            .filter(|function| !self.stack.contains(function));
        let Some(function) = function else {
            for (_, value, _) in &args {
                self.nested(value, cx);
            }
            let inner = self.place(&Tags::Unknown, CallAttrs::default(), &outside, cx);
            if let Some(children) = children {
                self.markups(
                    children,
                    &Cx {
                        scope: inner,
                        ..cx.clone()
                    },
                );
            }
            return;
        };

        self.reached.insert(function);
        if !cx.scope.path.is_empty() {
            self.followed.insert(function);
        }

        let frame = Frame {
            outside,
            args,
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
        self.function_blocks(function, &body);
        self.stack.pop();

        for (_, value, used) in &frame.args {
            if !used.get() {
                self.nested(value, cx);
            }
        }

        let scope = if frame.attrs_used.get() {
            cx.scope.clone()
        } else {
            self.place(&Tags::Unknown, CallAttrs::default(), &frame.outside, cx)
        };
        if !frame.children_used.get()
            && let Some(children) = children
        {
            self.markups(children, &Cx { scope, ..cx.clone() });
        }
    }

    fn call_attrs(&mut self, attrs: &[Attribute], location: &str, function: Option<usize>) -> CallAttrs {
        let mut call = CallAttrs::default();
        for attr in attrs {
            let Attribute::Named { name, attr_type } = attr else { continue };
            match name.to_string().as_str() {
                "bq-setup" => {
                    if let Some(setup) = self.setup_name(attr_type, location, function) {
                        call.setups.push(setup);
                    }
                }
                "bq-ref" => {
                    for reference in self.ref_names(attr_type, location) {
                        call.refs.push((reference, location.to_string()));
                    }
                }
                attribute if attribute.starts_with("bq-on:") => {
                    let value = match attr_type {
                        AttributeType::Normal {
                            value: Markup::Lit(lit), ..
                        } => lit.lit.value(),
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
            self.reference(reference, tags, &cx.scope, location, cx.function);
        }
        for item in outside {
            for (reference, location) in &item.attrs.refs {
                self.reference(reference.clone(), tags, &cx.scope, location.clone(), item.function);
            }
        }

        let mut setups = own.setups;
        for item in outside {
            setups.extend(item.attrs.setups.iter().cloned());
        }
        let entry = self.create(tags, setups);
        let mut inner = cx.scope.clone();
        if !entry.is_empty() {
            inner.path.insert(
                0,
                entry
                    .into_iter()
                    .map(|usage| Owner {
                        usage,
                        optional: false,
                        many: false,
                    })
                    .collect(),
            );
        }

        for (on, location) in own.ons {
            self.handler(on, tags, &inner, location, cx.function);
        }
        for item in outside {
            for (on, location) in &item.attrs.ons {
                self.handler(on.clone(), tags, &inner, location.clone(), item.function);
            }
        }
        inner
    }

    fn create(&mut self, tags: &Tags, setups: Vec<String>) -> Vec<usize> {
        setups
            .into_iter()
            .map(|name| {
                self.usages.push(Usage {
                    name,
                    tags: tags.clone(),
                    refs: Vec::new(),
                    handlers: Vec::new(),
                });
                self.usages.len() - 1
            })
            .collect()
    }

    fn orphan(&mut self, function: Option<usize>, message: String) {
        let binding = match &self.unspliced {
            Some((unspliced, binding)) if function == Some(*unspliced) => Some(binding.clone()),
            _ => None,
        };
        self.orphans.push((function, message, binding));
    }

    fn owner(&mut self, setup: Option<&str>, scope: &Scope, text: &str, location: &str, function: Option<usize>) -> Option<usize> {
        if let Some(setup) = setup {
            let found = scope.path.iter().flatten().find(|owner| self.usages[owner.usage].name == setup);
            if found.is_none() {
                let message = match self.index.setups.iter().find(|declaration| declaration.name == setup) {
                    Some(declaration) => format!(
                        "{location}: {text} is not inside the element with bare bq-setup in component `{}`",
                        declaration.function
                    ),
                    None => format!("{location}: {text} names the setup \"{setup}\", but no #[component] has that setup"),
                };
                self.orphan(function, message);
            }
            return found.map(|owner| owner.usage);
        }

        let nearest = scope.nearest();
        let mut names: Vec<String> = nearest.iter().map(|&owner| self.usages[owner].name.clone()).collect();
        names.dedup();
        match (nearest.as_slice(), names.as_slice()) {
            ([], _) => {
                self.orphan(function, format!("{location}: {text} is not inside an element with bq-setup"));
                None
            }
            ([owner, ..], [_]) => Some(*owner),
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

    fn handler(&mut self, on: On, tags: &Tags, scope: &Scope, location: String, function: Option<usize>) {
        let value = match &on.setup {
            Some(setup) => format!("{setup}.{}", on.handler),
            None => on.handler.clone(),
        };
        let text = format!("{}=\"{value}\"", on.attribute);
        let Some(owner) = self.owner(on.setup.as_deref(), scope, &text, &location, function) else {
            return;
        };
        self.usages[owner].handlers.push(HandlerUse {
            name: on.key,
            event: on.event,
            target: on.target,
            tags: tags.clone(),
        });
    }

    fn reference(&mut self, reference: RefName, tags: &Tags, scope: &Scope, location: String, function: Option<usize>) {
        let text = format!("bq-ref=\"{}\"", reference.text);
        let Some(owner) = self.owner(reference.setup.as_deref(), scope, &text, &location, function) else {
            return;
        };
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

        let in_file: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&c| self.index.functions[c].file == file)
            .collect();
        if let Some(&first) = in_file.first() {
            return Some(first);
        }
        let in_root: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&c| self.index.files[self.index.functions[c].file].root == root)
            .collect();
        match (in_root.as_slice(), candidates.as_slice()) {
            ([one], _) | ([], [one]) => Some(*one),
            _ => {
                let places: Vec<&str> = candidates
                    .iter()
                    .map(|&c| self.index.files[self.index.functions[c].file].name.as_str())
                    .collect();
                self.warnings.push(format!(
                    "{location}: `{name}` matches more than one function ({}), so bq_setups does not follow it",
                    places.join(", ")
                ));
                None
            }
        }
    }

    fn setup_name(&mut self, value: &AttributeType, location: &str, function: Option<usize>) -> Option<String> {
        if !matches!(value, AttributeType::Empty(None)) {
            self.errors.push(format!(
                "{location}: bq-setup does not take a value; write a bare bq-setup inside the #[component] function whose setup it attaches"
            ));
            return None;
        }

        let function = function.map(|function| &self.index.functions[function]);
        if let Some(Function { setup: Some(name), .. }) = function {
            return Some(name.clone());
        }
        let place = match function {
            Some(function) if !function.component => format!("in `{}`, which is not a #[component]", function.name),
            Some(function) => format!("in `{}`, which bq_setups did not recognise as having a setup", function.name),
            None => "outside a #[component] function".to_string(),
        };
        self.errors.push(format!(
            "{location}: bare bq-setup {place}; bare bq-setup can only be used directly inside a #[component] function, where it attaches that component's setup"
        ));
        None
    }

    fn ref_names(&mut self, value: &AttributeType, location: &str) -> Vec<RefName> {
        let AttributeType::Normal {
            value: Markup::Lit(lit), ..
        } = value
        else {
            self.errors
                .push(format!("{location}: bq-ref needs a fixed string, for example bq-ref=\"select\""));
            return Vec::new();
        };

        let value = lit.lit.value();
        if value.split_whitespace().next().is_none() {
            self.errors
                .push(format!("{location}: bq-ref needs a name, for example bq-ref=\"select\""));
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
            names.push(RefName {
                setup: setup.map(str::to_string),
                name: name.to_lower_camel_case(),
                text: text.to_string(),
            });
        }
        names
    }
}

#[derive(Default)]
struct MacroFinder {
    blocks: Vec<(TokenStream, bool, bool)>,
    flags: Flags,
}

impl<'ast> Visit<'ast> for MacroFinder {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let mut found = Vec::new();
        if mac.path.segments.last().is_some_and(|segment| segment.ident == "html") {
            found.push(mac.tokens.clone());
        } else {
            html_blocks(mac.tokens.clone(), &mut found);
        }
        let Flags { optional, many } = self.flags;
        self.blocks.extend(found.into_iter().map(|tokens| (tokens, optional, many)));
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        let saved = self.flags;
        self.flags.enter(expr);
        syn::visit::visit_expr(self, expr);
        self.flags = saved;
    }
}

#[derive(Clone, Copy, Default)]
pub struct Flags {
    pub optional: bool,
    pub many: bool,
}

impl Flags {
    pub fn enter(&mut self, expr: &Expr) {
        match expr {
            Expr::Closure(_) | Expr::ForLoop(_) | Expr::While(_) | Expr::Loop(_) => self.many = true,
            Expr::If(_) | Expr::Match(_) => self.optional = true,
            _ => {}
        }
    }
}

fn spliced_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(path) if path.qself.is_none() => path.path.get_ident().map(ToString::to_string),
        Expr::Reference(reference) => spliced_name(&reference.expr),
        Expr::Paren(paren) => spliced_name(&paren.expr),
        _ => None,
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
            Expr::If(if_) => tail(&if_.then_branch, out) && if_.else_branch.as_ref().is_some_and(|(_, else_)| collect(else_, out)),
            Expr::Match(match_) => match_.arms.iter().all(|arm| collect(&arm.body, out)),
            _ => false,
        }
    }

    fn tail(block: &syn::Block, out: &mut BTreeSet<String>) -> bool {
        matches!(block.stmts.as_slice(), [Stmt::Expr(expr, None)] if collect(expr, out))
    }

    let mut tags = BTreeSet::new();
    if collect(expr, &mut tags) && !tags.is_empty() {
        Tags::Known(tags)
    } else {
        Tags::Unknown
    }
}

fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    items.into_iter().filter(|item| seen.insert(item.clone())).collect()
}
