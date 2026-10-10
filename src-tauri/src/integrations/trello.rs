//! Trello (REST API 1), with an API key and a token.

use super::text::{criteria_of, names_criteria};
use super::{call, encode, Account, Container, ExternalIssue, IssueFilter, IssuePage, Query};
use crate::model::{ExternalRef, ExternalState, Service};
use anyhow::Result;
use serde_json::Value;

const S: Service = Service::Trello;

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
    pub fn new(http: reqwest::Client, a: &Account, base: &str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            key: a.key.trim().to_string(),
            token: a.token.trim().to_string(),
            user: a.user.clone(),
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
                    service: S,
                    id: id.to_string(),
                    key: format!("#{}", c["idShort"]),
                    title: c["name"].as_str().unwrap_or_default().to_string(),
                    kind: "Carte".into(),
                    meta: meta.into_iter().filter(|m| !m.is_empty()).collect(),
                    url: c["shortUrl"].as_str().unwrap_or_default().to_string(),
                    criteria: card_criteria(&mine, &desc),
                    description: desc,
                    container: board.to_string(),
                    imported: false,
                }
            })
            .collect();
        let mut filters = vec![IssueFilter {
            id: "mine".into(),
            label: "Mes cartes".into(),
        }];
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::fake::FakeServer;
    use serde_json::json;

    async fn trello() -> (FakeServer, Trello) {
        let server = FakeServer::start().await;
        let account = Account {
            key: "k1".into(),
            token: "t 1".into(),
            user: "me".into(),
            ..Default::default()
        };
        let t = Trello::new(reqwest::Client::new(), &account, &server.url);
        (server, t)
    }

    fn board(server: &FakeServer) {
        server.on(
            "GET",
            "/boards/b1/lists",
            200,
            json!([{ "id": "l1", "name": "Prêt pour Claude" }, { "id": "l2", "name": "En cours" }, { "id": "l3", "name": "Recette" }]),
        );
        server.on(
            "GET",
            "/boards/b1/cards/open",
            200,
            json!([
                { "id": "c1", "idShort": 151, "name": "Exporter le journal", "desc": "Export CSV.", "idList": "l1",
                  "shortUrl": "https://trello.com/c/abc", "labels": [{ "name": "claude-ready" }], "idMembers": ["me"] },
                { "id": "c2", "idShort": 156, "name": "Email de bienvenue", "desc": "## Critères\n- Template HTML", "idList": "l2",
                  "shortUrl": "https://trello.com/c/def", "labels": [], "idMembers": [] }
            ]),
        );
        server.on(
            "GET",
            "/boards/b1/checklists",
            200,
            json!([
                { "id": "k1", "idCard": "c1", "name": "Tâches", "checkItems": [{ "name": "Préparer" }] },
                { "id": "k2", "idCard": "c1", "name": "Critères d'acceptation", "checkItems": [{ "name": "Filtre par date" }, { "name": "Colonnes IP" }] }
            ]),
        );
    }

    #[tokio::test]
    async fn the_key_and_token_go_with_every_call() {
        let (server, t) = trello().await;
        server.on(
            "GET",
            "/members/me",
            200,
            json!({ "id": "me", "username": "ada" }),
        );
        assert_eq!(
            t.me().await.unwrap(),
            ("@ada".to_string(), "me".to_string())
        );
        let req = &server.requests()[0];
        assert_eq!(req.query("key").as_deref(), Some("k1"));
        assert_eq!(req.query("token").as_deref(), Some("t 1"));
        assert_eq!(req.query("fields").as_deref(), Some("username,fullName"));
        server.on(
            "GET",
            "/members/me",
            401,
            Value::String("invalid token".into()),
        );
        assert_eq!(
            t.me().await.unwrap_err().to_string(),
            "Trello refuse ces identifiants (401) — invalid token"
        );
    }

    #[tokio::test]
    async fn the_boards_and_their_lists_are_listed() {
        let (server, t) = trello().await;
        board(&server);
        server.on(
            "GET",
            "/members/me/boards",
            200,
            json!([{ "id": "b1", "name": "Atlas" }]),
        );
        assert_eq!(
            t.containers().await.unwrap(),
            [Container {
                id: "b1".into(),
                name: "Atlas".into()
            }]
        );
        let lists: Vec<String> = t
            .states("b1")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(lists, ["Prêt pour Claude", "En cours", "Recette"]);
    }

    #[tokio::test]
    async fn the_cards_come_with_their_list_and_criteria_and_are_filtered() {
        let (server, t) = trello().await;
        board(&server);
        let page = t.issues("b1", &Query::default()).await.unwrap();
        let c1 = &page.issues[0];
        assert_eq!(
            (c1.key.as_str(), c1.kind.as_str(), c1.id.as_str()),
            ("#151", "Carte", "c1")
        );
        assert_eq!(c1.meta, ["Prêt pour Claude", "claude-ready"]);
        assert_eq!(c1.criteria, ["Filtre par date", "Colonnes IP"]);
        assert_eq!(c1.url, "https://trello.com/c/abc");
        // Without a checklist: its description's.
        assert_eq!(page.issues[1].criteria, ["Template HTML"]);
        let ids: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["mine", "list:l1", "list:l2", "list:l3"]);
        // A board's open cards come all at once: no next page.
        assert_eq!((&page.next, page.total), (&None, None));
        let keys = |p: IssuePage| p.issues.into_iter().map(|i| i.key).collect::<Vec<_>>();
        let q = |text: &str, filters: &[&str], label: Option<&str>| Query {
            text: text.into(),
            filters: filters.iter().map(|f| f.to_string()).collect(),
            label: label.map(str::to_string),
            ..Default::default()
        };
        assert_eq!(
            keys(t.issues("b1", &q("bienvenue", &[], None)).await.unwrap()),
            ["#156"]
        );
        assert_eq!(
            keys(t.issues("b1", &q("156", &[], None)).await.unwrap()),
            ["#156"]
        );
        assert_eq!(
            keys(t.issues("b1", &q("", &["mine"], None)).await.unwrap()),
            ["#151"]
        );
        assert_eq!(
            keys(
                t.issues("b1", &q("", &["list:l2", "list:l3"], None))
                    .await
                    .unwrap()
            ),
            ["#156"]
        );
        assert_eq!(
            keys(
                t.issues("b1", &q("", &[], Some("Claude-Ready")))
                    .await
                    .unwrap()
            ),
            ["#151"]
        );
    }

    #[tokio::test]
    async fn a_card_moves_to_its_list_and_is_commented() {
        let (server, t) = trello().await;
        server.on("PUT", "/cards/c1", 200, json!({}));
        server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
        let r = ExternalRef {
            service: S,
            id: "c1".into(),
            ..Default::default()
        };
        t.set_state(
            &r,
            &ExternalState {
                id: "l3".into(),
                name: "Recette".into(),
            },
        )
        .await
        .unwrap();
        t.comment(&r, "Escouade : prêt & fini").await.unwrap();
        let w = server.writes();
        assert_eq!(
            (w[0].method.as_str(), w[0].query("idList").as_deref()),
            ("PUT", Some("l3"))
        );
        // In the body: a long comment would not fit in an address.
        assert_eq!(w[1].json(), json!({ "text": "Escouade : prêt & fini" }));
        assert_eq!(w[1].query("text"), None);
        assert_eq!(w[1].query("token").as_deref(), Some("t 1"));
    }
}
