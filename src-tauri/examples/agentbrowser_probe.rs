//! The probe of the read bridge of the browsers (ESC-668), in a real webview of this system: what
//! it reads, captures and keeps of a page that defends itself. `src/agentbrowser_probe.rs` says how.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::agentbrowser_probe_cli(args));
}
