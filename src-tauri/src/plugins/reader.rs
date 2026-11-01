//! The reader of workflows a plugin brings (ESC-200, choice 158 of `UX-NOTES.md`;
//! `docs/PLUGINS.md` §4.5): a plugin that learns Escouade a workflow it does not read by itself
//! (a framework of its own, a team's convention). Escouade gives it the project's files the
//! plugin declared (`contributes.workflowReaders[].files`), by `workflow/read`; it answers the
//! graph of the workflow (steps, transitions, ways back), and what follows is the same as for a
//! workflow Escouade read: the tracking of the agents, the tab, the `workflow.*` facts, the MCP
//! tools.
//!
//! Nothing here reads anything the plugin was not given: the files are those its globs match,
//! under the project's folder, text, and of a bounded weight and number.

// Nothing in the app asks a reader yet: the registry reads the manifests with INT-6, and the tests
// play this reader until then (as for the adapters of the sources and the hosts).
#![allow(dead_code)]

use super::adapters::Line;
use super::runtime::Runtime;
use crate::workflow::{Kind, Scan, Step, StepAgent, Transition, Workflow};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

/// The most files a reader is given, and the heaviest one, in bytes.
pub const MAX_FILES: usize = 200;
pub const MAX_FILE_BYTES: u64 = 256 * 1024;
/// The most workflows a reader may answer, and steps in each.
const MAX_WORKFLOWS: usize = 50;
const MAX_STEPS: usize = 200;

/// A file of the project, as a reader is given it: its path from the project's folder, with `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct File {
    pub path: String,
    pub text: String,
}

