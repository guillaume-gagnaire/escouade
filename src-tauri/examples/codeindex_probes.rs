//! The probes bench of the code index (ESC-294): the five tools against the grep an agent would
//! write, on this repository at a given commit, in a folder of its own. `docs/CODEINDEX-PROBES.md`
//! says what it measures and how; `cargo run --example codeindex_probes -- --help` its options.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::codeindex_probes_cli(args));
}
