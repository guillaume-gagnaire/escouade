//! The spec the tickets keep up to date (ESC-263, `docs/FACTORY.md` « La spec toujours à jour »).
//!
//! A ticket's agent that changes how the product behaves updates the pages it touches and leaves
//! a decision, by the docs tools, before it hands the ticket in (its protocol says so; nothing
//! stops the validation). Where those writes go:
//!
//! - **the current branch**: its own worktree, so the spec is merged with the code;
//! - **a fixed branch**: a copy of the branch's spec and decisions, kept for the ticket in
//!   Escouade's folder (`ticket-docs/<key>/work`, with the copy as it was taken in `base`), which
//!   its agent reads and writes; poured on the branch **when the ticket is validated**, never
//!   before, so that the spec never tells of a behaviour not merged yet. Only what the ticket
//!   changed is poured: what the team wrote on the branch meanwhile stays. Its documents are the
//!   branch's own.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::Result;
use tauri::Runtime;

use crate::core::Core;
use crate::docs::{self, Space, DECISIONS, ROOT, SPEC};
use crate::model::Column;

/// What a ticket's copy brought to the branch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Poured {
    /// The pages written (`subject/page`).
    pub pages: Vec<String>,
    /// The pages it removed.
    pub removed: Vec<String>,
    /// The decisions written: by their number on the branch, or their id for a proposal.
    pub decisions: Vec<docs::Ref>,
}

/// The spec and the decisions of `from` copied into `to`, which holds `.escouade/` then, even when
/// there was nothing to copy.
pub fn seed(from: &Space, to: &Space) -> Result<()> {
    for sub in [SPEC, DECISIONS] {
        for rel in from.md_files(sub) {
            to.raw_write(sub, &rel, &from.raw_read(sub, &rel)?)?;
        }
    }
    std::fs::create_dir_all(to.root.join(ROOT))?;
    Ok(())
}

/// What `work` changed against `base` (the copy as it was taken), written in `into`. A new
/// decision whose number the branch took meanwhile gets the next free one, and the pages and
/// decisions that name it follow.
pub fn pour(work: &Space, base: &Space, into: &Space) -> Result<Poured> {
    let mut out = Poured::default();
    let changed =
        |sub: &str, rel: &str, text: &str| base.raw_read(sub, rel).ok().as_deref() != Some(text);

    // The decisions first, for their numbers. A proposal accepted by the validation took its number
    // after the branch's register already (ESC-303); one the branch took meanwhile all the same
    // gets the next free one.
    let base_numbers: HashSet<u32> = base.decisions().iter().filter_map(|d| d.number).collect();
    let mut taken: HashSet<u32> = into.decisions().iter().filter_map(|d| d.number).collect();
    let mut decisions: Vec<(String, docs::Decision)> = Vec::new();
    let mut kept_decisions = HashSet::new();
    for rel in work.md_files(DECISIONS) {
        kept_decisions.insert(rel.clone());
        let text = work.raw_read(DECISIONS, &rel)?;
        if !changed(DECISIONS, &rel, &text) {
            continue;
        }
        let Ok(mut d) = docs::decision_read(&text) else {
            continue;
        };
        if d.id.is_empty() {
            d.id = docs::id_of_file(std::path::Path::new(&rel));
        }
        decisions.push((rel, d));
    }
    let mut renumber: HashMap<u32, u32> = HashMap::new();
    let mut next = into.next_number().max(work.next_number());
    for (_, d) in &decisions {
        let Some(n) = d.number else { continue };
        if !base_numbers.contains(&n) && taken.contains(&n) {
            while taken.contains(&next) {
                next += 1;
            }
            renumber.insert(n, next);
            taken.insert(next);
        }
    }
    let map = |n: u32| renumber.get(&n).copied().unwrap_or(n);
    for (rel, mut d) in decisions {
        let moved = d.number.is_some_and(|n| renumber.contains_key(&n));
        d.number = d.number.map(map);
        d.supersedes = d.supersedes.iter().map(|n| map(*n)).collect();
        d.superseded_by = d.superseded_by.map(map);
        let name = if moved { d.file_name() } else { rel };
        into.raw_write(DECISIONS, &name, &docs::decision_write(&d))?;
        out.decisions.push(d.reference());
    }
    // A proposal numbered in the copy left its file `<id>.md`: gone from the branch too.
    for rel in base.md_files(DECISIONS) {
        if !kept_decisions.contains(&rel) {
            into.raw_remove(DECISIONS, &rel)?;
        }
    }

    // The pages.
    let mut kept = HashSet::new();
    for rel in work.md_files(SPEC) {
        kept.insert(rel.clone());
        let text = work.raw_read(SPEC, &rel)?;
        if !changed(SPEC, &rel, &text) {
            continue;
        }
        let path = rel.trim_end_matches(".md").to_string();
        let text = if renumber.is_empty() {
            text
        } else {
            let mut p = docs::page_read(&path, &text);
            p.decisions = p
                .decisions
                .iter()
                .map(|r| match r {
                    docs::Ref::Number(n) => docs::Ref::Number(map(*n)),
                    r => r.clone(),
                })
                .collect();
            docs::page_write(&p)
        };
        into.raw_write(SPEC, &rel, &text)?;
        out.pages.push(path);
    }
    for rel in base.md_files(SPEC) {
        if !kept.contains(&rel) {
            into.raw_remove(SPEC, &rel)?;
            out.removed.push(rel.trim_end_matches(".md").to_string());
        }
    }
    Ok(out)
}

