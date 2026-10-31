//! The contract of the Kanban's ticket sources (INT-1): what a source answers (`TicketSource`),
//! what Escouade knows of its kind (`TicketSourceKind`), and the registry of the kinds, as the
//! code hosts' (`forge::Registry`). Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues
//! and Azure Boards are behind it; a source a plugin serves will be too (`docs/PLUGINS.md` §4).

use super::*;
use crate::forge::BoxFuture;
use std::sync::{Arc, OnceLock};

/// What talks to one source for one account (or, for the sources whose account is a code host's,
/// for every account of its host). The calls the window and the sync make.
pub trait TicketSource: Send + Sync {
    /// The account's label and user, once its credentials are accepted.
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>>;

    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>>;

    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>>;

    /// The states of a container, and those each column gives by default: guessed by their
    /// names unless the source knows better (ClickUp by their types too, Azure Boards by its
    /// categories).
    fn states_view<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<StatesView>> {
        Box::pin(async move {
            let states = self.states(container).await?;
            Ok(StatesView {
                defaults: text::default_states(&states),
                states,
            })
        })
    }

    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>>;

    /// Gives the external ticket `state`; `mapped`: every state the link gives a column (GitHub
    /// and GitLab take the labels of the others off).
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>>;

    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>>;
}

/// One kind of source: what Escouade knows of it before any account is connected.
pub trait TicketSourceKind: Send + Sync {
    /// What the data and the window know it by (`ExternalRef::service`): `jira`, `github`…
    fn id(&self) -> &'static str;
    /// Its name, as the window and the comments say it: « Jira », « GitHub », « Azure Boards ».
    fn label(&self) -> &'static str;
    /// How it names itself in a sentence (« depuis GitHub »).
    fn short(&self) -> &'static str;
    /// The code host whose accounts serve it, when it has one (`github`, `gitlab`, `azure`).
    fn host(&self) -> Option<&'static str> {
        None
    }
    /// It has an account of its own (`Accounts`); false when it is connected, and disconnected,
    /// as its code host.
    fn own_account(&self) -> bool {
        true
    }
    /// A client for `account` (for GitHub, `account.token` is the token itself: the CLI's
    /// already asked for).
    fn build(
        &self,
        account: &Account,
        http: reqwest::Client,
        bases: &Bases,
    ) -> Box<dyn TicketSource>;
}

/// The kinds of sources Escouade knows.
#[derive(Clone)]
pub struct Registry {
    kinds: Vec<Arc<dyn TicketSourceKind>>,
}

impl Registry {
    /// Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues and Azure Boards.
    pub fn standard() -> Self {
        Self {
            kinds: vec![
                Arc::new(JiraKind),
                Arc::new(TrelloKind),
                Arc::new(GithubKind),
                Arc::new(LinearKind),
                Arc::new(ClickupKind),
                Arc::new(GitlabKind),
                Arc::new(BoardsKind),
            ],
        }
    }

    /// One more kind (a fake one in tests; a plugin's, later).
    #[cfg(test)]
    pub fn with(mut self, kind: Arc<dyn TicketSourceKind>) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn all(&self) -> &[Arc<dyn TicketSourceKind>] {
        &self.kinds
    }

    /// Their ids, in the registry's order.
    pub fn services(&self) -> Vec<Service> {
        self.kinds.iter().map(|k| Service::new(k.id())).collect()
    }

    pub fn get(&self, service: Service) -> Option<&Arc<dyn TicketSourceKind>> {
        self.kinds.iter().find(|k| k.id() == service.as_str())
    }

    pub fn position(&self, service: Service) -> Option<usize> {
        self.kinds.iter().position(|k| k.id() == service.as_str())
    }

    /// A client of `service` for `account`; an error when this version does not know the source.
    pub fn client(
        &self,
        service: Service,
        account: &Account,
        http: reqwest::Client,
        bases: &Bases,
    ) -> Result<Box<dyn TicketSource>> {
        let kind = self
            .get(service)
            .ok_or_else(|| anyhow!("Source inconnue : {service}"))?;
        Ok(kind.build(account, http, bases))
    }
}

/// The registry this version runs with.
pub fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::standard)
}

/// What the window knows of a kind, at its start (`InitialState::sources`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceDecl {
    pub id: String,
    pub name: String,
    pub short: String,
    /// The code host whose accounts serve it.
    pub host: Option<String>,
    /// It has an account of its own; false when it is connected as its code host.
    pub own_account: bool,
}

pub fn declared() -> Vec<SourceDecl> {
    registry()
        .all()
        .iter()
        .map(|k| SourceDecl {
            id: k.id().into(),
            name: k.label().into(),
            short: k.short().into(),
            host: k.host().map(Into::into),
            own_account: k.own_account(),
        })
        .collect()
}

