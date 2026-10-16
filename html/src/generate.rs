use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Expr, Local, Path, ext::IdentExt, parse_quote, punctuated::Punctuated, spanned::Spanned, token::Brace, token::Comma};

use crate::{
    ast::*,
    conventions::{ATTRS, CHILDREN, FINISH_FN, maybe_setter, merges},
    escape,
};

pub fn generate(markups: Markups<Element>, output_ident: Ident, crate_path: TokenStream) -> TokenStream {
    let mut build = Builder::new(output_ident.clone());
    Generator::new(output_ident, crate_path).markups(markups, &mut build);
    build.finish()
}

struct Generator {
    output_ident: Ident,
    crate_path: TokenStream,
}

impl Generator {
    fn new(output_ident: Ident, crate_path: TokenStream) -> Generator {
        Generator { output_ident, crate_path }
    }

    fn builder(&self) -> Builder {
        Builder::new(self.output_ident.clone())
    }

    fn markups<E: Into<Element>>(&self, markups: Markups<E>, build: &mut Builder) {
        for markup in markups.markups {
            self.markup(markup, build);
        }
    }

    fn markup<E: Into<Element>>(&self, markup: Markup<E>, build: &mut Builder) {
        match markup {
            Markup::Block(block) => {
                if block.markups.markups.iter().any(|markup| {
                    matches!(
                        *markup,
                        Markup::ControlFlow(ControlFlow {
                            kind: ControlFlowKind::Let(_),
                            ..
                        })
                    )
                }) {
                    self.block(block, build);
                } else {
                    self.markups(block.markups, build);
                }
            }
            Markup::Lit(lit) => build.push_escaped(&lit.to_string()),
            Markup::Splice { expr, .. } => self.splice(expr, build),
            Markup::Element(element) => self.element(element.into(), build),
            Markup::ControlFlow(control_flow) => self.control_flow(control_flow, build),
            Markup::Semi(_) => {}
        }
    }

