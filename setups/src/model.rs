use std::collections::{BTreeMap, BTreeSet};

use crate::{attrs::Target, diagnostics::Loc};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TagSet {
    known: BTreeSet<String>,
    any: bool,
}

impl TagSet {
    pub fn one(tag: impl Into<String>) -> Self {
        Self {
            known: BTreeSet::from([tag.into()]),
            any: false,
        }
    }

    pub fn any() -> Self {
        Self {
            known: BTreeSet::new(),
            any: true,
        }
    }

    pub fn known(tags: BTreeSet<String>) -> Self {
        if tags.is_empty() {
            Self::any()
        } else {
            Self { known: tags, any: false }
        }
    }

    pub fn extend(&mut self, other: &TagSet) {
        self.known.extend(other.known.iter().cloned());
        self.any |= other.any;
    }

    pub fn is_any(&self) -> bool {
        self.any
    }

    pub fn is_open(&self) -> bool {
        self.any || self.known.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.known.iter()
    }
}

pub struct Declaration {
    pub name: String,
    pub function: String,
    pub function_id: usize,
    pub eager: bool,
    pub import: String,
    pub loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handler {
    pub event: String,
    pub target: Target,
    pub tag: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RefUse {
    pub tags: TagSet,
    pub optional: bool,
    pub many: bool,
    pub repeated: bool,
}

impl RefUse {
    pub fn merge(&mut self, other: &RefUse) {
        self.tags.extend(&other.tags);
        self.optional |= other.optional;
        self.many |= other.many;
        self.repeated = true;
    }
}

pub struct Usage {
    pub name: String,
    pub tags: TagSet,
    pub refs: BTreeMap<String, RefUse>,
    pub handlers: BTreeMap<String, BTreeSet<Handler>>,
}

#[derive(Default)]
pub struct Setup {
    pub tags: TagSet,
    pub refs: BTreeMap<String, Ref>,
    pub handlers: BTreeMap<String, BTreeSet<Handler>>,
    usages: usize,
}

#[derive(Default)]
pub struct Ref {
    pub tags: TagSet,
    pub optional: bool,
    pub many: bool,
    usages: usize,
}

pub fn merge(usages: &[Usage], declarations: &BTreeMap<String, &Declaration>) -> BTreeMap<String, Setup> {
    let mut setups: BTreeMap<String, Setup> = BTreeMap::new();
    for usage in usages.iter().filter(|usage| declarations.contains_key(&usage.name)) {
        let setup = setups.entry(usage.name.clone()).or_default();
        setup.usages += 1;
        setup.tags.extend(&usage.tags);
        for (name, handlers) in &usage.handlers {
            setup.handlers.entry(name.clone()).or_default().extend(handlers.iter().cloned());
        }
        for (name, reference) in &usage.refs {
            let entry = setup.refs.entry(name.clone()).or_default();
            entry.tags.extend(&reference.tags);
            entry.optional |= reference.optional;
            entry.many |= reference.many || reference.repeated;
            entry.usages += 1;
        }
    }

    for setup in setups.values_mut() {
        for reference in setup.refs.values_mut() {
            if reference.usages < setup.usages {
                reference.optional = true;
            }
        }
    }
    setups
}
