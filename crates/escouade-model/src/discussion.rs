//! What makes an agent a discussion of the Docs tab (`discussion`, ESC-261): its role, its subject, and what it
//! proposes for the spec before it is poured.

use serde::{Deserialize, Serialize};

use crate::docs::{Decision, Gap, Page};

/// Its role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The spec writer.
    #[default]
    Discussion,
    /// The product agent.
    Brainstorm,
}

impl Kind {
    /// Its word as a decision's origin.
    pub fn origin(self) -> &'static str {
        match self {
            Kind::Discussion => "discussion",
            Kind::Brainstorm => "brainstorm",
        }
    }
}

/// What makes an agent a discussion (`AgentMeta::discussion`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Discussion {
    pub kind: Kind,
    pub subject: String,
    /// The pages it proposes, one change each (the latest of the page), not poured yet.
    pub pages: Vec<PageChange>,
    /// The decisions it proposes, not poured yet.
    pub decisions: Vec<Decision>,
    /// What was poured into the spec so far; None until then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poured: Option<Poured>,
    /// The gap it was opened on (« Mettre la spec à jour »): gone from its page once a change of
    /// that page is poured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settles: Option<Settles>,
    /// What « Planifier » made of what was poured (ESC-262); None until then, and once cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned: Option<crate::docsplan::Plan>,
}

/// A gap of a page a discussion settles.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settles {
    pub page: String,
    pub gap: Gap,
}

/// A page as the discussion proposes it.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PageChange {
    pub path: String,
    /// The page's text when its first change was proposed; None: a new page.
    pub before: Option<String>,
    /// The page as it would be written.
    pub page: Page,
    /// The lines it adds and removes from `before`.
    pub added: u32,
    pub removed: u32,
}

/// What « Verser dans la spec » wrote, every time added up.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Poured {
    /// When it was last poured (ms).
    pub at: i64,
    /// The pages written, each once.
    pub pages: Vec<String>,
    /// The decisions proposed in the register (their ids, ESC-303).
    pub decisions: Vec<crate::docs::Ref>,
}

impl Discussion {
    /// The page `path` as it proposes it, if it does.
    pub fn page(&self, path: &str) -> Option<&PageChange> {
        self.pages.iter().find(|p| p.path == path)
    }

    /// Keeps `page`, the latest change of its page: its `before` is the first change's.
    pub fn propose_page(&mut self, before: Option<String>, page: Page) -> &PageChange {
        let i = match self.pages.iter().position(|p| p.path == page.path) {
            Some(i) => {
                let c = &mut self.pages[i];
                c.page = page;
                i
            }
            None => {
                self.pages.push(PageChange {
                    path: page.path.clone(),
                    before,
                    page,
                    ..Default::default()
                });
                self.pages.len() - 1
            }
        };
        let c = &mut self.pages[i];
        let (added, removed) =
            crate::docs::line_delta(c.before.as_deref().unwrap_or(""), &c.page.body);
        c.added = added;
        c.removed = removed;
        c
    }

    /// Keeps the decision `d`, from this discussion: one of the same title is revised. Its place
    /// in the list, from 1.
    pub fn propose_decision(&mut self, mut d: Decision) -> usize {
        d.origin = self.kind.origin().into();
        if d.origin_ref.trim().is_empty() {
            d.origin_ref = self.subject.clone();
        }
        let title = d.title.trim().to_lowercase();
        match self
            .decisions
            .iter()
            .position(|o| o.title.trim().to_lowercase() == title)
        {
            Some(i) => {
                self.decisions[i] = d;
                i + 1
            }
            None => {
                self.decisions.push(d);
                self.decisions.len()
            }
        }
    }
}
