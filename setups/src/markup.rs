use std::collections::{HashMap, HashSet};

use heck::ToLowerCamelCase;
use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};

pub struct Usage {
    pub name: String,
    pub tag: String,
    pub refs: Vec<RefUse>,
    pub location: String,
}

pub struct RefUse {
    pub name: String,
    pub tag: String,
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

#[derive(Default)]
pub struct Walker {
    pub file: String,
    pub usages: Vec<Usage>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub functions: HashMap<String, Vec<TokenStream>>,
    pub function: Option<String>,
    stack: Vec<String>,
    followed: HashSet<String>,
    orphans: Vec<(Option<String>, String)>,
}

impl Walker {
    pub fn start_file(&mut self, file: String, functions: HashMap<String, Vec<TokenStream>>) {
        self.file = file;
        self.functions = functions;
        self.function = None;
        self.followed.clear();
        self.orphans.clear();
    }

    pub fn finish_file(&mut self) {
        for (function, message) in std::mem::take(&mut self.orphans) {
            let followed = function.as_ref().is_some_and(|name| self.followed.contains(name));
            if !followed {
                self.warnings.push(message);
            }
        }
    }
    pub fn html(&mut self, tokens: TokenStream) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        self.markups(&tokens, &Scope::default());
    }

    pub fn nested(&mut self, tokens: TokenStream) {
        self.nested_in(tokens, &Scope::default());
    }

