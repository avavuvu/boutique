use std::{
    env, fs,
    path::{Path, PathBuf},
};

const EXTENSIONS: [&str; 3] = ["rs", "ts", "css"];

fn main() {
    println!("cargo:rerun-if-changed=src");

    let src = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect(&src, &mut files);

    let mut paths: Vec<String> = files
        .iter()
        .map(|file| {
            let relative = file.strip_prefix(&src).expect("file under src");
            relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    paths.sort();

    let mut out = String::from("pub static FILES: &[(&str, &str)] = &[\n");
    for path in &paths {
        let full = format!("/src/{path}");
        out.push_str(&format!(
            "    ({path:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {full:?}))),\n"
        ));
    }
    out.push_str("];\n");

    let target = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("files.rs");
    fs::write(&target, out).unwrap_or_else(|error| panic!("write {}: {error}", target.display()));
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