    fn block<E: Into<Element>>(&self, block: Block<E>, build: &mut Builder) {
        let markups = {
            let mut build = self.builder();
            self.markups(block.markups, &mut build);
            build.finish()
        };

        build.push_tokens(quote!({ #markups }));
    }

    fn splice(&self, expr: Expr, build: &mut Builder) {
        let output_ident = &self.output_ident;
        let crate_path = &self.crate_path;
        build.push_tokens(quote!(#crate_path::maud::macro_private::render_to!(&(#expr), &mut #output_ident);));
    }

    fn element(&self, element: Element, build: &mut Builder) {
        let element_name = match element.name {
            Some(ElementName::Static(name)) => name,
            Some(ElementName::Dynamic { expr, .. }) => {
                return self.dynamic_element(expr, element.attrs, element.body, build);
            }
            Some(ElementName::Component { path, args, .. }) => {
                return self.component(path, args, element.attrs, element.body, build);
            }
            None => parse_quote!(div),
        };
        build.push_str("<");
        self.name(element_name.clone(), build);
        self.attrs(element.attrs, build);
        build.push_str(">");
        if let ElementBody::Block(block) = element.body {
            self.markups(block.markups, build);
            build.push_str("</");
            self.name(element_name, build);
            build.push_str(">");
        }
    }

    fn dynamic_element(&self, expr: Expr, attrs: Vec<Attribute>, body: ElementBody, build: &mut Builder) {
        let output_ident = &self.output_ident;
        let tag = Ident::new("__bq_tag", Span::mixed_site());
        let push_tag = quote!(#output_ident.push_str(#tag););

        let mut inner = self.builder();
        inner.push_str("<");
        inner.push_tokens(push_tag.clone());
        self.attrs(attrs, &mut inner);
        inner.push_str(">");
        if let ElementBody::Block(block) = body {
            self.markups(block.markups, &mut inner);
            inner.push_str("</");
            inner.push_tokens(push_tag);
            inner.push_str(">");
        }
        let inner = inner.finish();
        let crate_path = &self.crate_path;

        build.push_tokens(quote! {{
            let #tag: &'static str = #crate_path::__private::tag(#expr);
            #inner
        }});
    }

    fn component(&self, path: Path, args: Punctuated<ComponentArg, Comma>, attrs: Vec<Attribute>, body: ElementBody, build: &mut Builder) {
        let output_ident = &self.output_ident;
        let crate_path = &self.crate_path;
        let span = path.span();
        let function = component_function(&path);

        let mut positional = Vec::new();
        let mut named = Vec::new();
        for arg in args {
            match arg {
                ComponentArg::Positional(expr) => positional.push(expr),
                ComponentArg::Named {
                    name, maybe: None, value, ..
                } => named.push(quote!(.#name(#value))),
                ComponentArg::Named {
                    name,
                    maybe: Some(_),
                    value,
                    ..
                } => {
                    let setter = Ident::new(&maybe_setter(&name.unraw().to_string()), name.span());
                    named.push(quote!(.#setter(#value)));
                }
            }
        }

        let attrs_ident = Ident::new("__bq_attrs", Span::mixed_site());
        let (attrs_stmts, attrs_setter) = if attrs.is_empty() {
            (TokenStream::new(), TokenStream::new())
        } else {
            let stmts = self.build_attrs(&attrs_ident, attrs);
            let attrs_method = Ident::new(ATTRS, span);
            (stmts, quote!(.#attrs_method(#attrs_ident)))
        };

        let children_setter = match body {
            ElementBody::Void(_) => TokenStream::new(),
            ElementBody::Block(block) => {
                let children = Ident::new("__bq_children", Span::mixed_site());
                let mut inner = Builder::new(children.clone());
                Generator::new(children.clone(), crate_path.clone()).markups(block.markups, &mut inner);
                let inner = inner.finish();
                let children_method = Ident::new(CHILDREN, span);
                quote! {
                    .#children_method({
                        let mut #children = alloc::string::String::new();
                        #inner
                        #crate_path::maud::PreEscaped(#children)
                    })
                }
            }
        };

        let build_method = Ident::new(FINISH_FN, span);
        build.push_tokens(quote! {{
            #attrs_stmts
            #crate_path::maud::macro_private::render_to!(
                &(#function(#(#positional),*) #(#named)* #attrs_setter #children_setter .#build_method()),
                &mut #output_ident
            );
        }});
    }

    fn build_attrs(&self, attrs_ident: &Ident, attrs: Vec<Attribute>) -> TokenStream {
        let crate_path = &self.crate_path;
        let (classes, id, named_attrs, spreads) = split_attrs(attrs);
        let mut stmts = vec![quote!(let mut #attrs_ident = #crate_path::Attrs::new();)];
        let mut skip = Vec::new();
        let private = quote!(#crate_path::__private);
        let text =
            |name: &str, value: TokenStream| quote!(#private::text(&mut #attrs_ident, #name, #crate_path::maud::PreEscaped(#value)););
        let class = |value: TokenStream| quote!(#private::class(&mut #attrs_ident, #crate_path::maud::PreEscaped(#value)););
        let flag = |name: &str| quote!(#private::flag(&mut #attrs_ident, #name););

        if let Some(name) = id {
            skip.push("id".to_string());
            stmts.push(match name {
                HtmlNameOrMarkup::HtmlName(name) => {
                    let escaped = escaped(&name.to_string());
                    text("id", quote!(alloc::string::String::from(#escaped)))
                }
                HtmlNameOrMarkup::Markup(markup) => rendered(
                    crate_path,
                    |generator, build| generator.markup(markup, build),
                    |part| text("id", quote!(#part)),
                ),
            });
        }

        for (name, toggler) in classes {
            let push = match name {
                HtmlNameOrMarkup::HtmlName(name) => {
                    let escaped = escaped(&name.to_string());
                    class(quote!(#escaped))
                }
                HtmlNameOrMarkup::Markup(markup) => rendered(
                    crate_path,
                    |generator, build| generator.markup(markup, build),
                    |part| class(quote!(&#part)),
                ),
            };
            stmts.push(match toggler {
                Some(cond) => quote!(if (#cond) { #push }),
                None => push,
            });
        }

        for (name, attr_type) in named_attrs {
            let name = name.to_string();
            if name != "class" && !merges(&name) {
                skip.push(name.clone());
            }
            stmts.push(match attr_type {
                AttributeType::Normal { value, .. } => rendered(
                    crate_path,
                    |generator, build| generator.markup(value, build),
                    |part| text(&name, quote!(#part)),
                ),
                AttributeType::Optional {
                    toggler: Toggler { cond, .. },
                    ..
                } => {
                    let inner_value: Expr = parse_quote!(inner_value);
                    let push = rendered(
                        crate_path,
                        |generator, build| generator.splice(inner_value.clone(), build),
                        |part| text(&name, quote!(#part)),
                    );
                    quote!(if let Some(#inner_value) = (#cond) { #push })
                }
                AttributeType::Empty(None) => flag(&name),
                AttributeType::Empty(Some(Toggler { cond, .. })) => {
                    let flag = flag(&name);
                    quote!(if (#cond) { #flag })
                }
            });
        }

        for expr in spreads {
            stmts.push(quote!(#private::extend(&mut #attrs_ident, &(#expr), &[#(#skip),*]);));
        }

        quote!(#(#stmts)*)
    }

    fn name(&self, name: HtmlName, build: &mut Builder) {
        build.push_escaped(&name.to_string());
    }

    fn name_or_markup(&self, name: HtmlNameOrMarkup, build: &mut Builder) {
        match name {
            HtmlNameOrMarkup::HtmlName(name) => self.name(name, build),
            HtmlNameOrMarkup::Markup(markup) => self.markup(markup, build),
        }
    }

    fn attr(&self, name: HtmlName, value: AttributeType, build: &mut Builder) {
        match value {
            AttributeType::Normal { value, .. } => {
                build.push_str(" ");
                self.name(name, build);
                build.push_str("=\"");
                self.markup(value, build);
                build.push_str("\"");
            }
            AttributeType::Optional {
                toggler: Toggler { cond, .. },
                ..
            } => {
                let inner_value: Expr = parse_quote!(inner_value);

                let body = {
                    let mut build = self.builder();
                    build.push_str(" ");
                    self.name(name, &mut build);
                    build.push_str("=\"");
                    self.splice(inner_value.clone(), &mut build);
                    build.push_str("\"");
                    build.finish()
                };
                build.push_tokens(quote!(if let Some(#inner_value) = (#cond) { #body }));
            }
            AttributeType::Empty(None) => {
                build.push_str(" ");
                self.name(name, build);
            }
            AttributeType::Empty(Some(Toggler { cond, .. })) => {
                let body = {
                    let mut build = self.builder();
                    build.push_str(" ");
                    self.name(name, &mut build);
                    build.finish()
                };
                build.push_tokens(quote!(if (#cond) { #body }));
            }
        }
    }

    fn attrs(&self, attrs: Vec<Attribute>, build: &mut Builder) {
        if needs_attrs(&attrs) {
            let output_ident = &self.output_ident;
            let crate_path = &self.crate_path;
            let attrs_ident = Ident::new("__bq_attrs", Span::mixed_site());
            let stmts = self.build_attrs(&attrs_ident, attrs);
            build.push_tokens(quote!({
                #stmts
                #crate_path::__private::write(&#attrs_ident, &mut #output_ident);
            }));
            return;
        }

        let (classes, id, named_attrs, _) = split_attrs(attrs);

        if !classes.is_empty() {
            let mut toggle_class_exprs = vec![];

            build.push_str(" ");
            self.name(parse_quote!(class), build);
            build.push_str("=\"");
            for (i, (name, toggler)) in classes.into_iter().enumerate() {
                if let Some(toggler) = toggler {
                    toggle_class_exprs.push((i > 0, name, toggler));
                } else {
                    if i > 0 {
                        build.push_str(" ");
                    }
                    self.name_or_markup(name, build);
                }
            }

            for (not_first, name, toggler) in toggle_class_exprs {
                let body = {
                    let mut build = self.builder();
                    if not_first {
                        build.push_str(" ");
                    }
                    self.name_or_markup(name, &mut build);
                    build.finish()
                };
                build.push_tokens(quote!(if (#toggler) { #body }));
            }

            build.push_str("\"");
        }

        if let Some(id) = id {
            build.push_str(" ");
            self.name(parse_quote!(id), build);
            build.push_str("=\"");
            self.name_or_markup(id, build);
            build.push_str("\"");
        }

        for (name, attr_type) in named_attrs {
            self.attr(name, attr_type, build);
        }
    }

    fn control_flow<E: Into<Element>>(&self, control_flow: ControlFlow<E>, build: &mut Builder) {
        match control_flow.kind {
            ControlFlowKind::If(if_) => self.control_flow_if(if_, build),
            ControlFlowKind::Let(let_) => self.control_flow_let(let_, build),
            ControlFlowKind::For(for_) => self.control_flow_for(for_, build),
            ControlFlowKind::While(while_) => self.control_flow_while(while_, build),
            ControlFlowKind::Match(match_) => self.control_flow_match(match_, build),
        }
    }

    fn control_flow_if<E: Into<Element>>(
        &self,
        IfExpr {
            if_token,
            cond,
            then_branch,
            else_branch,
        }: IfExpr<E>,
        build: &mut Builder,
    ) {
        build.push_tokens(quote!(#if_token #cond));
        self.block(then_branch, build);

        if let Some((_, else_token, if_or_block)) = else_branch {
            build.push_tokens(quote!(#else_token));
            self.control_flow_if_or_block(*if_or_block, build);
        }
    }

    fn control_flow_if_or_block<E: Into<Element>>(&self, if_or_block: IfOrBlock<E>, build: &mut Builder) {
        match if_or_block {
            IfOrBlock::If(if_) => self.control_flow_if(if_, build),
            IfOrBlock::Block(block) => self.block(block, build),
        }
    }

    fn control_flow_let(&self, let_: Local, build: &mut Builder) {
        build.push_tokens(let_.to_token_stream());
    }

    fn control_flow_for<E: Into<Element>>(
        &self,
        ForExpr {
            for_token,
            pat,
            in_token,
            expr,
            body,
        }: ForExpr<E>,
        build: &mut Builder,
    ) {
        build.push_tokens(quote!(#for_token #pat #in_token (#expr)));
        self.block(body, build);
    }

    fn control_flow_while<E: Into<Element>>(&self, WhileExpr { while_token, cond, body }: WhileExpr<E>, build: &mut Builder) {
        build.push_tokens(quote!(#while_token #cond));
        self.block(body, build);
    }

    fn control_flow_match<E: Into<Element>>(
        &self,
        MatchExpr {
            match_token,
            expr,
            brace_token,
            arms,
        }: MatchExpr<E>,
        build: &mut Builder,
    ) {
        let arms = {
            let mut build = self.builder();
            for MatchArm {
                pat,
                guard,
                fat_arrow_token,
                body,
                comma_token,
            } in arms
            {
                build.push_tokens(quote!(#pat));
                if let Some((if_token, cond)) = guard {
                    build.push_tokens(quote!(#if_token #cond));
                }
                build.push_tokens(quote!(#fat_arrow_token));
                self.block(
                    Block {
                        brace_token: Brace(Span::call_site()),
                        markups: Markups { markups: vec![body] },
                    },
                    &mut build,
                );
                build.push_tokens(quote!(#comma_token));
            }
            build.finish()
        };

        let mut arm_block = TokenStream::new();

        brace_token.surround(&mut arm_block, |tokens| {
            arms.to_tokens(tokens);
        });

        build.push_tokens(quote!(#match_token #expr #arm_block));
    }
}

////////////////////////////////////////////////////////

fn needs_attrs(attrs: &[Attribute]) -> bool {
    let mut names = Vec::new();
    let mut dotted = false;
    for attr in attrs {
        match attr {
            Attribute::Spread { .. } => return true,
            Attribute::Class { .. } => dotted = true,
            Attribute::Id { .. } => names.push("id".to_string()),
            Attribute::Named { name, .. } => names.push(name.to_string()),
        }
    }
    let repeated = names.iter().enumerate().any(|(index, name)| names[..index].contains(name));
    repeated || (dotted && names.iter().any(|name| name == "class"))
}

fn escaped(text: &str) -> String {
    let mut escaped = String::new();
    escape::escape_to_string(text, &mut escaped);
    escaped
}

fn rendered(
    crate_path: &TokenStream,
    write: impl FnOnce(&Generator, &mut Builder),
    then: impl FnOnce(&Ident) -> TokenStream,
) -> TokenStream {
    let part = Ident::new("__bq_part", Span::mixed_site());
    let mut build = Builder::new(part.clone());
    write(&Generator::new(part.clone(), crate_path.clone()), &mut build);
    let body = build.finish();
    let then = then(&part);
    quote! {{
        let mut #part = alloc::string::String::new();
        #body
        #then
    }}
}

fn component_function(path: &Path) -> Path {
    let mut path = path.clone();
    if let Some(last) = path.segments.last_mut() {
        let name = last.ident.unraw().to_string();
        last.ident = Ident::new(&name, Span::mixed_site().located_at(last.ident.span()));
    }
    path
}

#[allow(clippy::type_complexity)]
fn split_attrs(
    attrs: Vec<Attribute>,
) -> (
    Vec<(HtmlNameOrMarkup, Option<Expr>)>,
    Option<HtmlNameOrMarkup>,
    Vec<(HtmlName, AttributeType)>,
    Vec<Expr>,
) {
    let mut classes = vec![];
    let mut id = None;
    let mut named_attrs = vec![];
    let mut spreads = vec![];

    for attr in attrs {
        match attr {
            Attribute::Class { name, toggler, .. } => classes.push((name, toggler.map(|toggler| toggler.cond))),
            Attribute::Id { name, .. } => id = Some(name),
            Attribute::Named { name, attr_type } => named_attrs.push((name, attr_type)),
            Attribute::Spread { expr, .. } => spreads.push(expr),
        }
    }

    (classes, id, named_attrs, spreads)
}

////////////////////////////////////////////////////////

struct Builder {
    output_ident: Ident,
    tokens: TokenStream,
    tail: String,
}

impl Builder {
    fn new(output_ident: Ident) -> Builder {
        Builder {
            output_ident,
            tokens: TokenStream::new(),
            tail: String::new(),
        }
    }

    fn push_str(&mut self, string: &'static str) {
        self.tail.push_str(string);
    }

    fn push_escaped(&mut self, string: &str) {
        escape::escape_to_string(string, &mut self.tail);
    }

    fn push_tokens(&mut self, tokens: TokenStream) {
        self.cut();
        self.tokens.extend(tokens);
    }

    fn cut(&mut self) {
        if self.tail.is_empty() {
            return;
        }
        let push_str_expr = {
            let output_ident = self.output_ident.clone();
            let tail = &self.tail;
            quote!(#output_ident.push_str(#tail);)
        };
        self.tail.clear();
        self.tokens.extend(push_str_expr);
    }

    fn finish(mut self) -> TokenStream {
        self.cut();
        self.tokens
    }
}
