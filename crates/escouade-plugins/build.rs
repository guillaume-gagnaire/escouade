//! The version of the app the host is part of (`ESCOUADE_APP_VERSION`): that of `src-tauri/Cargo.toml`, which a release
//! bumps, so that the host says the app's (the agent of its requests, the bench, the manifests of the workspace).

fn main() {
    let manifest = "../../src-tauri/Cargo.toml";
    println!("cargo:rerun-if-changed={manifest}");
    let text = std::fs::read_to_string(manifest).expect("src-tauri/Cargo.toml");
    let mut in_package = false;
    let version = text
        .lines()
        .find_map(|l| {
            let l = l.trim();
            if l.starts_with('[') {
                in_package = l == "[package]";
                return None;
            }
            let rest = l.strip_prefix("version")?.trim_start().strip_prefix('=')?;
            in_package.then(|| rest.trim().trim_matches('"').to_string())
        })
        .expect("the app's version in src-tauri/Cargo.toml");
    println!("cargo:rustc-env=ESCOUADE_APP_VERSION={version}");
}
