//! `escouade-plugins`: the one program of the plugins the app gives (docs/PLUGINS.md §4.3, INT-4). Each plugin of
//! Escouade's own hand is a process of its own, but they share this executable, started with the name of the plugin:
//! `escouade-plugins minimal`. One program costs its runtime once, where one a plugin would cost it a dozen times.
//!
//! It is shipped beside the app and signed with it; the app starts no other (`plugins::builtin`). It speaks the
//! protocol of `escouade-plugin-protocol` and nothing of the app's own, but the voice services (INT-13), which run the
//! code of the core's services through `escouade_lib`; the ACP client (PLG-6) is the crate `escouade-acp`.
//!
//! An entry is a function from the name to a `fn() -> i32`: add the plugin's module and one line to `ENTRIES`. A
//! plugin that serves code hosts or Kanban sources is a crate of its own beside its manifest
//! (`plugins/builtin/<id>/`), on the kit `escouade-plugin-kit` (its loop, the service of `forge/*` and `tickets/*`).

#[path = "escouade-plugins/isola.rs"]
mod isola;
#[path = "escouade-plugins/minimal.rs"]
mod minimal;
#[path = "escouade-plugins/minimal_harness.rs"]
mod minimal_harness;
#[path = "escouade-plugins/voice.rs"]
mod voice;

/// An entry: the name a manifest's `run.args` gives, and the function that runs the plugin and returns its exit code.
type Entry = (&'static str, fn() -> i32);

/// The plugins of this program. `minimal-harness` is none: the harness of the demonstration plugin, which the core
/// launches from the plugin's description (INT-9).
const ENTRIES: &[Entry] = &[
    ("isola", isola::run_plugin),
    ("minimal", minimal::run),
    ("minimal-harness", minimal_harness::run),
    ("voice-openai", voice::openai),
    ("voice-groq", voice::groq),
    ("voice-mistral", voice::mistral),
    ("voice-elevenlabs", voice::elevenlabs),
    ("voice-deepgram", voice::deepgram),
    // GitHub (INT-8.1): its code host and GitHub Issues, in its crate (plugins/builtin/dev.escouade.github).
    ("github", escouade_plugin_github::run),
    // GitLab (ESC-56): its code host, in its crate (plugins/builtin/dev.escouade.gitlab); GitLab Issues joins it (INT-8.2).
    ("gitlab", escouade_plugin_gitlab::run),
    // Bitbucket Cloud (INT-8.3, ESC-57): its code host, in its crate (plugins/builtin/dev.escouade.bitbucket).
    ("bitbucket", escouade_plugin_bitbucket::run),
    // Gitea and Forgejo (INT-8.4): its code host, in its crate (plugins/builtin/dev.escouade.gitea).
    ("gitea", escouade_plugin_gitea::run),
    // Trello (INT-7.1): the Kanban's source of Trello cards, in its crate (plugins/builtin/dev.escouade.trello).
    ("trello", escouade_plugin_trello::run),
    // Azure DevOps and Azure Boards (INT-8.5, ESC-280): its code host and its source, in its crate
    // (plugins/builtin/dev.escouade.azure-devops).
    ("azure-devops", escouade_plugin_azure_devops::run),
    // The generic ACP plugin (PLG-6): the harnesses a manifest describes, in its crate (plugins/builtin/dev.escouade.acp).
    ("acp", escouade_plugin_acp::run),
    // Linear (INT-7.2): the Kanban's source of Linear issues, in its crate (plugins/builtin/dev.escouade.linear).
    ("linear", escouade_plugin_linear::run),
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
