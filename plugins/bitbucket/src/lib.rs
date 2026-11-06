//! The plugin « Bitbucket » (`dev.escouade.bitbucket`, docs/PLUGINS.md §4.4): Bitbucket Cloud (bitbucket.org), on one
//! account, an Atlassian API token with the e-mail of its account. It serves the code host `bitbucket`: pull requests
//! (drafts included), merge, commit statuses, pipelines and their step logs, comments on lines and their threads,
//! deployments. Bitbucket's API has no auto-merge: the core declares the capability absent (`Capabilities::auto_merge`).
//! It is a module (`run.wasm`); the kit (`escouade-plugin-kit::hosts`) answers the protocol, gets the
//! account's token from Escouade and keeps one client per account.

pub mod pulls;
mod redirect;

#[cfg(test)]
#[path = "../../fake_service.rs"]
mod fake;

use escouade_plugin_kit::forge::Forge;
use escouade_plugin_kit::hosts::{Credentials, ForgeKind, Hosts};
use escouade_plugin_kit::http::host_of;
use escouade_plugin_kit::http::reqwest;

/// Its name, in what it says.
pub const NAME: &str = "Bitbucket";

/// Where its API is: api.bitbucket.org for bitbucket.org, `/2.0` of any other address (a fake one in the tests).
pub fn api_base(url: &str) -> String {
    if host_of(url) == "bitbucket.org" || url.trim().is_empty() {
        return "https://api.bitbucket.org/2.0".into();
    }
    format!("{}/2.0", url.trim_end_matches('/'))
}

/// The code host.
pub struct BitbucketForge;

impl ForgeKind for BitbucketForge {
    fn id(&self) -> &str {
        "bitbucket"
    }
    fn name(&self) -> &str {
        NAME
    }
    fn default_url(&self) -> Option<&str> {
        Some("https://bitbucket.org")
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        let base = account
            .api
            .clone()
            .filter(|a| !a.trim().is_empty())
            .unwrap_or_else(|| api_base(&account.url));
        Box::new(pulls::Bitbucket::new(
            http,
            base,
            account.email.clone(),
            account.token.clone(),
        ))
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "bitbucket",
        Hosts::new(vec![Box::new(BitbucketForge)], vec![]),
    )
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_is_found_from_the_instance() {
        assert_eq!(
            api_base("https://bitbucket.org"),
            "https://api.bitbucket.org/2.0"
        );
        assert_eq!(
            api_base("http://127.0.0.1:4141/"),
            "http://127.0.0.1:4141/2.0"
        );
    }
}
