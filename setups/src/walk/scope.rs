use std::rc::Rc;

use crate::html::Flags;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Owner {
    pub usage: usize,
    pub optional: bool,
    pub many: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scope {
    levels: Vec<Vec<Owner>>,
}

impl Scope {
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    pub fn nearest(&self) -> &[Owner] {
        self.levels.first().map(Vec::as_slice).unwrap_or_default()
    }

    pub fn owners(&self) -> impl Iterator<Item = &Owner> {
        self.levels.iter().flatten()
    }

    pub fn owner(&self, usage: usize) -> Option<Owner> {
        self.owners().find(|owner| owner.usage == usage).copied()
    }

    pub fn flagged(self: &Rc<Self>, flags: Flags) -> Rc<Self> {
        let unchanged = self
            .owners()
            .all(|owner| (owner.optional || !flags.optional) && (owner.many || !flags.many));
        if unchanged {
            return self.clone();
        }
        let mut scope = Scope::clone(self);
        for owner in scope.levels.iter_mut().flatten() {
            owner.optional |= flags.optional;
            owner.many |= flags.many;
        }
        Rc::new(scope)
    }

    pub fn inside(self: &Rc<Self>, usages: &[usize]) -> Rc<Self> {
        if usages.is_empty() {
            return self.clone();
        }
        let mut scope = Scope::clone(self);
        let level = usages
            .iter()
            .map(|&usage| Owner {
                usage,
                optional: false,
                many: false,
            })
            .collect();
        scope.levels.insert(0, level);
        Rc::new(scope)
    }
}
