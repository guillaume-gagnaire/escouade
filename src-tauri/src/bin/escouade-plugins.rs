//! `escouade-plugins`: the one program of the plugins the app gives (docs/PLUGINS.md §4.3, INT-4). Each plugin of
//! Escouade's own hand is a process of its own, but they share this executable, started with the name of the plugin:
//! `escouade-plugins minimal`. One program costs its runtime once, where one a plugin would cost it a dozen times.
//!
//! It is shipped beside the app and signed with it; the app starts no other (`plugins::builtin`). It speaks the
//! protocol of `escouade-plugin-protocol` and nothing of the app's own; the ACP client (PLG-6) is the crate `escouade-acp`.
//!
//! An entry is a function from the name to a `fn() -> i32`: add the plugin's module and one line to `ENTRIES`. A
//! plugin that serves code hosts or Kanban sources is a crate of its own beside its manifest
//! (`plugins/<id-court>/`), on the kit `escouade-plugin-kit` (its loop, the service of `forge/*` and `tickets/*`).

#[path = "escouade-plugins/minimal.rs"]
mod minimal;
#[path = "escouade-plugins/minimal_harness.rs"]
mod minimal_harness;

/// An entry: the name a manifest's `run.args` gives, and the function that runs the plugin and returns its exit code.
type Entry = (&'static str, fn() -> i32);

/// The plugins of this program. `minimal-harness` is none: the harness of the demonstration plugin, which the core
/// launches from the plugin's description (INT-9).
const ENTRIES: &[Entry] = &[
    ("minimal", minimal::run),
    ("minimal-harness", minimal_harness::run),
];

fn main() {
    let name = std::env::args().nth(1).unwrap_or_default();
    match ENTRIES.iter().find(|(n, _)| *n == name) {
        Some((_, run)) => std::process::exit(run()),
        None => {
            let names: Vec<&str> = ENTRIES.iter().map(|(n, _)| *n).collect();
            eprintln!(
                "escouade-plugins <plugin>: the program of the plugins Escouade gives, started by Escouade. Plugins: {}",
                names.join(", ")
            );
            std::process::exit(2);
        }
    }
}
