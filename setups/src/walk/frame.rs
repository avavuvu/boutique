use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use crate::{
    diagnostics::Loc,
    model::{Handler, RefUse},
};

#[derive(Clone, Debug)]
pub struct Origin {
    pub function: Option<usize>,
    pub context: bool,
    pub via: Option<Loc>,
    pub unspliced: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OriginKey {
    function: Option<usize>,
    context: bool,
    unspliced: Option<usize>,
}

impl Origin {
    pub fn key(&self) -> OriginKey {
        OriginKey {
            function: self.function,
            context: self.context,
            unspliced: self.unspliced,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Outside {
    pub attrs: usize,
    pub origin: Origin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Flag {
    Attrs,
    Children,
    Arg(usize),
}

pub struct Frame<'f> {
    pub serial: usize,
    pub key: usize,
    pub outside: Rc<[Outside]>,
    pub args: &'f [(String, usize)],
    pub children: Option<usize>,
    pub caller: Option<&'f Frame<'f>>,
    pub caller_origin: Origin,
    attrs_used: Cell<bool>,
    children_used: Cell<bool>,
    args_used: Vec<Cell<bool>>,
}

impl<'f> Frame<'f> {
    pub fn new(
        serial: usize,
        key: usize,
        outside: Rc<[Outside]>,
        args: &'f [(String, usize)],
        children: Option<usize>,
        caller: Option<&'f Frame<'f>>,
        caller_origin: Origin,
    ) -> Self {
        Self {
            serial,
            key,
            outside,
            args,
            children,
            caller,
            caller_origin,
            attrs_used: Cell::new(false),
            children_used: Cell::new(false),
            args_used: args.iter().map(|_| Cell::new(false)).collect(),
        }
    }

    pub fn set(&self, flag: Flag) {
        match flag {
            Flag::Attrs => self.attrs_used.set(true),
            Flag::Children => self.children_used.set(true),
            Flag::Arg(arg) => self.args_used[arg].set(true),
        }
    }

    pub fn attrs_used(&self) -> bool {
        self.attrs_used.get()
    }

    pub fn children_used(&self) -> bool {
        self.children_used.get()
    }

    pub fn arg_used(&self, arg: usize) -> bool {
        self.args_used[arg].get()
    }

    pub fn chain(&self) -> Vec<usize> {
        let mut chain = vec![self.serial];
        let mut frame = self.caller;
        while let Some(next) = frame {
            chain.push(next.serial);
            frame = next.caller;
        }
        chain
    }

    pub fn ancestor(&self, depth: usize) -> Option<&Frame<'f>> {
        let mut frame = self;
        for _ in 0..depth {
            frame = frame.caller?;
        }
        Some(frame)
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct FrameKey {
    pub outside: Vec<(usize, OriginKey)>,
    pub args: Vec<(String, usize)>,
    pub children: Option<usize>,
    pub caller: Option<usize>,
    pub caller_origin: OriginKey,
}

#[derive(Default)]
pub struct Recorded {
    pub refs: BTreeMap<(usize, String), RefUse>,
    pub handlers: BTreeSet<(usize, String, Handler)>,
    pub flags: BTreeSet<(usize, Flag)>,
}

pub struct Recorder {
    pub start: usize,
    pub chain: Vec<usize>,
    pub recorded: Recorded,
}
