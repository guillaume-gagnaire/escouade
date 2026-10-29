//! `escouade-plugin`: the command of the plugins' authors (PLG-3, ESC-175). `test <folder>` plays
//! the conformance bench, `validate <folder>` reads the manifest, `dev <folder>` is the fake
//! Escouade, which sends the plugin facts and decisions and shows what it answers, without the app.
//! `escouade-plugin --help` says the rest.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::plugin_cli(args));
}
