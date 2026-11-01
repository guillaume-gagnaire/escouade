fn main() {
    #[allow(unused_mut)] // only changed on Windows
    let mut attributes = tauri_build::Attributes::new();
    // Embed the Common-Controls v6 manifest in every binary, test harnesses included:
    // without it, test binaries that link the dialog/tray code fail to start on Windows
    // (STATUS_ENTRYPOINT_NOT_FOUND, TaskDialogIndirect).
    #[cfg(windows)]
    {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = std::env::current_dir()
            .expect("cwd")
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rustc-link-arg=/WX");
    }
    escouade_js();
    tauri_build::try_build(attributes).expect("tauri build script");
}

/// escouade-js, the Javy plugin the `run.js` plugins are compiled with and run on (ESC-320), built by
/// scripts/escouade-js.sh (CI's `escouade-js` job): embedded in the app, so that it ships with it. Where it is not
/// built, an empty file: the app runs no `run.js` plugin and says why (`plugins::wasm::escouade_js`).
fn escouade_js() {
    println!("cargo:rerun-if-env-changed=ESCOUADE_JS_WASM");
    let built = match std::env::var_os("ESCOUADE_JS_WASM") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir()
            .expect("cwd")
            .join("escouade-js/target/escouade-js.wasm"),
    };
    let bytes = std::fs::read(&built).unwrap_or_default();
    // The file when it is there; else its crate's folder, where the script puts it.
    if bytes.is_empty() {
        println!("cargo:rerun-if-changed=escouade-js");
    } else {
        println!("cargo:rerun-if-changed={}", built.display());
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"))
        .join("escouade-js.wasm");
    if std::fs::read(&out).ok().as_deref() != Some(&bytes[..]) {
        std::fs::write(&out, &bytes).expect("escouade-js.wasm in OUT_DIR");
    }
}
