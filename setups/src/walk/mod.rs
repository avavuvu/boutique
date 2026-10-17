mod frame;
mod scope;

use std::{
    collections::{BTreeMap, HashMap, HashSet, btree_map::Entry},
    rc::Rc,
};

use frame::{Flag, Frame, FrameKey, Origin, OriginKey, Outside, Recorded, Recorder};
use scope::Scope;

use crate::{
    attrs::{OnAttr, RefAttr},
    diagnostics::{Loc, Sink},
    html::Flags,
    index::{Call, Callee, Element, Index, NO_ATTRS, Node, Splice},
    model::{Handler, RefUse, TagSet, Usage},
};

pub fn walk(index: &Index, sink: &mut Sink) -> Vec<Usage> {
    let mut walker = Walker {
        index,
        sink,
        usages: Vec::new(),
        stack: Vec::new(),
        bound: Vec::new(),
        in_context: HashSet::new(),
        deferred: Vec::new(),
        memo: HashMap::new(),
        recorders: Vec::new(),
        frame_keys: HashMap::new(),
        frames: 0,
    };
    let empty = Rc::new(Scope::default());
    for &block in &index.loose {
        let cx = Cx::root(&empty, None);
        walker.region(index.blocks[block].region, &cx);
    }
    for function in index.roots() {
        walker.body(function, &Cx::root(&empty, Some(function)));
    }
    walker.finish()
}

struct Walker<'a> {
    index: &'a Index,
    sink: &'a mut Sink,
    usages: Vec<Usage>,
    stack: Vec<usize>,
    bound: Vec<usize>,
    in_context: HashSet<usize>,
    deferred: Vec<Deferred>,
    memo: HashMap<WalkKey, Rc<Recorded>>,
    recorders: Vec<Recorder>,
    frame_keys: HashMap<FrameKey, usize>,
    frames: usize,
}

struct Deferred {
    function: Option<usize>,
    loc: Loc,
    message: String,
    binding: Option<String>,
}

#[derive(PartialEq, Eq, Hash)]
struct WalkKey {
    function: usize,
    scope: Rc<Scope>,
    frame: Option<usize>,
    origin: OriginKey,
    stack: Vec<usize>,
    bound: Vec<usize>,
}

#[derive(Clone)]
struct Cx<'f> {
    scope: Rc<Scope>,
    frame: Option<&'f Frame<'f>>,
    origin: Origin,
}

impl<'f> Cx<'f> {
    fn root(scope: &Rc<Scope>, function: Option<usize>) -> Self {
        Self {
            scope: scope.clone(),
            frame: None,
            origin: Origin {
                function,
                context: false,
                via: None,
                unspliced: None,
            },
        }
    }

    fn with_scope(&self, scope: Rc<Scope>) -> Self {
        Self { scope, ..self.clone() }
    }

    fn flagged(&self, flags: Flags) -> Self {
        self.with_scope(self.scope.flagged(flags))
    }

    fn caller(frame: &'f Frame<'f>, scope: &Rc<Scope>) -> Self {
        Self {
            scope: scope.clone(),
            frame: frame.caller,
            origin: frame.caller_origin.clone(),
        }
    }
}

impl<'a> Walker<'a> {
    fn finish(mut self) -> Vec<Usage> {
        for deferred in std::mem::take(&mut self.deferred) {
            if deferred.function.is_some_and(|function| self.in_context.contains(&function)) {
                continue;
            }
            match deferred.binding {
                Some(binding) => self.sink.warning(
                    &deferred.loc,
                    format!(
                        "{}; it is in `{binding}`, a let-bound html! that is not spliced into another html!, so bq_setups cannot tell which setup it belongs to",
                        deferred.message
                    ),
                    None,
                ),
                None => self.sink.error(&deferred.loc, deferred.message, None),
            }
        }
        self.usages
    }

    fn enter(&mut self, function: usize, cx: &Cx) {
        let mut stack = self.stack.clone();
        stack.sort_unstable();
        let mut bound = self.bound.clone();
        bound.sort_unstable();
        let key = WalkKey {
            function,
            scope: cx.scope.clone(),
            frame: cx.frame.map(|frame| frame.key),
            origin: cx.origin.key(),
            stack,
            bound,
        };
        if let Some(recorded) = self.memo.get(&key).cloned() {
            self.replay(&recorded, cx.frame);
            return;
        }

        self.recorders.push(Recorder {
            start: self.usages.len(),
            chain: cx.frame.map(Frame::chain).unwrap_or_default(),
            recorded: Recorded::default(),
        });
        self.stack.push(function);
        self.body(function, cx);
        self.stack.pop();
        let recorder = self.recorders.pop().expect("a recorder for every walk");
        self.memo.insert(key, Rc::new(recorder.recorded));
    }

