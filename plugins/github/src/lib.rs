//! The plugin « GitHub » (`dev.escouade.github`, docs/PLUGINS.md §4.4): github.com, GHE.com and GitHub Enterprise
//! Server, on one account per instance. It serves the code host `github` (pull requests, merge and auto-merge, checks,
//! Actions logs and reruns, reviews and their threads, deployments: `pulls`) and, on github.com's account, the Kanban's
//! source `github` (GitHub Issues: `issues`). It is a module (`run.wasm`); the kit
//! (`escouade-plugin-kit::hosts`) answers the protocol, gets each account's token from Escouade and keeps one client
//! per account.

pub mod issues;
pub mod pulls;
mod redirect;

#[cfg(test)]
#[path = "../../fake_service.rs"]
mod fake;

use escouade_plugin_kit::forge::Forge;
use escouade_plugin_kit::hosts::{Credentials, ForgeKind, Hosts, SourceKind, TicketSource};
use escouade_plugin_kit::http::host_of;
use escouade_plugin_kit::http::reqwest;

/// Its name, in what it says.
pub const NAME: &str = "GitHub";

/// Where an instance's API is when Escouade does not say it: api.github.com for github.com, `api.<name>.ghe.com` for
/// GHE.com, `/api/v3` of a GitHub Enterprise Server. (The core says github.com's: `ESCOUADE_GITHUB_API`, a fake one.)
pub fn api_base(url: &str) -> String {
    let host = host_of(url);
    if host.is_empty() || host == "github.com" {
        return "https://api.github.com".into();
    }
    if host.ends_with(".ghe.com") {
        return format!("https://api.{host}");
    }
    format!("{}/api/v3", url.trim_end_matches('/'))
}

fn base_of(account: &Credentials) -> String {
    account
        .api
        .clone()
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| api_base(&account.url))
}

/// The code host.
pub struct GithubForge;

impl ForgeKind for GithubForge {
    fn id(&self) -> &str {
        "github"
    }
    fn name(&self) -> &str {
        NAME
    }
    fn default_url(&self) -> Option<&str> {
        Some("https://github.com")
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(pulls::Github::new(
            http,
            base_of(account),
            account.token.clone(),
        ))
    }
}

/// The Kanban's source, GitHub Issues.
pub struct GithubIssues;

impl SourceKind for GithubIssues {
    fn id(&self) -> &str {
        issues::ID
    }
    fn name(&self) -> &str {
        NAME
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        Box::new(issues::Github::new(http, &account.token, &base_of(account)))
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "github",
        Hosts::new(vec![Box::new(GithubForge)], vec![Box::new(GithubIssues)]),
    )
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_is_found_from_the_instance() {
        assert_eq!(api_base("https://github.com"), "https://api.github.com");
        assert_eq!(api_base("https://octo.ghe.com"), "https://api.octo.ghe.com");
        assert_eq!(
            api_base("https://ghe.acme.dev/"),
            "https://ghe.acme.dev/api/v3"
        );
    }
}
