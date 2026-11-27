//! The agents' pages as the state keeps them (ESC-673, docs/AGENT-PAGES.md § 3): a page, `p-<n>`, belongs
//! to the agent that showed it (its project, its ticket), and has its versions, each a frozen copy of what
//! Escouade serves (`<données>/pages/<projet>/<p-n>/v<k>/`). The app's `pages` keeps, bounds and purges them.

use serde::{Deserialize, Serialize};

/// A page an agent showed.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentPage {
    /// `p-<n>`, never given twice.
    pub id: String,
    pub project_id: String,
    /// The agent that showed it (its id).
    pub agent_id: String,
    /// Its name when it showed the page, for what outlives it.
    pub agent_name: String,
    /// The agent's ticket when it showed the page, if any.
    pub ticket_id: Option<String>,
    /// One line, 120 characters at most.
    pub title: String,
    /// `review`, `diagram`, `compare`, `html` (its own) or `custom` (the user's own page).
    pub template: String,
    /// Its versions kept, the oldest first (10 at most).
    pub versions: Vec<PageVersion>,
    /// The request of the box it carries (`a-<n>`), if any (AP-4).
    pub ask: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    /// When its agent was found deleted or archived: the page goes after the days of the box's closed
    /// files (« Garder les fichiers des demandes closes »).
    pub orphaned_at: Option<i64>,
}

impl AgentPage {
    /// The last version.
    pub fn latest(&self) -> Option<&PageVersion> {
        self.versions.last()
    }

    /// What all its versions weigh.
    pub fn weight(&self) -> u64 {
        self.versions.iter().map(|v| v.bytes).sum()
    }
}

/// A version of a page: a frozen copy.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct PageVersion {
    /// 1, 2…: never given twice within its page.
    pub n: u32,
    pub at: i64,
    /// What its files weigh.
    pub bytes: u64,
    /// How many files it has (its document among them).
    pub files: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_saved_before_a_field_reads_with_its_default() {
        let p: AgentPage =
            serde_json::from_str(r#"{"id":"p-3","versions":[{"n":1,"bytes":10}]}"#).unwrap();
        assert_eq!(p.id, "p-3");
        assert_eq!(p.latest().map(|v| v.n), Some(1));
        assert_eq!(p.weight(), 10);
        assert!(p.orphaned_at.is_none());
        let back = serde_json::to_value(&p).unwrap();
        assert_eq!(back["agentId"], "");
        assert!(back["orphanedAt"].is_null());
    }
}