    fn replay(&mut self, recorded: &Recorded, frame: Option<&Frame>) {
        for ((usage, name), reference) in &recorded.refs {
            self.add_ref(*usage, name, reference);
        }
        for (usage, name, handler) in &recorded.handlers {
            self.add_handler(*usage, name, handler);
        }
        for &(depth, flag) in &recorded.flags {
            if let Some(target) = frame.and_then(|frame| frame.ancestor(depth)) {
                self.set(target, flag);
            }
        }
    }

    fn set(&mut self, frame: &Frame, flag: Flag) {
        frame.set(flag);
        for recorder in &mut self.recorders {
            if let Some(depth) = recorder.chain.iter().position(|&serial| serial == frame.serial) {
                recorder.recorded.flags.insert((depth, flag));
            }
        }
    }

    fn add_ref(&mut self, usage: usize, name: &str, reference: &RefUse) {
        merge_ref(self.usages[usage].refs.entry(name.to_string()), reference);
        for recorder in self.recorders.iter_mut().rev() {
            if usage >= recorder.start {
                break;
            }
            merge_ref(recorder.recorded.refs.entry((usage, name.to_string())), reference);
        }
    }

    fn add_handler(&mut self, usage: usize, name: &str, handler: &Handler) {
        self.usages[usage]
            .handlers
            .entry(name.to_string())
            .or_default()
            .insert(handler.clone());
        for recorder in self.recorders.iter_mut().rev() {
            if usage >= recorder.start {
                break;
            }
            recorder.recorded.handlers.insert((usage, name.to_string(), handler.clone()));
        }
    }

    fn body(&mut self, function: usize, cx: &Cx) {
        let index = self.index;
        let blocks = &index.functions[function].blocks;
        for &block in blocks {
            let html = &index.blocks[block];
            if html.binding.is_none() {
                self.region(html.region, &cx.flagged(html.flags));
            }
        }
        for &block in blocks.iter().rev() {
            let html = &index.blocks[block];
            if html.binding.is_some() && !html.spliced {
                let origin = Origin {
                    unspliced: Some(block),
                    ..cx.origin.clone()
                };
                self.bound_block(block, &Cx { origin, ..cx.clone() });
            }
        }
    }

    fn bound_block(&mut self, block: usize, cx: &Cx) {
        if self.bound.contains(&block) {
            return;
        }
        self.bound.push(block);
        let html = &self.index.blocks[block];
        self.region(html.region, &cx.flagged(html.flags));
        self.bound.pop();
    }

    fn region(&mut self, region: usize, cx: &Cx) {
        let index = self.index;
        for node in &index.regions[region] {
            match node {
                Node::Element(element) => self.element(element, cx),
                Node::Splice(splice) => self.splice(splice, cx),
                Node::Branch { flags, regions } => {
                    let inner = cx.flagged(*flags);
                    for &region in regions {
                        self.region(region, &inner);
                    }
                }
                Node::Expr(expr) => self.expr(*expr, cx),
            }
        }
    }

    fn expr(&mut self, expr: usize, cx: &Cx) {
        let index = self.index;
        let expr = &index.exprs[expr];
        if let Some(block) = expr.binding {
            self.bound_block(block, cx);
            return;
        }
        for &(flags, region) in &expr.nested {
            self.region(region, &cx.flagged(flags));
        }
    }

    fn splice(&mut self, splice: &Splice, cx: &Cx) {
        if let Some(frame) = cx.frame
            && splice.children
        {
            self.set(frame, Flag::Children);
            if let Some(children) = frame.children {
                self.region(children, &Cx::caller(frame, &cx.scope));
            }
            return;
        }

        if let Some(frame) = cx.frame
            && let Some(name) = &splice.name
            && let Some(arg) = frame.args.iter().position(|(arg, _)| arg == name)
        {
            self.set(frame, Flag::Arg(arg));
            self.expr(frame.args[arg].1, &Cx::caller(frame, &cx.scope));
            return;
        }

        if let Some((callee, loc)) = &splice.callee
            && !cx.scope.is_empty()
        {
            if let Callee::Function(function) = callee
                && !self.stack.contains(function)
            {
                self.follow(*function, loc, cx);
            }
            if let Some(message) = callee.ambiguity() {
                self.sink.warning(loc, message, None);
            }
        }

        self.expr(splice.expr, cx);
    }

