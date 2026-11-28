//! The probe of an agent's page in a real webview of this system (ESC-667): the trap of
//! docs/AGENT-PAGES.md § 8.4 tries every way out. `src/agentpage_probe.rs` says how.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::agentpage_probe_cli(args));
}
