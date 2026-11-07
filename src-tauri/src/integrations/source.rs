//! The contract of the Kanban's ticket sources (INT-1): what a source answers (`TicketSource`),
//! what Escouade knows of its kind (`TicketSourceKind`), and the registry of the kinds, as the
//! code hosts' (`forge::Registry`). Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues
//! and Azure Boards are behind it, each served by its plugin (`docs/PLUGINS.md` §4): the core keeps the contract.

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
    /// The plugin of Escouade's own hand that serves it, once it left the core (GitHub Issues: `dev.escouade.github`,
    /// INT-8.1): the core asks that plugin (`ProcessTicketSource`), and `build` is not called.
    fn plugin(&self) -> Option<&'static str> {
        None
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

// Jira: out of the core, served by the plugin « Jira » (INT-7.3).
pub struct JiraKind;

impl TicketSourceKind for JiraKind {
    fn id(&self) -> &'static str {
        "jira"
    }
    fn label(&self) -> &'static str {
        "Jira"
    }
    fn short(&self) -> &'static str {
        "Jira"
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::JIRA)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("Jira"))
    }
}
// Trello: out of the core, served by the plugin « Trello » (INT-7.1).
pub struct TrelloKind;

impl TicketSourceKind for TrelloKind {
    fn id(&self) -> &'static str {
        "trello"
    }
    fn label(&self) -> &'static str {
        "Trello"
    }
    fn short(&self) -> &'static str {
        "Trello"
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::TRELLO)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("Trello"))
    }
}
// GitHub Issues: out of the core, served by the plugin « GitHub » (INT-8.1).
pub struct GithubKind;

impl TicketSourceKind for GithubKind {
    fn id(&self) -> &'static str {
        "github"
    }
    fn label(&self) -> &'static str {
        "GitHub"
    }
    fn short(&self) -> &'static str {
        "GitHub"
    }
    fn host(&self) -> Option<&'static str> {
        Some("github")
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::GITHUB)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("GitHub"))
    }
}

/// A source the core does not serve itself: its plugin does (`TicketSourceKind::plugin`).
pub struct Unserved(pub &'static str);

impl Unserved {
    fn said<T>(&self) -> Result<T> {
        let name = self.0;
        Err(anyhow::anyhow!(tr!(
            "{name} est servi par son plugin, pas par le cœur d'Escouade",
            "{name} is served by its plugin, not by the core of Escouade"
        )))
    }
}

impl TicketSource for Unserved {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(async move { self.said() })
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(async move { self.said() })
    }
    fn states<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(async move { self.said() })
    }
    fn issues<'a>(&'a self, _: &'a str, _: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(async move { self.said() })
    }
    fn set_state<'a>(
        &'a self,
        _: &'a ExternalRef,
        _: &'a ExternalState,
        _: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move { self.said() })
    }
    fn comment<'a>(&'a self, _: &'a ExternalRef, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move { self.said() })
    }
}
// Linear: out of the core, served by the plugin « Linear » (INT-7.2).
pub struct LinearKind;

impl TicketSourceKind for LinearKind {
    fn id(&self) -> &'static str {
        "linear"
    }
    fn label(&self) -> &'static str {
        "Linear"
    }
    fn short(&self) -> &'static str {
        "Linear"
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::LINEAR)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("Linear"))
    }
}
// ClickUp: out of the core, served by the plugin « ClickUp » (INT-7.4).
pub struct ClickupKind;

impl TicketSourceKind for ClickupKind {
    fn id(&self) -> &'static str {
        "clickup"
    }
    fn label(&self) -> &'static str {
        "ClickUp"
    }
    fn short(&self) -> &'static str {
        "ClickUp"
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::CLICKUP)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("ClickUp"))
    }
}
// GitLab Issues: out of the core, served by the plugin « GitLab » (INT-8.2), on the accounts of the code host.
pub struct GitlabKind;

impl TicketSourceKind for GitlabKind {
    fn id(&self) -> &'static str {
        "gitlab"
    }
    fn label(&self) -> &'static str {
        "GitLab"
    }
    fn short(&self) -> &'static str {
        "GitLab"
    }
    fn host(&self) -> Option<&'static str> {
        Some("gitlab")
    }
    fn own_account(&self) -> bool {
        false
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::GITLAB)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("GitLab"))
    }
}
// Azure Boards: out of the core, served by the plugin « Azure DevOps » (INT-8.5), on its code host's accounts.
pub struct BoardsKind;

impl TicketSourceKind for BoardsKind {
    fn id(&self) -> &'static str {
        "azure"
    }
    fn label(&self) -> &'static str {
        "Azure Boards"
    }
    fn short(&self) -> &'static str {
        "Azure Boards"
    }
    fn host(&self) -> Option<&'static str> {
        Some("azure")
    }
    fn own_account(&self) -> bool {
        false
    }
    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::AZURE_DEVOPS)
    }
    fn build(&self, _: &Account, _: reqwest::Client, _: &Bases) -> Box<dyn TicketSource> {
        Box::new(Unserved("Azure Boards"))
    }
}

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
            Box::new(Unserved("YouGile"))
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
