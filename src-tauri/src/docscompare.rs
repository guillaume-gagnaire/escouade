//! The spec compared to the code (ESC-263, `docs/FACTORY.md` « La spec toujours à jour »): a short
//! agent, which reads the code without changing anything (`Weight::Reading`), says where a page
//! and the code disagree; what it found (`gaps`, `compared`) is kept in Escouade's folder, not in
//! the page (`Space::spec_check`, ESC-301: the page under git stays as it was, and the next
//! validation is not refused for it), and read over the page: the map of the subjects, the top of
//! the page and the agents' `read_spec_page` show it.
//!
//! - **After each ticket validated** (and merged): the pages it touches, those it wrote and those
//!   linked to the code it changed: by the index (the symbols of its files) when the project has
//!   it on, by the files' paths and names otherwise, and those that name its ticket.
//! - **On demand**: a page, or the whole spec (« Comparer au code »).
//!
//! « Mettre la spec à jour » on a gap opens a discussion with the spec writer (ESC-261,
//! `discussion.rs`), which proposes the page from the code; its pouring takes the gap away.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use serde::Serialize;
use tauri::Runtime;

use crate::core::Core;
use crate::docs::{Gap, Page, Space};
use crate::docsticket::Poured;
use crate::i18n::Lang;
use crate::model::{Project, UiEvent};

/// How long the short agent has for one page.
pub const LIMIT: Duration = Duration::from_secs(300);
/// « Comparer au code » on the whole spec: at most this many pages, those compared longest ago
/// first.
pub const MAX_PAGES: usize = 40;
/// After a ticket: at most this many pages, those it wrote first.
pub const MAX_TOUCHED: usize = 12;
/// What of a page's text the agent is given.
const MAX_BODY: usize = 30_000;
/// The files the index brings closer to a page, named to the agent.
const MAX_HINTS: usize = 10;

