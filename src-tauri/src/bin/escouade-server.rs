//! `escouade-server` (ESC-514, docs/REMOTE.md, « Le mode serveur »): Escouade's core without a window, its remote API
//! on 127.0.0.1, for a machine with no desktop. `escouade-server --help` says the rest. Linux's is built without the
//! desktop: `cargo build --bin escouade-server --no-default-features` (docs/CRATES.md, § 14).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The page of the browser's client (BC-1): named here alone, so that the desktop app never carries it.
    std::process::exit(escouade_lib::server_main_with(
        args,
        escouade_web::FILES,
        escouade_web::MARK,
    ));
}
