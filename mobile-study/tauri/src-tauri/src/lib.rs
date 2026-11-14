//! La maquette Tauri 2 : le Rust commun appelé directement, sans pont à générer (ESC-445).
use escouade_mobile_core as core;

#[tauri::command]
fn core_version() -> String {
    core::core_version()
}

#[tauri::command]
fn seal(key: String, plain: String) -> Result<String, String> {
    core::seal(key, plain).map_err(|e| e.to_string())
}

#[tauri::command]
fn open(key: String, sealed: String) -> Result<String, String> {
    core::open(key, sealed).map_err(|e| e.to_string())
}

#[tauri::command]
fn bench_in_rust(n: u32, size: u32) -> u64 {
    core::bench_roundtrips(n, size)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_notifications::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init()).plugin(tauri_plugin_biometric::init());
    builder
        .invoke_handler(tauri::generate_handler![core_version, seal, open, bench_in_rust])
        .run(tauri::generate_context!())
        .expect("la maquette Tauri n'a pas démarré");
}