/// The pages being compared, by project, for the window (`UiEvent::DocsCompared`).
static COMPARING: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());
/// One comparison at a time: they read the same code, and each costs tokens.
static ONE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The pages of `project_id` being compared now (queued ones included).
pub fn comparing(project_id: &str) -> Vec<String> {
    COMPARING
        .lock()
        .map(|c| {
            c.iter()
                .filter(|(p, _)| p == project_id)
                .map(|(_, path)| path.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// What a comparison did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compared {
    pub pages: u32,
    pub gaps: u32,
    /// The pages that could not be compared, and the first reason.
    pub failed: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------- what the short agent is told, and what is read of its answer ----------

pub fn system(lang: Lang) -> String {
    tr_in!(
        lang,
        "Tu compares une page de la spécification d'un produit au code de ce dépôt, que tu lis sans rien modifier. \
         Relève seulement ce que le code fait autrement que la page ne le dit, ou ce que la page promet et que le code ne fait pas ; \
         pas le style, pas ce dont la page ne parle pas. Réponds par un bloc ```json qui contient \
         {{\"gaps\": [{{\"text\": \"<l'écart en une phrase : ce que dit la spec, ce que fait le code>\", \"code\": [\"<chemin/relatif/du/fichier>\"]}}]}}, \
         avec \"gaps\": [] quand la page dit vrai.",
        "You compare a page of a product's specification to the code of this repository, which you read without changing anything. \
         Report only what the code does otherwise than the page says, or what the page promises and the code does not do; \
         not the style, not what the page does not talk about. Answer with a ```json block that holds \
         {{\"gaps\": [{{\"text\": \"<the gap in one sentence: what the spec says, what the code does>\", \"code\": [\"<relative/path/of/the/file>\"]}}]}}, \
         with \"gaps\": [] when the page tells the truth."
    )
}

fn cut(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

pub fn prompt(lang: Lang, page: &Page, hints: &[String]) -> String {
    let mut p = format!(
        "<spec-page path=\"{}\" title=\"{}\">\n{}\n</spec-page>\n",
        page.path,
        page.title.replace('"', "'"),
        cut(&page.body, MAX_BODY)
    );
    if !hints.is_empty() {
        p.push_str(&format!(
            "<code-hints>\n{}\n</code-hints>\n",
            hints.join("\n")
        ));
    }
    p.push_str(&if hints.is_empty() {
        tr_in!(lang, "Compare cette page au code.", "Compare this page to the code.")
    } else {
        tr_in!(
            lang,
            "Compare cette page au code (les fichiers de <code-hints> sont ceux que l'index du code rapproche d'elle).",
            "Compare this page to the code (the files in <code-hints> are those the code index links to it)."
        )
    });
    p
}

/// The JSON of an answer: its ```json block, else the answer itself.
fn json_of(answer: &str) -> &str {
    if let Some(at) = answer.find("```json") {
        let rest = &answer[at + 7..];
        return rest.split("```").next().unwrap_or(rest).trim();
    }
    let t = answer.trim();
    match (t.find('{'), t.rfind('}')) {
        (Some(a), Some(b)) if a < b => &t[a..=b],
        _ => t,
    }
}

/// The gaps of an answer; None when it is not one (the page is then left as it was).
pub fn gaps_from_answer(answer: &str) -> Option<Vec<Gap>> {
    // Read loosely: a file that is not a text is left out, not the whole answer.
    let v: serde_json::Value = serde_json::from_str(json_of(answer)).ok()?;
    let gaps = v.get("gaps")?.as_array()?;
    Some(
        gaps.iter()
            .filter_map(|g| {
                let text = g.get("text")?.as_str()?.trim().to_string();
                let code = g
                    .get("code")
                    .and_then(|c| c.as_array())
                    .map(|c| {
                        c.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(Gap { text, code })
            })
            .filter(|g| !g.text.is_empty())
            .collect(),
    )
}

/// A gap names only files of `folder` that are there, relative, with `/`, and none the fence
/// keeps out (a private project's).
pub fn keep_code(gaps: &mut [Gap], folder: &Path, fenced: impl Fn(&str) -> bool) {
    for g in gaps {
        let mut seen = HashSet::new();
        g.code = g
            .code
            .iter()
            .map(|c| {
                c.trim()
                    .replace('\\', "/")
                    .trim_start_matches("./")
                    .to_string()
            })
            .filter(|c| {
                !c.is_empty()
                    && !c.starts_with('/')
                    && !c.contains(':')
                    && !c.split('/').any(|s| s == "..")
                    && folder.join(c).is_file()
                    && !fenced(&folder.join(c).to_string_lossy())
            })
            .filter(|c| seen.insert(c.clone()))
            .collect();
    }
}

// ---------- the pages a ticket touches ----------

/// Names too common to link a page to a file.
const COMMON: &[&str] = &[
    "index", "main", "mod", "lib", "utils", "util", "types", "test", "tests", "readme", "package",
    "config", "app",
];

fn has_word(text: &str, word: &str) -> bool {
    let is_id = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(is_id) && !after.is_some_and(is_id)
    })
}

/// The pages ticket `key` touches, in order: those it wrote (`written`), those that name it, then
/// those that name one of the files it changed (`changed`, its path or, a name not too common, its
/// file name) or one of their symbols (`symbols`, by the index). At most `MAX_TOUCHED`.
pub fn touched(
    pages: &[Page],
    key: &str,
    changed: &[String],
    symbols: &[String],
    written: &[String],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |p: &str| {
        if !out.iter().any(|x| x == p) && pages.iter().any(|x| x.path == p) {
            out.push(p.to_string());
        }
    };
    for w in written {
        add(w);
    }
    for p in pages {
        if p.tickets.iter().any(|t| t.eq_ignore_ascii_case(key)) {
            add(&p.path);
        }
    }
    let code: Vec<&String> = changed
        .iter()
        .filter(|c| !c.starts_with(".escouade/"))
        .collect();
    let names: Vec<&str> = code
        .iter()
        .filter_map(|c| c.rsplit('/').next())
        .filter(|n| {
            let stem = n.split('.').next().unwrap_or(n).to_lowercase();
            stem.chars().count() >= 5 && !COMMON.contains(&stem.as_str())
        })
        .collect();
    let symbols: Vec<&String> = symbols.iter().filter(|s| s.chars().count() >= 5).collect();
    for p in pages {
        let text = format!(
            "{}\n{}",
            p.body,
            p.gaps
                .iter()
                .flat_map(|g| g.code.iter().cloned())
                .collect::<Vec<_>>()
                .join("\n")
        );
        let linked = code.iter().any(|c| text.contains(c.as_str()))
            || names.iter().any(|n| has_word(&text, n))
            || symbols.iter().any(|s| has_word(&text, s));
        if linked {
            add(&p.path);
        }
    }
    out.truncate(MAX_TOUCHED);
    out
}

/// The files a ticket's worktree changed against where its branch left the target: committed or
/// not, new ones included. Empty when git cannot tell.
pub async fn changed_files(wt: &str, target: &str) -> Vec<String> {
    let Ok(base) = crate::git::text(wt, &["merge-base", target, "HEAD"]).await else {
        return Vec::new();
    };
    let base = base.trim().to_string();
    let mut out: Vec<String> = Vec::new();
    for args in [
        vec!["diff", "--name-only", "-z", base.as_str()],
        vec!["ls-files", "--others", "--exclude-standard", "-z"],
    ] {
        if let Ok(t) = crate::git::text(wt, &args).await {
            for f in t.split('\0').filter(|f| !f.is_empty()) {
                if !out.iter().any(|x| x == f) {
                    out.push(f.to_string());
                }
            }
        }
    }
    out
}

/// The page files among `changed` (`.escouade/spec/a/b.md` → `a/b`).
pub fn pages_of(changed: &[String]) -> Vec<String> {
    changed
        .iter()
        .filter_map(|f| f.strip_prefix(".escouade/spec/")?.strip_suffix(".md"))
        .map(str::to_string)
        .collect()
}

struct Marked {
    project: String,
    paths: Vec<String>,
}

impl Drop for Marked {
    fn drop(&mut self) {
        if let Ok(mut c) = COMPARING.lock() {
            c.retain(|(p, path)| !(p == &self.project && self.paths.contains(path)));
        }
    }
}

impl<R: Runtime> Core<R> {
    fn docs_tell_compared(&self, project_id: &str) {
        self.hub.emit(UiEvent::DocsCompared {
            project_id: project_id.to_string(),
            comparing: comparing(project_id),
        });
    }

    /// The folder whose code the spec is compared to: the project's (its first repository's).
    fn docs_code_folder(project: &Project) -> std::path::PathBuf {
        crate::docsbranch::base_of(project)
    }

    /// « Comparer au code »: `pages` (every page when None, at most `MAX_PAGES`, those compared
    /// longest ago first), one after the other. Refused when none could be compared.
    pub async fn docs_compare(
        self: &Arc<Self>,
        project_id: &str,
        pages: Option<Vec<String>>,
    ) -> Result<Compared> {
        let project = self.project(project_id)?;
        let space = self.docs_space(project_id, None).await?;
        let mut all = space.pages();
        let paths: Vec<String> = match pages {
            Some(p) => p
                .into_iter()
                .filter(|x| all.iter().any(|a| &a.path == x))
                .collect(),
            None => {
                all.sort_by(|a, b| a.compared.cmp(&b.compared));
                all.iter().take(MAX_PAGES).map(|p| p.path.clone()).collect()
            }
        };
        if paths.is_empty() {
            return Ok(Compared::default());
        }
        let _marked = {
            let mut c = COMPARING.lock().unwrap_or_else(|e| e.into_inner());
            for p in &paths {
                c.push((project.id.clone(), p.clone()));
            }
            Marked {
                project: project.id.clone(),
                paths: paths.clone(),
            }
        };
        self.docs_tell_compared(&project.id);
        let _one = ONE.lock().await;
        let mut out = Compared::default();
        for path in &paths {
            match self.docs_compare_page(&project, &space, path).await {
                Ok(n) => {
                    out.pages += 1;
                    out.gaps += n;
                }
                Err(e) => {
                    log::warn!("docs of {}: {path} not compared: {e:#}", project.id);
                    out.failed += 1;
                    out.error
                        .get_or_insert_with(|| crate::tickets::error_line(&e));
                }
            }
            if let Ok(mut c) = COMPARING.lock() {
                if let Some(i) = c.iter().position(|(p, x)| p == &project.id && x == path) {
                    c.remove(i);
                }
            }
            self.docs_tell_compared(&project.id);
            self.docs_changed(&project.id).await;
        }
        if out.pages == 0 {
            bail!(out.error.unwrap_or_default());
        }
        Ok(out)
    }

    /// One page compared: what the short agent found written on it, with the time. Its gaps.
    async fn docs_compare_page(
        self: &Arc<Self>,
        project: &Project,
        space: &Space,
        path: &str,
    ) -> Result<u32> {
        let page = space.spec_read(path)?;
        let lang = self.lang().claude;
        let folder = Self::docs_code_folder(project);
        let hints = self.docs_hints(project, &page).await;
        let answer = self
            .read_folder(
                &project.id,
                &folder,
                &system(lang),
                &prompt(lang, &page, &hints),
                LIMIT,
            )
            .await?;
        let Some(mut gaps) = gaps_from_answer(&answer) else {
            bail!(tr!(
                "la réponse de l’agent n’est pas une liste d’écarts",
                "the agent’s answer is not a list of gaps"
            ));
        };
        let fence = self.project_fence(&project.id);
        keep_code(&mut gaps, &folder, |p| fence.covers(p));
        let n = gaps.len() as u32;
        let when = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();
        // What the comparison finds is its gaps, not the page: `docs.gaps` says it. Kept in
        // Escouade's folder, the page left as it is (ESC-301).
        let quiet = Space {
            facts: None,
            ..space.clone()
        };
        quiet.spec_check(path, when, gaps)?;
        self.facts.emit(
            crate::facts::Body::DocsGaps {
                page: path.to_string(),
                gaps: n as usize,
            },
            crate::facts::About::project(&project.id),
        );
        Ok(n)
    }

    /// The files the index links to a page (none when the project has it off).
    async fn docs_hints(self: &Arc<Self>, project: &Project, page: &Page) -> Vec<String> {
        if !self.code_index_on(project) {
            return Vec::new();
        }
        let Ok(reader) = self.code_index(&project.id, None).await else {
            return Vec::new();
        };
        let task = format!("{} {}", page.title, cut(&page.body, 400));
        let Ok(a) = reader.task_context(&task, MAX_HINTS) else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for p in a
            .result
            .entry_points
            .iter()
            .map(|h| &h.path)
            .chain(a.result.snippets.iter().map(|s| &s.path))
        {
            if !out.contains(p) && out.len() < MAX_HINTS {
                out.push(p.clone());
            }
        }
        out
    }

    /// The symbols of the files `changed`, by the index (none when the project has it off).
    async fn docs_symbols_of(
        self: &Arc<Self>,
        project: &Project,
        changed: &[String],
    ) -> Vec<String> {
        if !self.code_index_on(project) {
            return Vec::new();
        }
        let Ok(reader) = self.code_index(&project.id, None).await else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for f in changed
            .iter()
            .filter(|f| !f.starts_with(".escouade/"))
            .take(30)
        {
            let Ok(a) = reader.impact(f, 1, 40) else {
                continue;
            };
            for t in a.result.targets.iter().filter(|t| t.kind != "heading") {
                if !out.contains(&t.name) {
                    out.push(t.name.clone());
                }
            }
        }
        out
    }

    /// Ticket `key` validated: the pages it touches compared to the code, in the background.
    pub(crate) fn docs_after_ticket(
        self: &Arc<Self>,
        project_id: &str,
        key: &str,
        changed: Vec<String>,
        poured: Option<Poured>,
    ) {
        let me = self.clone();
        let (id, key) = (project_id.to_string(), key.to_string());
        tauri::async_runtime::spawn(async move {
            let Ok(project) = me.project(&id) else { return };
            let Ok(space) = me.docs_space(&id, None).await else {
                return;
            };
            let pages = space.pages();
            if pages.is_empty() {
                return;
            }
            let written = match &poured {
                Some(p) => p.pages.clone(),
                None => pages_of(&changed),
            };
            let symbols = me.docs_symbols_of(&project, &changed).await;
            let paths = touched(&pages, &key, &changed, &symbols, &written);
            if paths.is_empty() {
                return;
            }
            log::info!("ticket {key}: comparing {paths:?} to the code");
            if let Err(e) = me.docs_compare(&id, Some(paths)).await {
                log::warn!("ticket {key}: its pages not compared: {e:#}");
            }
        });
    }
}
