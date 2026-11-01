//! The plugin « Gitea / Forgejo » (`dev.escouade.gitea`, docs/PLUGINS.md §4.4): Gitea, Forgejo and Codeberg (a Forgejo)
//! and every instance of them, on one account per instance. It serves the code host `gitea`: pull requests, merge and
//! auto-merge (on a protected branch), combined statuses and Actions (logs, reruns), reviews and their threads. Its
//! program is `escouade-plugins gitea`; the kit (`escouade-plugin-kit::hosts`) answers the protocol, gets each
//! account's token from Escouade and keeps one client per account.
//!
//! It has no source of the Kanban, and no deployments: neither Gitea nor Forgejo has an API for them (the code host's
//! `Capabilities` say so). What only some versions do (Actions' logs and reruns, the threads' answers and resolutions)
//! is told by the client itself, which asks the instance's version once: `Unsupported`, never a wrong answer.

pub mod pulls;

use escouade_plugin_kit::forge::Forge;
use escouade_plugin_kit::hosts::{Credentials, ForgeKind, Hosts};

/// Its name, in what it says.
pub const NAME: &str = "Gitea";

/// Where an instance's API is when Escouade does not say it: `/api/v1` of its address.
pub fn api_base(url: &str) -> String {
    format!("{}/api/v1", url.trim_end_matches('/'))
}

/// The code host.
pub struct GiteaForge;

impl ForgeKind for GiteaForge {
    fn id(&self) -> &str {
        "gitea"
    }
    fn name(&self) -> &str {
        "Gitea / Forgejo"
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        let base = account
            .api
            .clone()
            .filter(|a| !a.trim().is_empty())
            .unwrap_or_else(|| api_base(&account.url));
        Box::new(pulls::Gitea::new(http, base, account.token.clone()))
    }
}

/// What `escouade-plugins gitea` runs.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run("gitea", Hosts::new(vec![Box::new(GiteaForge)], vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_is_found_from_the_instance() {
        assert_eq!(
            api_base("https://codeberg.org"),
            "https://codeberg.org/api/v1"
        );
        assert_eq!(
            api_base("https://git.acme.dev:3000/"),
            "https://git.acme.dev:3000/api/v1"
        );
    }
}
