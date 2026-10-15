use sha2::{Digest, Sha256};

fn main() {
    println!("cargo:rerun-if-changed=assets/htmx.js");
    let source = std::fs::read("assets/htmx.js").expect("failed to read assets/htmx.js");
    let hash: String = Sha256::digest(&source)[..8].iter().map(|byte| format!("{byte:02x}")).collect();
    println!("cargo:rustc-env=BOUTIQUE_HTMX_HASH={hash}");
}
