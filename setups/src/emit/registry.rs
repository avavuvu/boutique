use std::{
    collections::BTreeMap,
    fmt::{self, Write},
};

use bq_html::conventions::{ts_export_name, ts_type_name};

use super::ts::{list, ts_str};
use crate::model::{Declaration, Setup};

pub fn registry(runtime: &str, declarations: &[&Declaration], setups: &BTreeMap<String, Setup>) -> String {
    let mut out = String::new();
    write_registry(&mut out, runtime, declarations, setups).expect("writing to a String cannot fail");
    out
}

fn write_registry(out: &mut String, runtime: &str, declarations: &[&Declaration], setups: &BTreeMap<String, Setup>) -> fmt::Result {
    writeln!(out, "import type {{ Registry }} from {};", ts_str(runtime))?;
    writeln!(out, "import type * as setups from {};", ts_str("./setups"))?;

    let mut eager: Vec<(&str, Vec<String>)> = Vec::new();
    for declaration in declarations.iter().filter(|declaration| declaration.eager) {
        let export = ts_export_name(&declaration.name);
        match eager.iter_mut().find(|(import, _)| *import == declaration.import) {
            Some((_, exports)) => exports.push(export),
            None => eager.push((&declaration.import, vec![export])),
        }
    }
    for (import, exports) in eager {
        writeln!(out, "import {{ {} }} from {};", exports.join(", "), ts_str(import))?;
    }

    out.push_str("\nexport const registry: Registry = {\n");
    let empty = Setup::default();
    for declaration in declarations {
        let name = ts_type_name(&declaration.name);
        let export = ts_export_name(&declaration.name);
        let setup = setups.get(&declaration.name).unwrap_or(&empty);
        let tags = if setup.tags.is_any() {
            String::new()
        } else {
            list(setup.tags.iter(), ", ")
        };
        let many = list(setup.refs.iter().filter(|(_, reference)| reference.many).map(|(key, _)| key), ", ");

        writeln!(out, "    {}: {{", ts_str(&declaration.name))?;
        writeln!(out, "        tags: [{tags}],")?;
        writeln!(out, "        many: [{many}],")?;
        if declaration.eager {
            writeln!(out, "        setup: {export} satisfies setups.{name},")?;
        } else {
            writeln!(
                out,
                "        load: () => import({}).then((module) => module.{export} satisfies setups.{name}),",
                ts_str(&declaration.import)
            )?;
        }
        out.push_str("    },\n");
    }
    out.push_str("};\n");
    Ok(())
}