/// The files of the project that `patterns` match (globs from the project's folder and from each
/// repository's, for a project of several): the text ones, under `MAX_FILE_BYTES`, at most
/// `MAX_FILES`, in the order of their paths. A pattern that is absolute or leaves the project finds
/// nothing; nor does a link (a file, or a folder on the way, that leads out of the project: the
/// real path must stay under it), nor anything under `.git` (its config may hold a remote's
/// credentials).
pub fn files_of(root: &Path, repos: &[String], patterns: &[String]) -> Vec<File> {
    let Ok(real_root) = std::fs::canonicalize(root) else {
        return Vec::new();
    };
    let mut found: BTreeMap<String, File> = BTreeMap::new();
    for base in std::iter::once("").chain(repos.iter().map(String::as_str)) {
        for pattern in patterns {
            let leaves = pattern.is_empty()
                || pattern.starts_with(['/', '\\'])
                || pattern.contains(':')
                || pattern.split(['/', '\\']).any(|p| p == "..");
            if leaves {
                continue;
            }
            let full = root.join(base).join(pattern);
            let Ok(paths) = glob::glob(&full.to_string_lossy()) else {
                continue;
            };
            for path in paths.flatten() {
                // The entry itself, never what a link points to.
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(root) else {
                    continue;
                };
                if rel.components().any(|c| c.as_os_str() == ".git") {
                    continue;
                }
                // A folder on the way may be a link: the file must really be in the project.
                match std::fs::canonicalize(&path) {
                    Ok(real) if real.starts_with(&real_root) => {}
                    _ => continue,
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let rel = rel.to_string_lossy().replace('\\', "/");
                found.insert(rel.clone(), File { path: rel, text });
            }
        }
    }
    found.into_values().take(MAX_FILES).collect()
}

#[derive(Deserialize)]
struct Answer {
    workflows: Vec<Found>,
}

/// What a reader answers for a workflow: the graph, without what only Escouade's own reading knows
/// (the doors, the paths of a harness' folders).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
    id: String,
    name: String,
    #[serde(default)]
    harness: Option<crate::model::EngineId>,
    #[serde(default)]
    repo: String,
    steps: Vec<FoundStep>,
    #[serde(default)]
    transitions: Vec<FoundTransition>,
    #[serde(default)]
    exit: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FoundStep {
    name: String,
    /// What signals it when typed: `/bmad:plan`.
    #[serde(default)]
    command: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    path: String,
    /// The sub-agents it launches, by their names.
    #[serde(default)]
    agents: Vec<String>,
    #[serde(default)]
    reads: Vec<String>,
    #[serde(default)]
    writes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FoundTransition {
    from: String,
    to: String,
    #[serde(default)]
    back: bool,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    max: Option<u32>,
}

/// The id a reader's workflow has in Escouade: the plugin's, so that it never takes the place of
/// one the app reads.
pub fn id_of(plugin: &str, id: &str) -> String {
    format!("plugin:{plugin}:{id}")
}

/// A workflow of a reader as Escouade follows it. What does not hold (no name, no step, two steps
/// of one name, a transition to a step that is not there) is refused whole, with why.
fn workflow_of(plugin: &str, w: Found) -> Result<Workflow> {
    let id = w.id.clone();
    let refuse = |why: String| {
        anyhow!(
            "{}",
            tr!(
                "Workflow « {id} » refusé : {why}",
                "Workflow \"{id}\" refused: {why}"
            )
        )
    };
    if w.id.trim().is_empty() || w.name.trim().is_empty() {
        return Err(anyhow!(
            "{}",
            tr!(
                "Un workflow du lecteur n'a pas d'id ou pas de nom.",
                "A workflow of the reader has no id or no name."
            )
        ));
    }
    if w.steps.is_empty() || w.steps.len() > MAX_STEPS {
        return Err(refuse(tr!(
            "il faut de 1 à 200 étapes.",
            "it needs from 1 to 200 steps."
        )));
    }
    let mut names = BTreeSet::new();
    for s in &w.steps {
        if s.name.trim().is_empty() || !names.insert(s.name.as_str()) {
            return Err(refuse(tr!(
                "une étape sans nom, ou deux étapes du même nom.",
                "a step without a name, or two steps of one name."
            )));
        }
    }
    for t in &w.transitions {
        if !names.contains(t.from.as_str()) || !names.contains(t.to.as_str()) {
            let (from, to) = (&t.from, &t.to);
            return Err(refuse(tr!(
                "la transition {from} → {to} nomme une étape qui n'existe pas.",
                "the transition {from} → {to} names a step that does not exist."
            )));
        }
    }
    Ok(Workflow {
        id: id_of(plugin, &w.id),
        name: w.name,
        kind: Kind::Cycle,
        harness: w.harness,
        repo: w.repo,
        doors: Vec::new(),
        steps: w
            .steps
            .into_iter()
            .enumerate()
            .map(|(order, s)| Step {
                name: s.name,
                command: s.command,
                order,
                description: s.description,
                path: s.path,
                agents: s
                    .agents
                    .into_iter()
                    .map(|name| StepAgent {
                        name,
                        path: None,
                        description: String::new(),
                    })
                    .collect(),
                reads: s.reads,
                writes: s.writes,
                guessed: false,
                also: Vec::new(),
            })
            .collect(),
        transitions: w
            .transitions
            .into_iter()
            .map(|t| Transition {
                from: t.from,
                to: t.to,
                back: t.back,
                condition: t.condition,
                max: t.max,
            })
            .collect(),
        state_files: Vec::new(),
        exit: w.exit,
        prompt_loop: None,
        refined: None,
    })
}

/// A reader of workflows served by a plugin (`contributes.workflowReaders`).
pub struct ProcessWorkflowReader {
    line: Line,
    reader: String,
    files: Vec<String>,
}

impl ProcessWorkflowReader {
    /// `files`: the globs the manifest declares for this reader.
    pub fn new(rt: Arc<Runtime>, plugin: &str, reader: &str, files: &[String]) -> Self {
        Self {
            line: Line::new(rt, plugin),
            reader: reader.to_string(),
            files: files.to_vec(),
        }
    }

    /// The workflows the plugin reads in the project at `root` (of `repos`, for a project of
    /// several). A plugin that is down is a `PluginDown` (what was read before stays); an answer
    /// whose graph does not hold is refused whole.
    pub async fn read(&self, root: &Path, repos: &[String]) -> Result<Vec<Workflow>> {
        let files = files_of(root, repos, &self.files);
        let answer: Answer = self
            .line
            .call(
                "workflow/read",
                "workflow/read",
                json!({
                    "reader": self.reader,
                    "root": root.to_string_lossy(),
                    "repos": repos,
                    "files": files,
                }),
            )
            .await?;
        if answer.workflows.len() > MAX_WORKFLOWS {
            let reader = &self.reader;
            return Err(anyhow!(
                "{}",
                tr!(
                    "Le lecteur {reader} rend plus de 50 workflows.",
                    "The reader {reader} answers more than 50 workflows."
                )
            ));
        }
        answer
            .workflows
            .into_iter()
            .map(|w| workflow_of(&self.line.plugin, w))
            .collect()
    }
}

/// `scan` with what readers found: a workflow of the app's own reading stays as it is (a reader's
/// id is its plugin's), and one a reader read before is replaced by its new reading.
pub fn with_read(scan: &mut Scan, found: Vec<Workflow>) {
    for w in found {
        match scan.workflows.iter_mut().find(|x| x.id == w.id) {
            Some(x) => *x = w,
            None => scan.workflows.push(w),
        }
    }
}
