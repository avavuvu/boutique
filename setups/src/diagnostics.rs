use std::{collections::BTreeMap, fmt, sync::Arc};

use proc_macro2::Span;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loc {
    pub file: Arc<str>,
    pub line: usize,
    pub column: usize,
}

impl Loc {
    pub fn new(file: &Arc<str>, span: Span) -> Self {
        let start = span.start();
        Self {
            file: file.clone(),
            line: start.line,
            column: start.column,
        }
    }
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    pub loc: Loc,
    pub message: String,
    pub via: Option<Loc>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.loc, self.message)?;
        if let Some(via) = &self.via {
            write!(f, " (reached through {via})")?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Sink {
    entries: BTreeMap<(Level, Loc, String), Option<Loc>>,
}

impl Sink {
    pub fn error(&mut self, loc: &Loc, message: impl Into<String>, via: Option<&Loc>) {
        self.push(Level::Error, loc, message.into(), via);
    }

    pub fn warning(&mut self, loc: &Loc, message: impl Into<String>, via: Option<&Loc>) {
        self.push(Level::Warning, loc, message.into(), via);
    }

    fn push(&mut self, level: Level, loc: &Loc, message: String, via: Option<&Loc>) {
        self.entries.entry((level, loc.clone(), message)).or_insert_with(|| via.cloned());
    }

    pub fn has_errors(&self) -> bool {
        self.entries.keys().any(|(level, ..)| *level == Level::Error)
    }

    pub fn finish(self) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
        self.entries
            .into_iter()
            .map(|((level, loc, message), via)| Diagnostic { level, loc, message, via })
            .partition(|diagnostic| diagnostic.level == Level::Error)
    }
}
