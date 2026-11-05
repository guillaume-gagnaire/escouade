//! Trello (REST API 1), with an API key and a token: the Kanban's source of Trello cards, a board's lists as the
//! columns' states. The account's secret is `{"key":…,"token":…}` (`secrets/get` of `account:trello`).

use anyhow::Result;
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::hosts::{Credentials, SourceKind, TicketSource};
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::http::{call, encode};
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::protocol::tickets::{
    Container, ExternalIssue, ExternalRef, ExternalState, IssueFilter, IssuePage, Query,
};
use escouade_plugin_kit::text::{criteria_of, names_criteria};
use escouade_plugin_kit::tr_in;
use serde_json::Value;

/// The source's id, and its name in what it says.
pub const ID: &str = "trello";
const S: &str = "Trello";

/// Where the API is, unless Escouade says otherwise (`ESCOUADE_TRELLO_API`: tests, demos).
const API: &str = "https://api.trello.com/1";

pub struct Trello {
    http: reqwest::Client,
    base: String,
    key: String,
    token: String,
    user: String,
}

/// The criteria of a card: the items of its checklist named after them, else of all its
/// checklists, else those its description lists.
fn card_criteria(checklists: &[&Value], desc: &str) -> Vec<String> {
    let items = |lists: Vec<&&Value>| -> Vec<String> {
        lists
            .iter()
            .flat_map(|c| c["checkItems"].as_array().cloned().unwrap_or_default())
            .filter_map(|i| i["name"].as_str().map(|n| n.trim().to_string()))
            .filter(|n| !n.is_empty())
            .collect()
    };
    let named: Vec<&&Value> = checklists
        .iter()
        .filter(|c| names_criteria(c["name"].as_str().unwrap_or("")))
        .collect();
    let mut found = items(named);
    if found.is_empty() {
        found = items(checklists.iter().collect());
    }
    if found.is_empty() {
        found = criteria_of(desc);
    }
    found.truncate(20);
    found
}

