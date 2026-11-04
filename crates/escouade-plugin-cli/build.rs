/// escouade-js, the Javy plugin the `run.js` plugins are compiled with and run on (ESC-320), built by
/// scripts/escouade-js.sh (CI's `escouade-js` job, the store's image): embedded here, so that it ships with the app and
/// with `escouade-plugin`. Where it is not built, an empty file: nothing compiles a `run.js` plugin, and says why
/// (`wasm::escouade_js`).
fn main() {
    println!("cargo:rerun-if-env-changed=ESCOUADE_JS_WASM");
    let manifest =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("its folder"));
    let folder = manifest.join("../../src-tauri/escouade-js");
    let built = match std::env::var_os("ESCOUADE_JS_WASM") {
        Some(p) => std::path::PathBuf::from(p),
        None => folder.join("target/escouade-js.wasm"),
    };
    let bytes = std::fs::read(&built).unwrap_or_default();
    // The file when it is there; else its crate's folder, where the script puts it.
    if bytes.is_empty() {
        println!("cargo:rerun-if-changed={}", folder.display());
    } else {
        println!("cargo:rerun-if-changed={}", built.display());
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"))
        .join("escouade-js.wasm");
    if std::fs::read(&out).ok().as_deref() != Some(&bytes[..]) {
        std::fs::write(&out, &bytes).expect("escouade-js.wasm in OUT_DIR");
    }
}
