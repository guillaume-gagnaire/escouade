//! A ticket source whose accounts are a code host's (GitLab Issues, one account per instance): a plugin serves each
//! account (`ProcessTicketSource`, its process one for every instance), and this reads them as the core did when one
//! client held every instance. A container is `<host>/<path>` (`gitlab.com/acme/api`): its host names the account
//! that serves it. The projects are those of every account, instance by instance.

use super::adapters::ProcessTicketSource;
use crate::forge::BoxFuture;
use crate::integrations::source::TicketSource;
use crate::integrations::{Container, IssuePage, Query, StatesView};
use crate::model::{ExternalRef, ExternalState};
use anyhow::{anyhow, Result};

/// The accounts of one source on a code host's instances, each served by a plugin.
pub struct HostedTicketSource {
    /// The source's name, as it speaks (`GitLab`).
    name: &'static str,
    /// Each account: its instance's host (`gitlab.com`, `git.acme.dev:8443`), and its client.
    accounts: Vec<(String, ProcessTicketSource)>,
}

impl HostedTicketSource {
    pub fn new(name: &'static str, accounts: Vec<(String, ProcessTicketSource)>) -> Self {
        Self { name, accounts }
    }

    /// The account a container names by its host.
    fn of(&self, container: &str) -> Result<&ProcessTicketSource> {
        let host = container.split_once('/').map_or(container, |(h, _)| h);
        let name = self.name;
        self.accounts
            .iter()
            .find(|(h, _)| h.eq_ignore_ascii_case(host))
            .map(|(_, a)| a)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "Aucun compte {name} connecté pour {host}",
                    "No {name} account connected for {host}"
                ))
            })
    }
}

impl TicketSource for HostedTicketSource {
    /// The first account's, as the window shows one connection.
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(async move {
            let name = self.name;
            let (_, first) = self.accounts.first().ok_or_else(|| {
                anyhow!(tr!(
                    "Aucun compte {name} connecté",
                    "No {name} account connected"
                ))
            })?;
            first.me().await
        })
    }

    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for (_, account) in &self.accounts {
                out.extend(account.containers().await?);
            }
            Ok(out)
        })
    }

    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(async move { self.of(container)?.states(container).await })
    }

    fn states_view<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<StatesView>> {
        Box::pin(async move { self.of(container)?.states_view(container).await })
    }

    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(async move { self.of(container)?.issues(container, q).await })
    }

    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move { self.of(&r.container)?.set_state(r, state, mapped).await })
    }

    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move { self.of(&r.container)?.comment(r, text).await })
    }
}