    fn follow(&mut self, function: usize, loc: &Loc, cx: &Cx) {
        self.in_context.insert(function);
        let inner = Cx {
            scope: cx.scope.clone(),
            frame: None,
            origin: Origin {
                function: Some(function),
                context: true,
                via: Some(loc.clone()),
                unspliced: None,
            },
        };
        self.enter(function, &inner);
    }

    fn element(&mut self, element: &'a Element, cx: &Cx) {
        if let Some(call) = &element.call {
            return self.component(element, call, cx);
        }
        let outside: Rc<[Outside]> = match cx.frame {
            Some(frame) if element.spread => {
                self.set(frame, Flag::Attrs);
                frame.outside.clone()
            }
            _ => Rc::from([]),
        };
        let inner = self.place(&element.tags, element.attrs, &cx.origin, &outside, cx);
        if let Some(body) = element.body {
            self.region(body, &cx.with_scope(inner));
        }
    }

    fn component(&mut self, element: &'a Element, call: &'a Call, cx: &Cx) {
        for &arg in &call.positional {
            self.expr(arg, cx);
        }
        let mut outside = Vec::new();
        if element.attrs != NO_ATTRS {
            outside.push(Outside {
                attrs: element.attrs,
                origin: cx.origin.clone(),
            });
        }
        if element.spread
            && let Some(frame) = cx.frame
        {
            self.set(frame, Flag::Attrs);
            outside.extend(frame.outside.iter().cloned());
        }
        let outside: Rc<[Outside]> = outside.into();

        if let Some(message) = call.callee.ambiguity() {
            self.sink.warning(&call.loc, message, None);
        }
        let function = match call.callee {
            Callee::Function(function) if !self.stack.contains(&function) => function,
            _ => {
                for &(_, value) in &call.named {
                    self.expr(value, cx);
                }
                let inner = self.place(&TagSet::any(), NO_ATTRS, &cx.origin, &outside, cx);
                if let Some(body) = element.body {
                    self.region(body, &cx.with_scope(inner));
                }
                return;
            }
        };

        self.in_context.insert(function);
        let key = self.frame_key(&outside, &call.named, element.body, cx);
        self.frames += 1;
        let frame = Frame::new(self.frames, key, outside, &call.named, element.body, cx.frame, cx.origin.clone());
        let body = Cx {
            scope: cx.scope.clone(),
            frame: Some(&frame),
            origin: Origin {
                function: Some(function),
                context: true,
                via: Some(call.loc.clone()),
                unspliced: None,
            },
        };
        self.enter(function, &body);

        for (arg, &(_, value)) in call.named.iter().enumerate() {
            if !frame.arg_used(arg) {
                self.expr(value, cx);
            }
        }
        let scope = if frame.attrs_used() {
            cx.scope.clone()
        } else {
            self.place(&TagSet::any(), NO_ATTRS, &cx.origin, &frame.outside, cx)
        };
        if !frame.children_used()
            && let Some(body) = element.body
        {
            self.region(body, &cx.with_scope(scope));
        }
    }

    fn frame_key(&mut self, outside: &[Outside], args: &[(String, usize)], children: Option<usize>, cx: &Cx) -> usize {
        let key = FrameKey {
            outside: outside.iter().map(|item| (item.attrs, item.origin.key())).collect(),
            args: args.to_vec(),
            children,
            caller: cx.frame.map(|frame| frame.key),
            caller_origin: cx.origin.key(),
        };
        let next = self.frame_keys.len();
        *self.frame_keys.entry(key).or_insert(next)
    }

