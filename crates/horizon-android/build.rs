use std::{path::PathBuf, process::Command};
fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let out = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&root).output().expect("git source revision");
    assert!(out.status.success(), "Engine build requires its exact source revision");
    let sha = String::from_utf8(out.stdout).unwrap();
    let sha = sha.trim();
    assert!(sha.len() == 40 && sha.bytes().all(|c| c.is_ascii_hexdigit()));
    println!("cargo:rustc-env=VIS_ENGINE_COMMIT={sha}");
    println!("cargo:rerun-if-changed={}", root.join(".git").display());
}
