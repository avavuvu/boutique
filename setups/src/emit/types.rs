use std::{
    collections::BTreeMap,
    fmt::{self, Write},
};

use bq_html::conventions::ts_type_name;

use super::ts::{ts_str, union};
use crate::model::{Declaration, Handler, Setup};

pub fn types(runtime: &str, declarations: &[&Declaration], setups: &BTreeMap<String, Setup>) -> String {
    let mut out = String::new();
    write_types(&mut out, runtime, declarations, setups).expect("writing to a String cannot fail");
    out
}

fn write_types(out: &mut String, runtime: &str, declarations: &[&Declaration], setups: &BTreeMap<String, Setup>) -> fmt::Result {
    writeln!(out, "import type {{ Setup }} from {};\n", ts_str(runtime))?;

    let empty = Setup::default();
    for declaration in declarations {
        let name = ts_type_name(&declaration.name);
        let setup = setups.get(&declaration.name).unwrap_or(&empty);
        let tags = union(&setup.tags);

        if setup.refs.is_empty() && setup.handlers.is_empty() {
            writeln!(out, "export type {name} = Setup<{tags}>;")?;
            continue;
        }

        let refs = refs(setup);
        if setup.handlers.is_empty() {
            writeln!(out, "export type {name} = Setup<{tags}, {refs}>;")?;
            continue;
        }

        let handlers: Vec<String> = setup
            .handlers
            .iter()
            .map(|(handler, uses)| {
                let uses: Vec<String> = uses.iter().map(handler_use).collect();
                format!("{handler}: {}", uses.join(" | "))
            })
            .collect();
        writeln!(out, "export type {name} = Setup<{tags}, {refs}, {{ {} }}>;", handlers.join("; "))?;
    }
    Ok(())
}

fn refs(setup: &Setup) -> String {
    let refs: Vec<String> = setup
        .refs
        .iter()
        .map(|(key, reference)| {
            let tags = union(&reference.tags);
            if reference.many {
                format!("{key}: ({tags})[]")
            } else if reference.optional {
                format!("{key}?: {tags}")
            } else {
                format!("{key}: {tags}")
            }
        })
        .collect();
    if refs.is_empty() {
        "{}".to_string()
    } else {
        format!("{{ {} }}", refs.join("; "))
    }
}

fn handler_use(handler: &Handler) -> String {
    let tag = handler.tag.as_deref().map(ts_str).unwrap_or_else(|| "string".into());
    let target = handler
        .target
        .name()
        .map(|target| format!("; target: {}", ts_str(target)))
        .unwrap_or_default();
    format!("{{ event: {}; tag: {tag}{target} }}", ts_str(&handler.event))
}