/// A source's calls, `TicketSource`'s, from the inherent ones of its client (`set_state` says
/// which shape its `set_state` has, `view` whether it has a `states_view` of its own).
macro_rules! ticket_source {
    ($t:ty, set_state: $shape:ident, view: default) => {
        ticket_source!(@impl $t, $shape, {});
    };
    ($t:ty, set_state: $shape:ident, view: own) => {
        ticket_source!(@impl $t, $shape, {
            fn states_view<'a>(&'a self, c: &'a str) -> BoxFuture<'a, Result<StatesView>> {
                Box::pin(<$t>::states_view(self, c))
            }
        });
    };
    (@set plain, $t:ty, $s:ident, $r:ident, $st:ident, $m:ident) => {
        {
            let _ = $m;
            <$t>::set_state($s, $r, $st)
        }
    };
    (@set mapped, $t:ty, $s:ident, $r:ident, $st:ident, $m:ident) => {
        <$t>::set_state($s, $r, $st, $m)
    };
    (@impl $t:ty, $shape:ident, { $($extra:item)* }) => {
        impl TicketSource for $t {
            fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
                Box::pin(<$t>::me(self))
            }
            fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
                Box::pin(<$t>::containers(self))
            }
            fn states<'a>(&'a self, c: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
                Box::pin(<$t>::states(self, c))
            }
            $($extra)*
            fn issues<'a>(&'a self, c: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
                Box::pin(<$t>::issues(self, c, q))
            }
            fn set_state<'a>(
                &'a self,
                r: &'a ExternalRef,
                state: &'a ExternalState,
                mapped: &'a [ExternalState],
            ) -> BoxFuture<'a, Result<()>> {
                Box::pin(ticket_source!(@set $shape, $t, self, r, state, mapped))
            }
            fn comment<'a>(&'a self, r: &'a ExternalRef, t: &'a str) -> BoxFuture<'a, Result<()>> {
                Box::pin(<$t>::comment(self, r, t))
            }
        }
    };
}

ticket_source!(jira::Jira, set_state: plain, view: default);
ticket_source!(trello::Trello, set_state: plain, view: default);
ticket_source!(github::Github, set_state: mapped, view: default);
ticket_source!(linear::Linear, set_state: plain, view: default);
ticket_source!(clickup::Clickup, set_state: plain, view: own);
ticket_source!(gitlab::Gitlab, set_state: mapped, view: default);
ticket_source!(azure::Boards, set_state: plain, view: own);

/// One kind of source whose client is built from the account, with `$base` the address of its
/// API in `Bases` (Jira's is the account's site).
macro_rules! kind {
    ($name:ident, $id:literal, $label:literal, $short:literal, |$a:ident, $http:ident, $bases:ident| $build:expr $(, host: $host:literal)? $(, own: $own:literal)?) => {
        pub struct $name;

        impl TicketSourceKind for $name {
            fn id(&self) -> &'static str {
                $id
            }
            fn label(&self) -> &'static str {
                $label
            }
            fn short(&self) -> &'static str {
                $short
            }
            $(fn host(&self) -> Option<&'static str> {
                Some($host)
            })?
            $(fn own_account(&self) -> bool {
                $own
            })?
            fn build(
                &self,
                $a: &Account,
                $http: reqwest::Client,
                $bases: &Bases,
            ) -> Box<dyn TicketSource> {
                let _ = ($a, &$http, $bases);
                Box::new($build)
            }
        }
    };
}