    fn nested_in(&mut self, tokens: TokenStream, scope: &Scope) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        for (index, token) in tokens.iter().enumerate() {
            let TokenTree::Group(group) = token else { continue };
            let is_html = index >= 2 && is_ident(tokens.get(index - 2), "html") && is_punct(tokens.get(index - 1), '!');
            if is_html {
                let block: Vec<TokenTree> = group.stream().into_iter().collect();
                self.markups(&block, scope);
            } else {
                self.nested_in(group.stream(), scope);
            }
        }
    }

    fn location(&self, span: Span) -> String {
        format!("{}:{}", self.file, span.start().line)
    }

    fn markups(&mut self, tokens: &[TokenTree], scope: &Scope) {
        let mut index = 0;
        while index < tokens.len() {
            index = self.markup(tokens, index, scope);
        }
    }

    fn markup(&mut self, tokens: &[TokenTree], index: usize, scope: &Scope) -> usize {
        match &tokens[index] {
            TokenTree::Literal(_) => index + 1,
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                self.markups(&items(group), scope);
                index + 1
            }
            TokenTree::Group(group) => {
                self.splice(group.stream(), scope);
                index + 1
            }
            TokenTree::Punct(punct) if punct.as_char() == '@' => self.control(tokens, index + 1, scope),
            TokenTree::Punct(_) => index + 1,
            TokenTree::Ident(_) => self.element(tokens, index, scope),
        }
    }

    fn splice(&mut self, stream: TokenStream, scope: &Scope) {
        let tokens: Vec<TokenTree> = stream.clone().into_iter().collect();
        let called = match (tokens.first(), tokens.get(1)) {
            (Some(TokenTree::Ident(name)), Some(TokenTree::Group(arguments)))
                if arguments.delimiter() == Delimiter::Parenthesis =>
            {
                Some(name.to_string())
            }
            _ => None,
        };

        if let Some(name) = called.filter(|name| !scope.owners.is_empty() && !self.stack.contains(name)) {
            if let Some(blocks) = self.functions.get(&name).cloned() {
                self.followed.insert(name.clone());
                self.stack.push(name);
                for block in blocks {
                    let block: Vec<TokenTree> = block.into_iter().collect();
                    self.markups(&block, scope);
                }
                self.stack.pop();
            }
        }

        self.nested(stream);
    }

    fn control(&mut self, tokens: &[TokenTree], index: usize, scope: &Scope) -> usize {
        let Some(TokenTree::Ident(keyword)) = tokens.get(index) else { return index };

        match keyword.to_string().as_str() {
            "if" | "while" => {
                let branch = Scope { optional: true, ..scope.clone() };
                let mut next = self.block_after(tokens, index + 1, &branch);
                while is_punct(tokens.get(next), '@') && is_ident(tokens.get(next + 1), "else") {
                    let start = if is_ident(tokens.get(next + 2), "if") { next + 3 } else { next + 2 };
                    next = self.block_after(tokens, start, &branch);
                }
                next
            }
            "for" => {
                let each = Scope { many: true, ..scope.clone() };
                self.block_after(tokens, index + 1, &each)
            }
            "match" => {
                let arm = Scope { optional: true, ..scope.clone() };
                let mut next = index + 1;
                while next < tokens.len() {
                    if let TokenTree::Group(group) = &tokens[next] {
                        if group.delimiter() == Delimiter::Brace {
                            self.arms(&items(group), &arm);
                            return next + 1;
                        }
                    }
                    next += 1;
                }
                next
            }
            "let" => {
                let mut next = index + 1;
                while next < tokens.len() && !is_punct(tokens.get(next), ';') {
                    next += 1;
                }
                next + 1
            }
            _ => index + 1,
        }
    }

    fn block_after(&mut self, tokens: &[TokenTree], start: usize, scope: &Scope) -> usize {
        let mut index = start;
        while index < tokens.len() {
            if let TokenTree::Group(group) = &tokens[index] {
                if group.delimiter() == Delimiter::Brace {
                    self.markups(&items(group), scope);
                    return index + 1;
                }
            }
            index += 1;
        }
        index
    }

    fn arms(&mut self, tokens: &[TokenTree], scope: &Scope) {
        let mut index = 0;
        while index < tokens.len() {
            if is_punct(tokens.get(index), '=') && is_punct(tokens.get(index + 1), '>') {
                index += 2;
                if index < tokens.len() {
                    index = self.markup(tokens, index, scope);
                }
                if is_punct(tokens.get(index), ',') {
                    index += 1;
                }
            } else {
                index += 1;
            }
        }
    }

    fn element(&mut self, tokens: &[TokenTree], index: usize, scope: &Scope) -> usize {
        let location = self.location(tokens[index].span());
        let (tag, mut next) = name(tokens, index);
        let mut setups = Vec::new();
        let mut reference = None;
        let mut body = None;

        loop {
            match tokens.get(next) {
                None => break,
                Some(TokenTree::Punct(punct)) if punct.as_char() == ';' => {
                    next += 1;
                    break;
                }
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                    body = Some(items(group));
                    next += 1;
                    break;
                }
                Some(TokenTree::Punct(punct)) if punct.as_char() == '.' || punct.as_char() == '#' => {
                    next += 1;
                    match tokens.get(next) {
                        Some(TokenTree::Group(_)) => next += 1,
                        Some(_) => next = name(tokens, next).1,
                        None => {}
                    }
                    if is_group(tokens.get(next), Delimiter::Bracket) {
                        next += 1;
                    }
                }
                Some(TokenTree::Ident(_)) | Some(TokenTree::Literal(_)) => {
                    let (attribute, after) = name(tokens, next);
                    next = after;
                    if is_punct(tokens.get(next), '=') {
                        next += 1;
                        if let Some(value) = tokens.get(next) {
                            match attribute.as_str() {
                                "bq-setup" => setups = self.setup_names(value, &location),
                                "bq-ref" => reference = self.ref_name(value, &location),
                                _ => {}
                            }
                            next += 1;
                        }
                    } else if is_group(tokens.get(next), Delimiter::Bracket) {
                        next += 1;
                    }
                }
                Some(_) => next += 1,
            }
        }

        if let Some(reference) = reference {
            if scope.owners.is_empty() {
                let message = format!("{location}: bq-ref=\"{reference}\" is not inside an element with bq-setup");
                self.orphans.push((self.function.clone(), message));
            }
            for &owner in &scope.owners {
                self.usages[owner].refs.push(RefUse {
                    name: reference.clone(),
                    tag: tag.clone(),
                    optional: scope.optional,
                    many: scope.many,
                    location: location.clone(),
                });
            }
        }

        let inner = if setups.is_empty() {
            scope.clone()
        } else {
            let mut owners = Vec::new();
            for setup in setups {
                self.usages.push(Usage { name: setup, tag: tag.clone(), refs: Vec::new(), location: location.clone() });
                owners.push(self.usages.len() - 1);
            }
            Scope { owners, optional: false, many: false }
        };

        if let Some(body) = body {
            self.markups(&body, &inner);
        }

        next
    }

    fn setup_names(&mut self, value: &TokenTree, location: &str) -> Vec<String> {
        let message = || format!("{location}: bq-setup needs a name from setup!(..), for example bq-setup=(ThemeSelect)");

        match value {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => match path_name(group.stream()) {
                Some(name) => vec![name],
                None => {
                    self.errors.push(message());
                    Vec::new()
                }
            },
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                let mut names = Vec::new();
                for token in group.stream() {
                    match &token {
                        TokenTree::Group(inner) if inner.delimiter() == Delimiter::Parenthesis => match path_name(inner.stream()) {
                            Some(name) => names.push(name),
                            None => self.errors.push(message()),
                        },
                        TokenTree::Literal(literal) if string_value(&literal.to_string()).is_some_and(|text| text.trim().is_empty()) => {}
                        _ => self.errors.push(message()),
                    }
                }
                names
            }
            _ => {
                self.errors.push(message());
                Vec::new()
            }
        }
    }

    fn ref_name(&mut self, value: &TokenTree, location: &str) -> Option<String> {
        let text = match value {
            TokenTree::Literal(literal) => string_value(&literal.to_string()),
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

fn items(group: &Group) -> Vec<TokenTree> {
    group.stream().into_iter().collect()
}

fn is_punct(token: Option<&TokenTree>, character: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == character)
}

fn is_ident(token: Option<&TokenTree>, text: &str) -> bool {
    matches!(token, Some(TokenTree::Ident(ident)) if ident == text)
}

fn is_group(token: Option<&TokenTree>, delimiter: Delimiter) -> bool {
    matches!(token, Some(TokenTree::Group(group)) if group.delimiter() == delimiter)
}

fn fragment(token: &TokenTree) -> String {
    match token {
        TokenTree::Literal(literal) => {
            let text = literal.to_string();
            string_value(&text).unwrap_or(text)
        }
        other => other.to_string(),
    }
}

fn name(tokens: &[TokenTree], index: usize) -> (String, usize) {
    let mut text = fragment(&tokens[index]);
    let mut next = index + 1;
    while let Some(TokenTree::Punct(punct)) = tokens.get(next) {
        let character = punct.as_char();
        let joins = character == '-' || character == ':';
        if joins && matches!(tokens.get(next + 1), Some(TokenTree::Ident(_)) | Some(TokenTree::Literal(_))) {
            text.push(character);
            text.push_str(&fragment(&tokens[next + 1]));
            next += 2;
        } else {
            break;
        }
    }
    (text, next)
}

fn path_name(stream: TokenStream) -> Option<String> {
    let mut last = None;
    let mut expect_ident = true;
    let mut colons = 0;

    for token in stream {
        match token {
            TokenTree::Ident(ident) if expect_ident => {
                last = Some(ident.to_string());
                expect_ident = false;
            }
            TokenTree::Punct(punct) if punct.as_char() == ':' && !expect_ident => {
                colons += 1;
                if colons == 2 {
                    colons = 0;
                    expect_ident = true;
                }
            }
            _ => return None,
        }
    }

    if expect_ident { None } else { last }
}

fn string_value(text: &str) -> Option<String> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.to_string())
}