impl Trello {
    pub fn new(http: reqwest::Client, account: &Credentials, base: &str) -> Self {
        // The secret is the key and the token, as the core keeps them.
        let secret: Value = serde_json::from_str(&account.token).unwrap_or(Value::Null);
        let text = |k: &str| secret[k].as_str().unwrap_or("").trim().to_string();
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            key: text("key"),
            token: text("token"),
            user: account.user.clone(),
        }
    }

    fn url(&self, path: &str) -> String {
        let sep = if path.contains('?') { '&' } else { '?' };
        format!(
            "{}{path}{sep}key={}&token={}",
            self.base,
            encode(&self.key),
            encode(&self.token)
        )
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.get(self.url(path))
    }

    pub async fn me(&self) -> Result<(String, String)> {
        let me = call(S, self.get("/members/me?fields=username,fullName")).await?;
        Ok((
            format!("@{}", me["username"].as_str().unwrap_or("?")),
            me["id"].as_str().unwrap_or_default().to_string(),
        ))
    }

    pub async fn containers(&self) -> Result<Vec<Container>> {
        let v = call(S, self.get("/members/me/boards?filter=open&fields=name")).await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .map(|b| Container {
                id: b["id"].as_str().unwrap_or_default().to_string(),
                name: b["name"].as_str().unwrap_or_default().to_string(),
                group: None,
                own: false,
            })
            .collect())
    }

    /// The board's open lists.
    pub async fn states(&self, board: &str) -> Result<Vec<ExternalState>> {
        let v = call(
            S,
            self.get(&format!(
                "/boards/{}/lists?filter=open&fields=name",
                encode(board)
            )),
        )
        .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .map(|l| ExternalState {
                id: l["id"].as_str().unwrap_or_default().to_string(),
                name: l["name"].as_str().unwrap_or_default().to_string(),
            })
            .collect())
    }

    /// The board's open cards; "mine" keeps those I am a member of, a list's chip those of its
    /// list (any of the chosen lists).
    pub async fn issues(&self, board: &str, q: &Query) -> Result<IssuePage> {
        let b = encode(board);
        let lists = self.states(board).await?;
        let cards = call(
            S,
            self.get(&format!(
                "/boards/{b}/cards/open?fields=name,desc,idList,idShort,shortUrl,labels,idMembers"
            )),
        )
        .await?;
        let checklists = call(
            S,
            self.get(&format!(
                "/boards/{b}/checklists?fields=name,idCard&checkItem_fields=name,state"
            )),
        )
        .await?;
        let checklists: Vec<&Value> = checklists.as_array().into_iter().flatten().collect();
        let text = q.text.trim().to_lowercase();
        let chosen: Vec<&str> = q
            .filters
            .iter()
            .filter_map(|f| f.strip_prefix("list:"))
            .collect();
        let mine = q.filters.iter().any(|f| f == "mine");
        let label = q.label.as_deref().map(|l| l.trim().to_lowercase());
        let issues = cards
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| {
                let list = c["idList"].as_str().unwrap_or_default();
                let key = format!("#{}", c["idShort"]);
                let name = c["name"].as_str().unwrap_or_default().to_lowercase();
                (text.is_empty()
                    || name.contains(&text)
                    || key == text
                    || key == format!("#{text}"))
                    && (chosen.is_empty() || chosen.contains(&list))
                    && (!mine
                        || c["idMembers"]
                            .as_array()
                            .is_some_and(|m| m.iter().any(|x| x.as_str() == Some(&self.user))))
                    && label.as_ref().is_none_or(|l| {
                        c["labels"].as_array().into_iter().flatten().any(|x| {
                            x["name"]
                                .as_str()
                                .is_some_and(|n| n.trim().to_lowercase() == *l)
                        })
                    })
            })
            .map(|c| {
                let id = c["id"].as_str().unwrap_or_default();
                let desc = c["desc"].as_str().unwrap_or_default().to_string();
                let mine: Vec<&Value> = checklists
                    .iter()
                    .filter(|l| l["idCard"].as_str() == Some(id))
                    .copied()
                    .collect();
                let list = lists
                    .iter()
                    .find(|l| Some(l.id.as_str()) == c["idList"].as_str())
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                let labels: Vec<String> = c["labels"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|l| {
                        l["name"]
                            .as_str()
                            .filter(|n| !n.is_empty())
                            .map(str::to_string)
                    })
                    .collect();
                let mut meta = vec![list];
                meta.extend(labels);
                ExternalIssue {
                    service: S.to_lowercase(),
                    id: id.to_string(),
                    key: format!("#{}", c["idShort"]),
                    title: c["name"].as_str().unwrap_or_default().to_string(),
                    kind: card(i18n::ui()),
                    kind_code: "card".into(),
                    meta: meta.into_iter().filter(|m| !m.is_empty()).collect(),
                    url: c["shortUrl"].as_str().unwrap_or_default().to_string(),
                    criteria: card_criteria(&mine, &desc),
                    description: desc,
                    container: board.to_string(),
                    imported: false,
                }
            })
            .collect();
        let mut filters = vec![mine_filter(i18n::ui())];
        filters.extend(lists.iter().map(|l| IssueFilter {
            id: format!("list:{}", l.id),
            label: l.name.clone(),
        }));
        Ok(IssuePage {
            issues,
            filters,
            ..Default::default()
        })
    }

    /// The card goes into the list.
    pub async fn set_state(&self, r: &ExternalRef, state: &ExternalState) -> Result<()> {
        call(
            S,
            self.http.put(self.url(&format!(
                "/cards/{}?idList={}",
                encode(&r.id),
                encode(&state.id)
            ))),
        )
        .await?;
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        call(
            S,
            // In the body: a long comment would not fit in an address.
            self.http
                .post(self.url(&format!("/cards/{}/actions/comments", encode(&r.id))))
                .json(&serde_json::json!({ "text": text })),
        )
        .await?;
        Ok(())
    }
}

/// What a Trello card is called in the import's list.
fn card(lang: Lang) -> String {
    tr_in!(lang, "Carte", "Card")
}

/// The filter of the cards the account is a member of.
fn mine_filter(lang: Lang) -> IssueFilter {
    IssueFilter {
        id: "mine".into(),
        label: tr_in!(lang, "Mes cartes", "My cards"),
    }
}

impl TicketSource for Trello {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(Trello::me(self))
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(Trello::containers(self))
    }
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(Trello::states(self, container))
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(Trello::issues(self, container, q))
    }
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        _mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(Trello::set_state(self, r, state))
    }
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(Trello::comment(self, r, text))
    }
}

/// The Kanban's source, Trello.
pub struct TrelloSource;

impl SourceKind for TrelloSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        let base = account.api.clone().filter(|a| !a.trim().is_empty());
        Box::new(Trello::new(http, account, base.as_deref().unwrap_or(API)))
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "trello",
        escouade_plugin_kit::hosts::Hosts::new(vec![], vec![Box::new(TrelloSource)]),
    )
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_of_trello_reads_in_english() {
        assert_eq!(mine_filter(Lang::En).label, "My cards");
        assert_eq!(card(Lang::En), "Card");
    }

    #[test]
    fn the_secret_is_the_key_and_the_token() {
        let account = Credentials {
            token: r#"{"key":" k1 ","token":"t 1"}"#.into(),
            user: "me".into(),
            ..Default::default()
        };
        let t = Trello::new(reqwest::Client::new(), &account, "https://x/1/");
        assert_eq!((t.key.as_str(), t.token.as_str()), ("k1", "t 1"));
        assert_eq!(
            t.url("/members/me?fields=a"),
            "https://x/1/members/me?fields=a&key=k1&token=t%201"
        );
    }
}
