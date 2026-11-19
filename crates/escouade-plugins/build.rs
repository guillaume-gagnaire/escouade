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

    // wasmtime's exact version (`ESCOUADE_WASMTIME_VERSION`), pinned `=x.y.z` in this crate's Cargo.toml: the key of
    // the precompiled modules (`wasm::module_version`, ESC-542).
    let own = "Cargo.toml";
    println!("cargo:rerun-if-changed={own}");
    let text = std::fs::read_to_string(own).expect("crates/escouade-plugins/Cargo.toml");
    let wasmtime = text
        .lines()
        .find_map(|l| {
            let rest = l.trim().strip_prefix("wasmtime")?.trim_start();
            let pin = rest.strip_prefix('=')?.split('"').nth(1)?;
            Some(
                pin.strip_prefix('=')
                    .expect("wasmtime pinned exactly (`=x.y.z`)")
                    .to_string(),
            )
        })
        .expect("wasmtime's version in crates/escouade-plugins/Cargo.toml");
    println!("cargo:rustc-env=ESCOUADE_WASMTIME_VERSION={wasmtime}");
}
