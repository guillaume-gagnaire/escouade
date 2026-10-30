//! The bench of the real agents (ESC-299): Claude Code with no window plays the tasks of
//! `docs/codeindex-agents.json` without the index, with it and its hook, with it alone.
//! `docs/CODEINDEX-PROBES.md` « Avec de vrais agents » says what it measures and what it found.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::codeindex_agents_cli(args));
}
