use std::{
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

const EXTENSIONS: [&str; 5] = ["rs", "ts", "css", "js", "json"];

fn main() {
    println!("cargo::rerun-if-changed=files");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest.join("files");
    let mut files = Vec::new();
    collect(&root, &mut files);

    let mut paths: Vec<String> = files.iter().map(|file| relative(&root, file)).collect();
    paths.sort();

    let mut out = String::from("pub static FILES: &[(&str, &str)] = &[\n");
    for path in &paths {
        let full = format!("/files/{path}");
        writeln!(
            out,
            "    ({path:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {full:?}))),"
        )
        .expect("write to a String");
    }
    out.push_str("];\n\n#[doc(hidden)]\n#[macro_export]\nmacro_rules! module {\n");
    for path in &paths {
        if let Some(module) = path.strip_suffix("/mod.rs").filter(|module| !module.contains('/')) {
            let file = root.join(path);
            let file = file.to_str().expect("UTF-8 path to the kit sources");
            writeln!(
                out,
                "    ({module}) => {{\n        #[path = {file:?}]\n        mod {module};\n    }};"
            )
            .expect("write to a String");
        }
    }
    out.push_str("}\n");

    let target = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("files.rs");
    fs::write(&target, out).unwrap_or_else(|error| panic!("write {}: {error}", target.display()));
}

fn relative(root: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(root).expect("file under files/");
    relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|error| panic!("read {}: {error}", dir.display())).path();
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| EXTENSIONS.iter().any(|known| extension == *known))
        {
            out.push(path);
        }
    }
}
