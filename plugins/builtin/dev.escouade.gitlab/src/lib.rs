//! The plugin « GitLab » (`dev.escouade.gitlab`, docs/PLUGINS.md §4.4): gitlab.com and the self-managed instances, on
//! one account per instance. It serves the code host `gitlab` (merge requests, merge and auto-merge, pipelines and their
//! jobs, a job's log, reruns, approvals, discussions, deployments: `merges`). GitLab Issues, the Kanban's source on the
//! same accounts, is still the core's (`integrations/gitlab.rs`): INT-8.2 (ESC-218) brings it here, a `SourceKind`
//! beside `GitlabForge` as GitHub's. Its program is `escouade-plugins gitlab`; the kit (`escouade-plugin-kit::hosts`)
//! answers the protocol, gets each account's token from Escouade and keeps one client per account.

pub mod merges;

use escouade_plugin_kit::forge::Forge;
use escouade_plugin_kit::hosts::{Credentials, ForgeKind, Hosts};

/// Its name, in what it says.
pub const NAME: &str = "GitLab";

/// Where an instance's API is when Escouade does not say it: `/api/v4` of its address (gitlab.com's, or a
/// self-managed one's, under a path of its own too: `https://acme.dev/gitlab/api/v4`).
pub fn api_base(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.is_empty() {
        return "https://gitlab.com/api/v4".into();
    }
    format!("{url}/api/v4")
}

fn base_of(account: &Credentials) -> String {
    account
        .api
        .clone()
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| api_base(&account.url))
}

/// The code host.
pub struct GitlabForge;

impl ForgeKind for GitlabForge {
    fn id(&self) -> &str {
        "gitlab"
    }
    fn name(&self) -> &str {
        NAME
    }
    fn default_url(&self) -> Option<&str> {
        Some("https://gitlab.com")
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(merges::Gitlab::new(
            http,
            base_of(account),
            account.token.clone(),
        ))
    }
}

/// What `escouade-plugins gitlab` runs.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run("gitlab", Hosts::new(vec![Box::new(GitlabForge)], vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_is_found_from_the_instance() {
        assert_eq!(api_base("https://gitlab.com"), "https://gitlab.com/api/v4");
        assert_eq!(
            api_base("https://gitlab.acme.dev:8443/"),
            "https://gitlab.acme.dev:8443/api/v4"
        );
        assert_eq!(
            api_base("https://acme.dev/gitlab"),
            "https://acme.dev/gitlab/api/v4"
        );
        assert_eq!(api_base(""), "https://gitlab.com/api/v4");
    }

    #[test]
    fn the_api_the_account_says_wins() {
        let creds = Credentials {
            url: "https://gitlab.acme.dev".into(),
            api: Some("http://127.0.0.1:9/api/v4".into()),
            ..Default::default()
        };
        assert_eq!(base_of(&creds), "http://127.0.0.1:9/api/v4");
    }
}
