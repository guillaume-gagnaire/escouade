//! The plugin « Azure DevOps » (`dev.escouade.azure-devops`, docs/PLUGINS.md §4.4): Azure DevOps on dev.azure.com (its
//! former `<org>.visualstudio.com` too) and Azure DevOps Server, on one account per organization (a Server's:
//! collection). It serves the code host `azure` (pull requests, merge and auto-complete, the policies' and Azure
//! Pipelines' checks, the builds' logs and their retry, threads, reviews: `pulls`) and, on the same account, the
//! Kanban's source `azure` (Azure Boards: `boards`). It is a module (`run.wasm`); the kit
//! (`escouade-plugin-kit::hosts`) answers the protocol, gets each account's token from Escouade and keeps one client per
//! account.
//!
//! The token is lent as Escouade holds it: a personal access token (sent in Basic, `:token`), or the one the `az` CLI
//! gives (an Entra ID token, in Bearer), which Escouade asks of `az` itself — the plugin runs no program.

pub mod boards;
pub mod html;
mod joined;
pub mod pulls;
mod redirect;

#[cfg(test)]
#[path = "../../fake_service.rs"]
mod fake;

use escouade_plugin_kit::forge::{Forge, RepoRef};
use escouade_plugin_kit::hosts::{Credentials, ForgeKind, Hosts, SourceKind, TicketSource};
use escouade_plugin_kit::http::reqwest;

/// Its name, in what it says.
pub const NAME: &str = "Azure DevOps";

/// Its public instance.
pub const CLOUD: &str = "dev.azure.com";

/// A remote's path segment, its percent-encoding (`My%20Project`) undone.
pub fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(b) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The organization (or collection) and the project of a repository (`RepoRef` keeps `org/project` as its
/// namespace).
pub fn parts(repo: &RepoRef) -> (&str, &str) {
    repo.namespace
        .split_once('/')
        .unwrap_or((repo.namespace.as_str(), repo.name.as_str()))
}

/// Where Escouade says the API is, unless it says nothing (a fake server in the tests).
fn api_of(account: &Credentials) -> Option<String> {
    account.api.clone().filter(|a| !a.trim().is_empty())
}

/// The code host.
pub struct AzureForge;

impl ForgeKind for AzureForge {
    fn id(&self) -> &str {
        "azure"
    }
    fn name(&self) -> &str {
        NAME
    }
    fn default_url(&self) -> Option<&str> {
        Some("https://dev.azure.com")
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(pulls::Azure::new(
            http,
            account.url.clone(),
            api_of(account),
            account.token.clone(),
        ))
    }
}

/// The Kanban's source, Azure Boards.
pub struct AzureBoards;

impl SourceKind for AzureBoards {
    fn id(&self) -> &str {
        boards::ID
    }
    fn name(&self) -> &str {
        "Azure Boards"
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        let url = account.url.trim_end_matches('/').to_string();
        Box::new(boards::Boards::new(vec![boards::Instance {
            org: decode(url.rsplit('/').next().unwrap_or("")),
            token_page: format!("{url}/_usersSettings/tokens"),
            client: pulls::Azure::new(http, url, api_of(account), account.token.clone()),
        }]))
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "azure-devops",
        Hosts::new(vec![Box::new(AzureForge)], vec![Box::new(AzureBoards)]),
    )
}

escouade_plugin_kit::protocol::module!(run);
