//! The least of the Javy runtime, to weigh it: loads a module, starts it, sends `initialize`.
fn main() {
    let path = std::env::args().nth(1).expect("a .wasm");
    let m = escouade_wasm_host::load(&std::fs::read(path).unwrap()).unwrap();
    let g = escouade_wasm_host::Grants {
        id: "x".into(),
        folder: std::env::temp_dir(),
        ..Default::default()
    };
    let mut p = escouade_wasm_host::Plugin::start(&m, g, Default::default()).unwrap();
    println!(
        "{:?}",
        p.request("initialize", serde_json::json!({ "protocolVersion": 1 }))
            .ok()
    );
}