impl<R: Runtime> Core<R> {
    /// Where the copy of the docs of ticket `key` is kept, on a fixed branch.
    pub(crate) fn ticket_docs_dir(&self, project_id: &str, key: &str) -> PathBuf {
        self.data
            .project_files(project_id)
            .join("ticket-docs")
            .join(docs::slug(key))
    }

    /// The key of the ticket agent `agent_id` works on in this project (in progress or to test).
    pub(crate) fn ticket_key_of_agent(&self, project_id: &str, agent_id: &str) -> Option<String> {
        self.tickets
            .read()
            .iter()
            .find(|t| {
                t.agent_id.as_deref() == Some(agent_id)
                    && t.project_id == project_id
                    && matches!(t.column, Column::Doing | Column::Review)
            })
            .map(|t| t.key.clone())
    }

    /// The docs ticket `key`'s agent works on, on the fixed branch whose space is `fixed`: its copy,
    /// taken from the branch the first time.
    pub(crate) fn docs_ticket_space(
        &self,
        project_id: &str,
        key: &str,
        fixed: Space,
    ) -> Result<Space> {
        let dir = self.ticket_docs_dir(project_id, key);
        let mut work = Space::new(dir.join("work"), fixed.text_dir.clone());
        work.documents_root = Some(fixed.root.clone());
        if !work.root.join(ROOT).is_dir() {
            seed(&fixed, &Space::new(dir.join("base"), None))?;
            seed(&fixed, &work)?;
        }
        Ok(work)
    }

    /// Ticket `key` validated: its copy poured on the branch of the docs, then dropped. None when it
    /// has none (the current branch, or nothing written).
    pub async fn docs_pour_ticket(&self, project_id: &str, key: &str) -> Result<Option<Poured>> {
        let dir = self.ticket_docs_dir(project_id, key);
        if !dir.join("work").join(ROOT).is_dir() {
            return Ok(None);
        }
        let into = self.docs_space(project_id, None).await?;
        let poured = pour(
            &Space::new(dir.join("work"), None),
            &Space::new(dir.join("base"), None),
            &into,
        )?;
        let _ = std::fs::remove_dir_all(&dir);
        Ok(Some(poured))
    }

    /// Ticket `key` deleted: its copy goes with it.
    pub(crate) fn docs_drop_ticket(&self, project_id: &str, key: &str) {
        let dir = self.ticket_docs_dir(project_id, key);
        if dir.is_dir() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