kind!(JiraKind, "jira", "Jira", "Jira", |a, http, _b| {
    jira::Jira::new(http, a)
});
kind!(TrelloKind, "trello", "Trello", "Trello", |a, http, b| {
    trello::Trello::new(http, a, &b.trello)
});
kind!(GithubKind, "github", "GitHub", "GitHub", |a, http, b| github::Github::new(
    http, a, &b.github
), host: "github");
kind!(LinearKind, "linear", "Linear", "Linear", |a, http, b| {
    linear::Linear::new(http, a, &b.linear)
});
kind!(
    ClickupKind,
    "clickup",
    "ClickUp",
    "ClickUp",
    |a, http, b| clickup::Clickup::new(http, a, &b.clickup)
);
// One instance: the account's site, token and user (`Core::gitlab_client` gives every one).
kind!(GitlabKind, "gitlab", "GitLab", "GitLab", |a, http, _b| gitlab::Gitlab::new(
    http,
    vec![gitlab::Instance {
        host: crate::forge::host_of(&a.site),
        base: format!("{}/api/v4", a.site.trim_end_matches('/')),
        token: a.token.trim().to_string(),
        user: a.user.clone(),
        label: a.label.clone(),
    }],
), host: "gitlab", own: false);
// One organization: the account's address and token (`Core::azure_client` gives every one).
kind!(BoardsKind, "azure", "Azure Boards", "Azure Boards", |a, http, _b| {
    let url = a.site.trim_end_matches('/').to_string();
    azure::Boards::new(vec![azure::Instance {
        org: url.rsplit('/').next().unwrap_or("").to_string(),
        client: crate::forge::azure_api::Azure::new(
            http,
            url.clone(),
            crate::forge::azure::api_override(&url),
            a.token.clone(),
        ),
        token_page: format!("{url}/_usersSettings/tokens"),
    }])
}, host: "azure", own: false);

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl TicketSourceKind for Fake {
        fn id(&self) -> &'static str {
            "yougile"
        }
        fn label(&self) -> &'static str {
            "YouGile"
        }
        fn short(&self) -> &'static str {
            "YouGile"
        }
        fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
            Box::new(crate::integrations::jira::Jira::new(
                reqwest::Client::new(),
                &Account::default(),
            ))
        }
    }

    #[test]
    fn the_registry_has_the_seven_sources_in_the_order_of_the_enumeration() {
        let ids: Vec<&str> = registry().all().iter().map(|k| k.id()).collect();
        assert_eq!(
            ids,
            ["jira", "trello", "github", "linear", "clickup", "gitlab", "azure"]
        );
        let labels: Vec<&str> = registry().all().iter().map(|k| k.label()).collect();
        assert_eq!(
            labels,
            [
                "Jira",
                "Trello",
                "GitHub",
                "Linear",
                "ClickUp",
                "GitLab",
                "Azure Boards"
            ]
        );
        assert_eq!(Service::GITLAB.label(), "GitLab");
    }

    #[test]
    fn a_source_is_serialized_as_its_id_and_an_unknown_one_is_kept() {
        let ext = ExternalRef {
            service: Service::GITHUB,
            id: "42".into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&ext).unwrap();
        assert_eq!(json["service"], "github");
        let later: ExternalRef =
            serde_json::from_value(serde_json::json!({"service": "yougile", "id": "7"})).unwrap();
        assert_eq!(later.service.as_str(), "yougile");
        assert_eq!(later.service.label(), "yougile");
        assert_eq!(serde_json::to_value(&later).unwrap()["service"], "yougile");
        assert_eq!(Service::new("jira"), Service::JIRA);
        assert!(serde_json::from_value::<Service>(serde_json::json!("")).is_err());
        assert_eq!(Service::default(), Service::JIRA);
    }

    #[test]
    fn the_sources_keep_the_order_of_the_registry_the_unknown_after() {
        let mut v = [
            Service::new("zzz"),
            Service::AZURE,
            Service::new("aaa"),
            Service::JIRA,
            Service::GITHUB,
            Service::TRELLO,
        ];
        v.sort();
        let ids: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        assert_eq!(ids, ["jira", "trello", "github", "azure", "aaa", "zzz"]);
    }

    #[test]
    fn the_window_is_told_which_sources_are_a_code_hosts() {
        let d = declared();
        assert_eq!(d.len(), 7);
        let host = |id: &str| d.iter().find(|s| s.id == id).unwrap();
        assert_eq!(host("github").host.as_deref(), Some("github"));
        assert!(host("github").own_account);
        assert_eq!(host("gitlab").host.as_deref(), Some("gitlab"));
        assert!(!host("gitlab").own_account);
        assert!(!host("azure").own_account);
        assert_eq!(host("jira").host, None);
        assert_eq!(host("azure").short, "Azure Boards");
        assert_eq!(hosted_kind(Service::GITLAB), Some("gitlab"));
        assert_eq!(hosted_kind(Service::GITHUB), None);
        assert_eq!(hosted_kind(Service::new("yougile")), None);
    }

    #[test]
    fn a_kind_added_to_the_registry_is_found_and_an_unknown_one_has_no_client() {
        let reg = Registry::standard().with(Arc::new(Fake));
        let yougile = Service::new("yougile");
        assert_eq!(reg.services().len(), 8);
        assert_eq!(reg.get(yougile).unwrap().label(), "YouGile");
        assert!(reg
            .client(
                yougile,
                &Account::default(),
                reqwest::Client::new(),
                &Bases::from_env()
            )
            .is_ok());
        let err = Registry::standard().client(
            yougile,
            &Account::default(),
            reqwest::Client::new(),
            &Bases::from_env(),
        );
        assert!(err.is_err_and(|e| e.to_string().contains("yougile")));
    }

    #[test]
    fn the_forgotten_services_of_a_later_version_are_left_out() {
        let a: Accounts =
            serde_json::from_value(serde_json::json!({"toForget": ["jira", "yougile", "azure"]}))
                .unwrap();
        assert_eq!(a.to_forget, [Service::JIRA, Service::AZURE]);
    }
}
