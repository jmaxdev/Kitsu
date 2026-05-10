use std::fs;
use std::path::Path;

fn main() {
    let app_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "kitsu".to_string());
    let about =
        std::env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| "A modern VCS".to_string());

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir).join("Cargo.toml");
    let manifest_content = fs::read_to_string(manifest_path).expect("Failed to read Cargo.toml");
    let manifest: toml::Value =
        toml::from_str(&manifest_content).expect("Failed to parse Cargo.toml");

    let dir_name = manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("kitsu"))
        .and_then(|v| v.get("dir_name"))
        .and_then(|d| d.as_str())
        .unwrap_or(".kitsu");

    println!("cargo:rustc-env=APP_NAME={}", app_name);
    println!("cargo:rustc-env=DIR_NAME={}", dir_name);
    println!("cargo:rustc-env=ABOUT={}", about);

    println!("cargo:rerun-if-changed=Cargo.toml");
}
