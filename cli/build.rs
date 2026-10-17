use std::process::Command;

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    println!("cargo::rerun-if-changed=template");
    println!("cargo::rerun-if-changed=../.git/HEAD");
    println!("cargo::rerun-if-changed=../.git/index");

    let rev = git(&["rev-parse", "HEAD"]).unwrap_or_default();
    let is_dirty = git(&["status", "--porcelain"]).is_some_and(|status| !status.is_empty());
    println!("cargo::rustc-env=BOUTIQUE_REV={rev}");
    println!("cargo::rustc-env=BOUTIQUE_DIRTY={is_dirty}");
}