    fn place(&mut self, tags: &TagSet, own: usize, origin: &Origin, outside: &[Outside], cx: &Cx) -> Rc<Scope> {
        let index = self.index;
        let own = &index.attrs[own];
        for reference in &own.refs {
            self.reference(reference, tags, &cx.scope, origin);
        }
        for item in outside {
            for reference in &index.attrs[item.attrs].refs {
                self.reference(reference, tags, &cx.scope, &item.origin);
            }
        }

        let mut created = Vec::new();
        let names = own
            .setups
            .iter()
            .chain(outside.iter().flat_map(|item| &index.attrs[item.attrs].setups));
        for name in names {
            created.push(self.usages.len());
            self.usages.push(Usage {
                name: name.clone(),
                tags: tags.clone(),
                refs: BTreeMap::new(),
                handlers: BTreeMap::new(),
            });
        }
        let inner = cx.scope.inside(&created);

        for on in &own.ons {
            self.handler(on, tags, &inner, origin);
        }
        for item in outside {
            for on in &index.attrs[item.attrs].ons {
                self.handler(on, tags, &inner, &item.origin);
            }
        }
        inner
    }

    fn orphan(&mut self, origin: &Origin, loc: &Loc, message: String) {
        if origin.context {
            self.sink.error(loc, message, origin.via.as_ref());
            return;
        }
        self.deferred.push(Deferred {
            function: origin.function,
            loc: loc.clone(),
            message,
            binding: origin.unspliced.and_then(|block| self.index.blocks[block].binding.clone()),
        });
    }

    fn owner(&mut self, setup: Option<&str>, scope: &Scope, text: &str, loc: &Loc, origin: &Origin) -> Option<usize> {
        if let Some(setup) = setup {
            let found = scope
                .owners()
                .find(|owner| self.usages[owner.usage].name == setup)
                .map(|owner| owner.usage);
            if found.is_none() {
                let message = match self.index.declarations.iter().find(|declaration| declaration.name == setup) {
                    Some(declaration) => format!(
                        "{text} is not inside the element with bare bq-setup in component `{}`",
                        declaration.function
                    ),
                    None => format!("{text} names the setup \"{setup}\", but no #[component] has that setup"),
                };
                self.orphan(origin, loc, message);
            }
            return found;
        }

        let nearest = scope.nearest();
        let mut names: Vec<String> = nearest.iter().map(|owner| self.usages[owner.usage].name.clone()).collect();
        names.dedup();
        match (nearest, names.as_slice()) {
            ([], _) => {
                self.orphan(origin, loc, format!("{text} is not inside an element with bq-setup"));
                None
            }
            ([owner, ..], [_]) => Some(owner.usage),
            _ => {
                let (name, value) = text.split_once('=').unwrap_or((text, "\"\""));
                let value = value.trim_matches('"');
                self.sink.error(
                    loc,
                    format!(
                        "{text} is inside more than one setup ({}); write the setup name, for example {name}=\"{}.{value}\"",
                        names.join(", "),
                        names[0]
                    ),
                    origin.via.as_ref(),
                );
                None
            }
        }
    }

    fn handler(&mut self, on: &OnAttr, tags: &TagSet, scope: &Scope, origin: &Origin) {
        let value = match &on.setup {
            Some(setup) => format!("{setup}.{}", on.handler),
            None => on.handler.clone(),
        };
        let text = format!("{}=\"{value}\"", on.attribute);
        let Some(owner) = self.owner(on.setup.as_deref(), scope, &text, &on.loc, origin) else {
            return;
        };
        let handler = |tag| Handler {
            event: on.event.clone(),
            target: on.target,
            tag,
        };
        if tags.is_any() {
            self.add_handler(owner, &on.key, &handler(None));
        } else {
            for tag in tags.iter() {
                self.add_handler(owner, &on.key, &handler(Some(tag.clone())));
            }
        }
    }

    fn reference(&mut self, reference: &RefAttr, tags: &TagSet, scope: &Scope, origin: &Origin) {
        let text = format!("bq-ref=\"{}\"", reference.text);
        let Some(owner) = self.owner(reference.setup.as_deref(), scope, &text, &reference.loc, origin) else {
            return;
        };
        let flags = scope.owner(owner);
        self.add_ref(
            owner,
            &reference.name,
            &RefUse {
                tags: tags.clone(),
                optional: flags.is_some_and(|flags| flags.optional),
                many: flags.is_some_and(|flags| flags.many),
                repeated: false,
            },
        );
    }
}

fn merge_ref<K: Ord>(entry: Entry<'_, K, RefUse>, reference: &RefUse) {
    match entry {
        Entry::Occupied(mut existing) => existing.get_mut().merge(reference),
        Entry::Vacant(vacant) => {
            vacant.insert(reference.clone());
        }
    }
}
