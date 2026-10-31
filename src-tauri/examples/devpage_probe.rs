//! The probe of a launch's page in a real webview of this system (ESC-311): what the page could
//! reach of Escouade, its storage, its devtools and its widths. `src/devpage_probe.rs` says how.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::devpage_probe_cli(args));
}
