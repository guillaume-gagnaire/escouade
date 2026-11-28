//! The agents' pages (docs/AGENT-PAGES.md § 3, AP-2, ESC-673): the model and its keeping. The shelter
//! they are shown in is `agentpage` (AP-1), the templates `agentpage_templates` (AP-5), the tools
//! `mcp::pages`.
//!
//! - **A page** is `p-<n>`, never given twice, and belongs to the agent that showed it: to its project,
//!   and to its ticket if it has one. It has a title, a template, and its **versions**.
//! - **A version** is a frozen copy of what Escouade serves, made at the call: the document and the
//!   files it names (`Bundle`), written in `<données>/pages/<projet>/<p-n>/v<k>/`. It outlives the
//!   worktree. A page keeps its **10 last versions**.
//! - **The copy never leaves the worktree**: a file is opened within the agent's working folder (its
//!   path checked on the file opened, links followed), its relative files within its own folder and
//!   its subfolders alone; a hidden one (`.env`), a path out of it, a scheme or an absolute path is
//!   left out, said by name, never read.
//! - **Bounds**, held before anything is written: 20 pages per agent, a document of 2 MB, 64 files and
//!   10 MB per version, 500 MB for every page of the machine (the oldest page whose row is closed goes
//!   first). Each refusal says what to do.
//! - **The registry** is in the durable state (`PersistedState::pages`, `page_seq`, with its copies).
//!   A row of « Navigateurs » (`page:<p-n>`) lives for the app's run, like a browser's: `rows`.
//! - **Its life** follows its agent: kept while the agent is there, and the days of the box's closed
//!   files (« Garder les fichiers des demandes closes ») after it was archived or deleted (`pages_sweep`).
//!   A project gone takes its pages at once.
//! - **The log** says only « page p-12 v2 shown by agent e6e0… (review, 41 KB) »: never a title, a path
//!   or a content; the statistics only the template and a size bracket (`page_shown`).

use crate::agentpage::{self, Shelter};
use crate::agentpage_templates as templates;
use crate::core::Core;
use crate::devpage::Placement;
use crate::i18n::Lang;
use crate::model::{now_ms, UiEvent};
pub use escouade_model::agentpages::{AgentPage, PageVersion};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::Manager;

const MB: u64 = 1 << 20;
const DAY_MS: i64 = 24 * 3600 * 1000;
/// The pages an agent keeps.
pub(crate) const PER_AGENT: usize = 20;
/// The versions a page keeps.
pub(crate) const VERSIONS_KEPT: usize = 10;
/// A page's document.
pub(crate) const HTML_MAX: u64 = 2 * MB;
/// The files of a version, its document among them.
pub(crate) const FILES_MAX: usize = 64;
/// What a version weighs.
pub(crate) const VERSION_MAX: u64 = 10 * MB;
/// What every page of the machine weighs.
pub(crate) const ALL_MAX: u64 = 500 * MB;
/// The key of a page's row in « Navigateurs » starts with it: `page:p-12`.
pub(crate) const KEY_PREFIX: &str = "page:";
/// The templates `show_page` takes.
pub(crate) const TEMPLATES: [&str; 5] = ["review", "diagram", "compare", "html", "custom"];

/// The pages, and their rows open in « Navigateurs » for this run, with where the window shows each.
#[derive(Debug, Default)]
pub struct PageBox {
    pub list: Vec<AgentPage>,
    /// The number of the last page shown.
    pub seq: u64,
    pub rows: HashMap<String, Placement>,
    /// The rows the agent asked to wait (`open: quiet`): the window does not open them beside.
    pub quiet: HashSet<String>,
    /// The version each open row shows, when it is not the last (« v2 ▾ » of its bar, AP-3).
    pub shown: HashMap<String, u32>,
    /// What each page last proposed by `_/choose` (AP-4), checked against its request: the band of
    /// the window shows it, in the request's own words, and sends it on the user's gesture alone.
    pub proposals: HashMap<String, PageProposal>,
    /// The answer each open page was last told (its options; empty: none), to tell it once.
    pub told: HashMap<String, Vec<String>>,
}

/// A page's proposal, as the core kept it: its request, the options (among the request's), and the
/// note the page joined.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageProposal {
    pub ask: String,
    pub options: Vec<String>,
    pub note: Option<String>,
    pub at: i64,
}

/// A page as the window draws it (AP-3): its row of « Navigateurs », its bar, its card.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRow {
    pub page: String,
    pub project_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub ticket_id: Option<String>,
    pub title: String,
    pub template: String,
    /// Its versions kept, the oldest first: their number and when they were made.
    pub versions: Vec<RowVersion>,
    /// The version its row shows (the last unless the user went back to another).
    pub shown: u32,
    /// Its row is open in « Navigateurs ».
    pub open: bool,
    /// The agent asked its row to wait (`open: quiet`).
    pub quiet: bool,
    /// Its webview's last load finished.
    pub loaded: bool,
    /// The request it carries (`a-<n>`, AP-4).
    pub ask: Option<String>,
    /// What it proposed and the user has not sent yet.
    pub proposal: Option<PageProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RowVersion {
    pub n: u32,
    pub at: i64,
}

impl PageBox {
    /// What every page weighs.
    pub fn weight(&self) -> u64 {
        self.list.iter().map(AgentPage::weight).sum()
    }
}

/// The key of page `page`'s row.
pub(crate) fn key(page: &str) -> String {
    format!("{KEY_PREFIX}{page}")
}

/// The page a row's key names, if it is one (the target of `drive_browser`, AB-7).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn of_key(key: &str) -> Option<&str> {
    key.strip_prefix(KEY_PREFIX)
        .filter(|p| agentpage::is_page(p))
}

/// The folder of page `page` of project `project` in the data folder `root`.
pub(crate) fn folder(root: &Path, project: &str, page: &str) -> PathBuf {
    let project: String = project
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    root.join("pages").join(project).join(page)
}

/// The folder of version `n` of page `page`.
pub(crate) fn version_dir(root: &Path, project: &str, page: &str, n: u32) -> PathBuf {
    folder(root, project, page).join(format!("v{n}"))
}

/// The anchors of the version in `dir` (`…/v<k>`), kept beside it: `…/anchors-v<k>.json` (never served: the shelter
/// serves the version's folder alone).
pub(crate) fn anchors_file(dir: &Path) -> PathBuf {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    dir.with_file_name(format!("anchors-{name}.json"))
}

/// The title of a page, checked: one line, 1 to 120 characters.
pub(crate) fn checked_title(lang: Lang, title: &str) -> Result<String, String> {
    let t = title.trim();
    let n = t.chars().count();
    if n == 0 || n > templates::TITLE_MAX || t.chars().any(char::is_control) {
        return Err(tr_in!(
            lang,
            "Le titre (title) tient en une ligne de 1 à {max} caractères.",
            "The title (title) is one line of 1 to {max} characters.",
            max = templates::TITLE_MAX
        ));
    }
    Ok(t.to_string())
}

/// The size bracket of what a version weighs, for the statistics.
pub(crate) fn bracket(bytes: u64) -> &'static str {
    match bytes {
        b if b < 100 * 1024 => "small",
        b if b < MB => "medium",
        _ => "large",
    }
}

// ---------- the page of one's own (AP-7) ----------

/// Where the page of one's own of a project comes from (`show_page(template: "custom")`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CustomPage {
    /// Chosen on this machine (« Page à soi » of Réglages › Projets › Lancement): ahead of the file.
    Machine(PathBuf),
    /// Proposed by `pages.custom` of `.escouade.yml`, trusted: the document, and the repository it
    /// never leaves.
    Repo { file: PathBuf, root: PathBuf },
    /// Proposed by the file, waiting for the user's trust.
    Waiting,
    /// None.
    None,
}

/// « Page à soi » as the window saves it: trimmed, an `.html` document or none.
pub(crate) fn checked_custom_page(lang: Lang, raw: &str) -> Result<String, String> {
    let page = raw.trim();
    if page.is_empty() {
        return Ok(String::new());
    }
    let html = Path::new(page)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
    if !html {
        return Err(tr_in!(
            lang,
            "La page à soi est un document .html.",
            "A page of one's own is an .html document."
        ));
    }
    Ok(page.to_string())
}

/// The page of one's own of project `p`: **the machine's first**, then the file's once trusted (its
/// last trusted value stays while a new one waits), never one that leaves the repository.
pub(crate) fn custom_page_of(p: &crate::model::Project) -> CustomPage {
    let root = PathBuf::from(&p.path);
    let mine = p.custom_page.trim();
    if !mine.is_empty() {
        let path = Path::new(mine);
        return CustomPage::Machine(if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        });
    }
    let file = p.file_custom_page.trim();
    if !file.is_empty() && crate::projectfile::repo_page_problem(file).is_none() {
        return CustomPage::Repo {
            file: root.join(file.replace('/', std::path::MAIN_SEPARATOR_STR)),
            root,
        };
    }
    let waiting = p
        .shared
        .file
        .as_ref()
        .and_then(|f| f.pending.as_ref())
        .is_some_and(|x| {
            x.commands
                .get(crate::projectfile::CUSTOM_PAGE)
                .and_then(|v| v.as_str())
                .is_some_and(|v| !v.trim().is_empty())
        });
    if waiting {
        CustomPage::Waiting
    } else {
        CustomPage::None
    }
}

/// The refusal of `custom` without a page of one's own, saying what to take instead.
pub(crate) fn no_custom_page(lang: Lang, waiting: bool) -> String {
    if waiting {
        tr_in!(
            lang,
            "La page à soi que propose .escouade.yml (pages.custom) attend l’accord de l’utilisateur (Réglages › Projets › Équipe) : prends review, diagram, compare ou html.",
            "The page of one's own .escouade.yml proposes (pages.custom) waits for the user's trust (Settings › Projects › Team): take review, diagram, compare or html."
        )
    } else {
        tr_in!(
            lang,
            "L’utilisateur n’a pas réglé de page à soi pour ce projet : prends review, diagram, compare ou html.",
            "The user has set no page of their own for this project: take review, diagram, compare or html."
        )
    }
}

/// The bundle of the page of one's own `page`: its document and the files it names, from its own
/// folder alone; the file's never out of the repository (its path checked on the file opened, links
/// followed). Why it failed is not said: a path of the machine is no business of the agent's.
pub(crate) fn from_custom(
    lang: Lang,
    page: &CustomPage,
    open: Opener<'_>,
) -> Result<Bundle, String> {
    let gone = || {
        tr_in!(
            lang,
            "La page à soi de ce projet est introuvable, n’est pas un document .html ou sort du dépôt : l’utilisateur doit la régler à nouveau (Réglages › Projets › Lancement).",
            "This project's page of one's own is not there, is not an .html document or leaves the repository: the user must set it again (Settings › Projects › Launch)."
        )
    };
    match page {
        CustomPage::Machine(file) => {
            let home = file.parent().ok_or_else(gone)?;
            from_file(lang, file, home, open).map_err(|_| gone())
        }
        CustomPage::Repo { file, root } => from_file(lang, file, root, open).map_err(|_| gone()),
        CustomPage::Waiting => Err(no_custom_page(lang, true)),
        CustomPage::None => Err(no_custom_page(lang, false)),
    }
}

/// `escouade` (what `window.escouade` holds), completed by the page's number and version, written
/// into the document of `b` before any script of its own: read-only (`Object.freeze`, all the way
/// down, on a property that can be neither written nor redefined), and as a literal no text of it
/// can close (`<`, `>`, `&`, U+2028 and U+2029 escaped).
pub(crate) fn with_escouade(b: &mut Bundle, mut escouade: serde_json::Value, page: &str, n: u32) {
    escouade["page"] = serde_json::json!(page);
    escouade["version"] = serde_json::json!(n);
    let literal = escouade
        .to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    let script = format!(
        "<script>(()=>{{const f=o=>{{if(o&&typeof o===\"object\"){{for(const k of Object.keys(o))f(o[k]);Object.freeze(o)}}return o}};const e={literal};const p=self.fetch.bind(self);e.choose=(options,note)=>p(\"_/choose\",{{method:\"POST\",headers:{{\"Content-Type\":\"application/json\"}},body:JSON.stringify(note==null?{{options}}:{{options,note:String(note)}})}}).then(r=>r.ok,()=>false);Object.defineProperty(window,\"escouade\",{{value:f(e),writable:false,configurable:false,enumerable:true}})}})()</script>"
    );
    let Some((_, doc)) = b.files.iter_mut().find(|(r, _)| r == "index.html") else {
        return;
    };
    let html = String::from_utf8_lossy(doc).into_owned();
    let at = crate::agentpage::doctype_end(&html);
    let mut out = String::with_capacity(html.len() + script.len());
    out.push_str(&html[..at]);
    out.push_str(&script);
    out.push_str(&html[at..]);
    *doc = out.into_bytes();
}

// ---------- the copy ----------

/// What a version is made of: its files by their relative path (`index.html`, the document, first), and
/// what its document named that was not taken (out of its folder, hidden, not there).
#[derive(Debug, Default)]
pub struct Bundle {
    pub files: Vec<(String, Vec<u8>)>,
    pub left_out: Vec<String>,
}

impl Bundle {
    /// What its files weigh.
    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|(_, b)| b.len() as u64).sum()
    }

    fn has(&self, rel: &str) -> bool {
        self.files.iter().any(|(r, _)| r == rel)
    }

    /// Adds a file, within the bounds of a version.
    pub(crate) fn add(&mut self, lang: Lang, rel: String, bytes: Vec<u8>) -> Result<(), String> {
        if self.files.len() >= FILES_MAX {
            return Err(tr_in!(
                lang,
                "Une version tient en {FILES_MAX} fichiers au plus (le document compris) : allège la page.",
                "A version holds {FILES_MAX} files at most (the document among them): make the page lighter."
            ));
        }
        if self.bytes() + bytes.len() as u64 > VERSION_MAX {
            return Err(too_heavy(lang));
        }
        self.files.push((rel, bytes));
        Ok(())
    }

    fn leave_out(&mut self, raw: &str) {
        let raw: String = raw.chars().take(200).collect();
        if self.left_out.len() < 20 && !self.left_out.contains(&raw) {
            self.left_out.push(raw);
        }
    }
}

fn too_heavy(lang: Lang) -> String {
    tr_in!(
        lang,
        "Une version pèse 10 Mo au plus, ses fichiers compris : allège la page.",
        "A version weighs 10 MB at most, its files included: make the page lighter."
    )
}

fn html_too_big(lang: Lang) -> String {
    tr_in!(
        lang,
        "Le document d’une page fait 2 Mo au plus.",
        "A page's document is 2 MB at most."
    )
}

/// A page of the agent's own HTML, given whole: its document alone (what it names relatively has no
/// folder to come from).
pub(crate) fn from_html(lang: Lang, html: &str) -> Result<Bundle, String> {
    if html.trim().is_empty() {
        return Err(tr_in!(
            lang,
            "Le document (html) est vide.",
            "The document (html) is empty."
        ));
    }
    if html.len() as u64 > HTML_MAX {
        return Err(html_too_big(lang));
    }
    let mut b = Bundle::default();
    b.add(lang, "index.html".into(), html.as_bytes().to_vec())?;
    Ok(b)
}

/// Opens `path` (absolute) as a file within `within`: the file opened is checked to be there (links
/// followed); its length comes with it. What the tools of the MCP server use (`mcp::images`).
pub(crate) type Opener<'a> = &'a dyn Fn(&Path, &Path) -> Result<(std::fs::File, u64), String>;

/// The opener of the MCP server: `open_within`.
pub(crate) fn open_within(path: &Path, within: &Path) -> Result<(std::fs::File, u64), String> {
    crate::mcp::open_within(&path.to_string_lossy(), Some(within))
}

fn read_up_to(file: std::fs::File, max: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The page of the agent's own HTML file `file` (absolute, within `home`, its working folder), and the
/// files it names relatively, taken from **its own folder and its subfolders alone**: from a document
/// (`src`, `href`, `poster`, `srcset`, `url(…)`), a style sheet (`url(…)`, `@import`) or a script
/// (`import … from "./…"`), each in turn. A name that leaves the folder, a hidden file (`.env`), one
/// that is not there or not a file is left out, never read.
pub(crate) fn from_file(
    lang: Lang,
    file: &Path,
    home: &Path,
    open: Opener<'_>,
) -> Result<Bundle, String> {
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    if !matches!(ext.as_deref(), Some("html" | "htm")) {
        return Err(tr_in!(
            lang,
            "Le fichier (file) d’une page est un document .html.",
            "A page's file (file) is an .html document."
        ));
    }
    let (f, len) = open(file, home)?;
    if len > HTML_MAX {
        return Err(html_too_big(lang));
    }
    let html = read_up_to(f, HTML_MAX)?;
    if html.len() as u64 > HTML_MAX {
        return Err(html_too_big(lang));
    }
    let base = file.parent().ok_or_else(|| "no folder".to_string())?;
    let mut b = Bundle::default();
    let mut queue: VecDeque<String> = VecDeque::new();
    let own = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    for raw in references(&String::from_utf8_lossy(&html), Kind::Html) {
        enqueue(&mut b, &mut queue, "", &raw, &own);
    }
    b.add(lang, "index.html".into(), html)?;
    let mut seen: HashSet<String> = HashSet::new();
    while let Some(rel) = queue.pop_front() {
        if !seen.insert(rel.clone()) || b.has(&rel) {
            continue;
        }
        let full = base.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        let Ok((f, len)) = open(&full, base) else {
            b.leave_out(&rel);
            continue;
        };
        if b.bytes() + len > VERSION_MAX {
            return Err(too_heavy(lang));
        }
        let bytes = read_up_to(f, VERSION_MAX)?;
        let kind = Kind::of(&rel);
        if let Some(kind) = kind {
            let dir = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            for raw in references(&String::from_utf8_lossy(&bytes), kind) {
                enqueue(&mut b, &mut queue, dir, &raw, &own);
            }
        }
        b.add(lang, rel, bytes)?;
    }
    Ok(b)
}

fn enqueue(b: &mut Bundle, queue: &mut VecDeque<String>, dir: &str, raw: &str, own: &str) {
    match resolve(dir, raw) {
        Resolved::Ignored => {}
        Resolved::Out => b.leave_out(raw),
        // The document names itself: it is `index.html` now.
        Resolved::In(rel) if dir.is_empty() && rel == own => {}
        Resolved::In(rel) => queue.push_back(rel),
    }
}

/// The images a template's data names (`compare`), read within the agent's working folder `home` and
/// checked by their content (never an SVG), beside its document.
pub(crate) fn add_images(
    lang: Lang,
    b: &mut Bundle,
    images: &[String],
    home: Option<&Path>,
    open: Opener<'_>,
) -> Result<(), String> {
    for raw in images {
        let outside = || {
            tr_in!(
                lang,
                "L’image « {raw} » n’est pas un fichier de ton dossier de travail : donne un chemin relatif à lui.",
                "The image “{raw}” is not a file of your working folder: give a path relative to it."
            )
        };
        let Resolved::In(rel) = resolve("", raw) else {
            return Err(outside());
        };
        if b.has(&rel) {
            continue;
        }
        let Some(home) = home else {
            return Err(outside());
        };
        let full = home.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        let (f, len) = open(&full, home).map_err(|_| outside())?;
        if b.bytes() + len > VERSION_MAX {
            return Err(too_heavy(lang));
        }
        let bytes = read_up_to(f, VERSION_MAX)?;
        let (kind, _) = crate::asks::sniff(&bytes, &rel);
        if kind != crate::asks::AskFileKind::Image {
            return Err(tr_in!(
                lang,
                "« {raw} » n’est pas une image (PNG, JPEG, GIF, WebP ou AVIF, lue par son contenu ; jamais un SVG).",
                "“{raw}” is not an image (PNG, JPEG, GIF, WebP or AVIF, read by its content; never an SVG)."
            ));
        }
        b.add(lang, rel, bytes)?;
    }
    Ok(())
}

/// What a name a file gives leads to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Resolved {
    /// Not a file of the folder: an anchor, an address with a scheme, an absolute path.
    Ignored,
    /// A file that would leave the folder, a hidden one, or a name of Escouade's: left out.
    Out,
    /// The file, by its path relative to the folder.
    In(String),
}

/// What `raw`, named by a file of the folder `dir` (relative to the folder, `""` for its top), leads to.
pub(crate) fn resolve(dir: &str, raw: &str) -> Resolved {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.starts_with('#')
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.starts_with('?')
    {
        return Resolved::Ignored;
    }
    // A scheme (`https:`, `data:`, `C:`…) before the first slash.
    let head = raw.split(['/', '\\']).next().unwrap_or("");
    if head.contains(':') {
        return Resolved::Ignored;
    }
    let path = raw.split(['?', '#']).next().unwrap_or("");
    let Some(path) = agentpage::decode(path) else {
        return Resolved::Out;
    };
    let mut parts: Vec<String> = dir
        .split('/')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Resolved::Out;
                }
            }
            p if p.starts_with('.')
                || p.contains(['\\', ':'])
                || p.chars().any(char::is_control) =>
            {
                return Resolved::Out
            }
            p => parts.push(p.to_string()),
        }
    }
    if parts.is_empty() {
        return Resolved::Ignored;
    }
    // Escouade's own names in a page: its channel, its templates' files, its document.
    if matches!(parts[0].as_str(), "_" | "_t") || (parts.len() == 1 && parts[0] == "index.html") {
        return Resolved::Out;
    }
    Resolved::In(parts.join("/"))
}

/// The kind of a file whose names are followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Html,
    Css,
    Js,
}

impl Kind {
    fn of(rel: &str) -> Option<Kind> {
        let ext = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
        match ext.as_str() {
            "html" | "htm" => Some(Kind::Html),
            "css" => Some(Kind::Css),
            "js" | "mjs" => Some(Kind::Js),
            _ => None,
        }
    }
}

/// The names a file gives of others. Wide rather than exact: a name found that is no file of the folder
/// is simply not copied.
fn references(text: &str, kind: Kind) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    if kind == Kind::Html {
        for attr in ["src", "href", "poster", "srcset"] {
            let mut from = 0;
            while let Some(found) = lower[from..].find(attr) {
                let at = from + found;
                from = at + attr.len();
                let before = lower[..at].chars().next_back();
                if !before.is_some_and(|c| c.is_ascii_whitespace() || c == '"' || c == '\'') {
                    continue;
                }
                let rest = lower[from..].trim_start();
                if !rest.starts_with('=') {
                    continue;
                }
                let start = lower.len() - rest.len() + 1;
                let Some(value) = value_at(text, start) else {
                    continue;
                };
                if attr == "srcset" {
                    out.extend(
                        value
                            .split(',')
                            .filter_map(|c| c.split_whitespace().next())
                            .map(str::to_string),
                    );
                } else {
                    out.push(value);
                }
            }
        }
    }
    if kind != Kind::Js {
        let mut from = 0;
        while let Some(found) = lower[from..].find("url(") {
            let at = from + found + 4;
            from = at;
            let end = lower[at..].find(')').map(|e| at + e).unwrap_or(lower.len());
            let v = text[at..end].trim().trim_matches(['"', '\'']).trim();
            out.push(v.to_string());
        }
        let mut from = 0;
        while let Some(found) = lower[from..].find("@import") {
            let at = from + found + 7;
            from = at;
            if let Some(v) =
                value_at(text, at).filter(|v| !v.to_ascii_lowercase().starts_with("url("))
            {
                out.push(v);
            }
        }
    }
    if kind == Kind::Js {
        for word in ["from", "import"] {
            let mut from = 0;
            while let Some(found) = lower[from..].find(word) {
                let at = from + found + word.len();
                from = at;
                let rest = text[at..].trim_start().trim_start_matches('(').trim_start();
                let start = text.len() - rest.len();
                if let Some(v) =
                    value_at(text, start).filter(|v| v.starts_with("./") || v.starts_with("../"))
                {
                    out.push(v);
                }
            }
        }
    }
    out
}

/// The value at `at` of `text`, after blanks: quoted (`"…"`, `'…'`), or up to a blank, `>` or `;`.
fn value_at(text: &str, at: usize) -> Option<String> {
    let rest = text.get(at..)?.trim_start();
    let mut chars = rest.chars();
    match chars.next()? {
        q @ ('"' | '\'') => {
            let inner = &rest[1..];
            let end = inner.find(q)?;
            Some(inner[..end].to_string())
        }
        _ => {
            let end = rest
                .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == ';')
                .unwrap_or(rest.len());
            Some(rest[..end].to_string()).filter(|v| !v.is_empty())
        }
    }
}

/// Writes `b` as the version folder `dir`: in a folder of its own first, then put in place, so that a
/// version is whole or not there.
fn write_version(dir: &Path, b: &Bundle) -> std::io::Result<()> {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let part = dir.with_file_name(format!("{name}.part-{}", uuid::Uuid::new_v4().simple()));
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(&part)?;
        for (rel, bytes) in &b.files {
            let at = part.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = at.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(at, bytes)?;
        }
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        std::fs::rename(&part, dir)
    };
    let done = write();
    if done.is_err() {
        let _ = std::fs::remove_dir_all(&part);
    }
    done
}

// ---------- the core ----------

/// Who shows a page.
#[derive(Debug, Clone)]
pub(crate) struct Author {
    pub agent: String,
    pub name: String,
    pub project_id: String,
    pub ticket_id: Option<String>,
}

/// A page to show, or a version of one.
#[derive(Debug)]
pub(crate) struct Show {
    pub title: String,
    pub template: &'static str,
    /// A page of the agent's, for a new version of it.
    pub page: Option<String>,
    pub bundle: Bundle,
    /// The anchors of a template's page, by its keys (`drive_browser`, ESC-679): kept beside the version.
    pub anchors: Vec<templates::Anchor>,
    /// `open: quiet`: its row waits, even when the user looks at the agent.
    pub quiet: bool,
    /// For the page of one's own (`custom`, AP-7): what `window.escouade` holds but its page and
    /// version, which `page_show` adds once it knows them (`with_escouade`).
    pub escouade: Option<serde_json::Value>,
    /// The request it now carries (`ask` of `show_page`, AP-4), deposited by the same call; none
    /// keeps the one it had.
    pub ask: Option<String>,
}

/// What `show_page` answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shown {
    pub page: String,
    pub version: u32,
    pub bytes: u64,
    pub files: u32,
    pub left_out: Vec<String>,
}

/// What `get_page` answers of a page: the page, and where its row is (none when it is closed).
#[derive(Debug, Clone)]
pub(crate) struct Seen {
    pub page: AgentPage,
    pub row: Option<Placement>,
    pub loaded: bool,
}

fn not_yours(lang: Lang, page: &str, yours: &[String]) -> String {
    if yours.is_empty() {
        tr_in!(
            lang,
            "Aucune page « {page} » parmi les tiennes : tu n’en as montré aucune (show_page).",
            "No page “{page}” among yours: you have shown none (show_page)."
        )
    } else {
        let list = yours.join(", ");
        tr_in!(
            lang,
            "Aucune page « {page} » parmi les tiennes. Tes pages : {list}",
            "No page “{page}” among yours. Your pages: {list}"
        )
    }
}

impl Core {
    /// « Pages des agents » is on. It stands under the MCP server: off, no tool is there at all, and
    /// the pages' rows closed when it went off (`save_settings`).
    pub(crate) fn pages_on(&self) -> bool {
        // Without a window (`escouade-server`, ESC-701), no page can be shown: never on there.
        cfg!(feature = "desktop") && self.settings.read().agent_pages
    }

    /// The pages of agent `agent`, their numbers, the last shown first.
    fn pages_of(&self, agent: &str) -> Vec<String> {
        let b = self.pages.lock();
        let mut mine: Vec<&AgentPage> = b.list.iter().filter(|p| p.agent_id == agent).collect();
        mine.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
        mine.into_iter()
            .take(crate::mcp::CHOICES)
            .map(|p| p.id.clone())
            .collect()
    }

    /// Shows a page of `who`, or a version of one of its pages: bounded, copied, kept, served in its
    /// shelter and opened in its row. Not on the async runtime: it writes the copy.
    pub(crate) fn page_show(self: &Arc<Self>, who: &Author, show: Show) -> Result<Shown, String> {
        let lang = crate::i18n::ui();
        if !self.pages_on() {
            return Err(switched_off(lang));
        }
        let root = self.data.root().to_path_buf();
        let now = now_ms();
        let mut show = show;
        // `window.escouade` is written into the document once its number is known: its room is held.
        let extra = show
            .escouade
            .as_ref()
            .map_or(0, |e| e.to_string().len() as u64 + 512);
        let incoming = show.bundle.bytes() + extra;
        let (id, n, project, dropped, evicted, incoming) = {
            let mut b = self.pages.lock();
            let at = match &show.page {
                Some(asked) => {
                    let asked = asked.trim();
                    let found = b
                        .list
                        .iter()
                        .position(|p| p.id == asked && p.agent_id == who.agent);
                    if found.is_none() {
                        drop(b);
                        return Err(not_yours(lang, asked, &self.pages_of(&who.agent)));
                    }
                    found
                }
                None => {
                    let mine: Vec<&AgentPage> =
                        b.list.iter().filter(|p| p.agent_id == who.agent).collect();
                    if mine.len() >= PER_AGENT {
                        let last = mine
                            .iter()
                            .max_by_key(|p| p.updated_at)
                            .map(|p| p.id.clone())
                            .unwrap_or_default();
                        return Err(tr_in!(
                            lang,
                            "Tu as déjà {PER_AGENT} pages : mets à jour l’une d’elles (page: \"{last}\") au lieu d’en ouvrir une autre.",
                            "You already have {PER_AGENT} pages: update one of them (page: \"{last}\") rather than opening another."
                        ));
                    }
                    None
                }
            };
            // Room among every page of the machine: the oldest whose row is closed go first.
            let mut evicted: Vec<AgentPage> = Vec::new();
            let keep = at.map(|i| b.list[i].id.clone());
            while b.weight() + incoming > ALL_MAX {
                let rows = b.rows.clone();
                let oldest = b
                    .list
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| Some(&p.id) != keep.as_ref() && !rows.contains_key(&p.id))
                    .min_by_key(|(_, p)| p.updated_at)
                    .map(|(i, _)| i);
                let Some(i) = oldest else {
                    return Err(tr_in!(
                        lang,
                        "Les pages des agents de ce poste pèsent déjà 500 Mo : l’utilisateur doit en fermer. Allège ta page.",
                        "The agents' pages of this machine already weigh 500 MB: the user must close some. Make your page lighter."
                    ));
                };
                evicted.push(b.list.remove(i));
            }
            let existing = keep
                .as_ref()
                .and_then(|k| b.list.iter().position(|p| &p.id == k));
            let (id, n, project) = match existing {
                Some(i) => {
                    let p = &b.list[i];
                    (
                        p.id.clone(),
                        p.latest().map_or(1, |v| v.n + 1),
                        p.project_id.clone(),
                    )
                }
                None => {
                    b.seq += 1;
                    (format!("p-{}", b.seq), 1, who.project_id.clone())
                }
            };
            if let Some(e) = show.escouade.take() {
                with_escouade(&mut show.bundle, e, &id, n);
            }
            let incoming = show.bundle.bytes();
            let dir = version_dir(&root, &project, &id, n);
            write_version(&dir, &show.bundle).map_err(|e| {
                tr_in!(
                    lang,
                    "La copie de la page a échoué : {e}",
                    "Copying the page failed: {e}"
                )
            })?;
            if !show.anchors.is_empty() {
                if let Ok(bytes) = serde_json::to_vec(&show.anchors) {
                    let _ = std::fs::write(anchors_file(&dir), bytes);
                }
            }
            let version = PageVersion {
                n,
                at: now,
                bytes: incoming,
                files: show.bundle.files.len() as u32,
            };
            let mut dropped = Vec::new();
            match b.list.iter_mut().find(|p| p.id == id) {
                Some(p) => {
                    p.versions.push(version);
                    if show.ask.is_some() {
                        p.ask = show.ask.clone();
                    }
                    p.title = show.title.clone();
                    p.template = show.template.to_string();
                    p.updated_at = now;
                    while p.versions.len() > VERSIONS_KEPT {
                        let old = p.versions.remove(0);
                        dropped.push(version_dir(&root, &project, &id, old.n));
                    }
                }
                None => b.list.push(AgentPage {
                    id: id.clone(),
                    project_id: project.clone(),
                    agent_id: who.agent.clone(),
                    agent_name: who.name.clone(),
                    ticket_id: who.ticket_id.clone(),
                    title: show.title.clone(),
                    template: show.template.to_string(),
                    versions: vec![version],
                    ask: show.ask.clone(),
                    created_at: now,
                    updated_at: now,
                    orphaned_at: None,
                }),
            }
            b.rows.entry(id.clone()).or_insert(Placement::Hidden);
            // A new version shows at once, wherever the user went back to; it is told its answer
            // again once it has loaded, and what the old one proposed is gone.
            b.shown.remove(&id);
            b.told.remove(&id);
            b.proposals.remove(&id);
            if show.quiet {
                b.quiet.insert(id.clone());
            } else {
                b.quiet.remove(&id);
            }
            (id, n, project, dropped, evicted, incoming)
        };
        for dir in dropped {
            let _ = std::fs::remove_file(anchors_file(&dir));
            let _ = std::fs::remove_dir_all(dir);
        }
        for p in &evicted {
            log::info!("page {} erased: room for the pages", p.id);
            self.page_erase_files(&root, p);
        }
        self.request_save();
        let short: String = who.agent.chars().take(8).collect();
        log::info!(
            "page {id} v{n} shown by agent {short}… ({}, {} KB)",
            show.template,
            incoming.div_ceil(1024)
        );
        if let Some(template) = TEMPLATES.iter().find(|t| **t == show.template) {
            self.track(crate::telemetry::Event::PageShown {
                template,
                ask: show.ask.is_some(),
                size: bracket(incoming),
            });
        }
        self.page_serve(&id, &version_dir(&root, &project, &id, n));
        self.page_tell(&id, "shown");
        Ok(Shown {
            page: id,
            version: n,
            bytes: incoming,
            files: show.bundle.files.len() as u32,
            left_out: show.bundle.left_out,
        })
    }

    /// Page `page` of agent `agent`, and its row.
    pub(crate) fn page_get(&self, agent: &str, page: &str) -> Result<Seen, String> {
        let lang = crate::i18n::ui();
        if !self.pages_on() {
            return Err(switched_off(lang));
        }
        let page = page.trim();
        let found = {
            let b = self.pages.lock();
            b.list
                .iter()
                .find(|p| p.id == page && p.agent_id == agent)
                .cloned()
                .map(|p| {
                    let row = b.rows.get(&p.id).copied();
                    (p, row)
                })
        };
        let Some((p, row)) = found else {
            return Err(not_yours(lang, page, &self.pages_of(agent)));
        };
        let loaded = self
            .shelter()
            .and_then(|s| s.view(&p.id))
            .is_some_and(|v| v.loaded);
        Ok(Seen {
            page: p,
            row,
            loaded,
        })
    }

    /// Closes the row of page `page` of agent `agent`: the page stays kept. Whether it was open.
    pub(crate) fn page_close(&self, agent: &str, page: &str) -> Result<bool, String> {
        let seen = self.page_get(agent, page)?;
        Ok(self.page_close_row(&seen.page.id))
    }

    /// Closes the row of page `page`, whoever asks (its agent, or the user by its row's ×): the page
    /// stays kept, its card reopens it. Whether it was open.
    pub(crate) fn page_close_row(&self, page: &str) -> bool {
        let open = {
            let mut b = self.pages.lock();
            b.quiet.remove(page);
            b.shown.remove(page);
            b.rows.remove(page).is_some()
        };
        self.page_close_view(page);
        if open {
            log::info!("page {page} closed");
            self.page_tell(page, "closed");
        }
        open
    }

    /// Page `page` as the window draws it, its row open or not; none once it is no longer kept.
    pub(crate) fn page_row(&self, page: &str) -> Option<PageRow> {
        let loaded = self.page_loaded(page);
        let b = self.pages.lock();
        let p = b.list.iter().find(|p| p.id == page)?;
        Some(row_of(&b, p, loaded))
    }

    /// The pages whose row is open, in every project, in the order they were shown.
    pub(crate) fn page_open_rows(&self) -> Vec<PageRow> {
        let open: Vec<String> = {
            let b = self.pages.lock();
            b.list
                .iter()
                .filter(|p| b.rows.contains_key(&p.id))
                .map(|p| p.id.clone())
                .collect()
        };
        open.iter().filter_map(|p| self.page_row(p)).collect()
    }

    fn page_loaded(&self, page: &str) -> bool {
        self.shelter()
            .and_then(|s| s.view(page))
            .is_some_and(|v| v.loaded)
    }

    /// The user opens page `page` again (its card, after its row closed or a restart): its row, on its
    /// last version. Refused while the pages are off, or once it is no longer kept.
    pub(crate) fn page_reopen(self: &Arc<Self>, page: &str) -> Result<PageRow, String> {
        let lang = crate::i18n::ui();
        if !self.pages_on() {
            return Err(switched_off(lang));
        }
        let dir = {
            let mut b = self.pages.lock();
            let Some(p) = b.list.iter().find(|p| p.id == page) else {
                return Err(gone(lang, page));
            };
            let n = p.latest().map_or(1, |v| v.n);
            let dir = version_dir(self.data.root(), &p.project_id, &p.id, n);
            b.rows.entry(page.to_string()).or_insert(Placement::Hidden);
            b.quiet.remove(page);
            b.shown.remove(page);
            dir
        };
        log::info!("page {page} opened again by the user");
        self.page_serve(page, &dir);
        self.page_row(page).ok_or_else(|| gone(lang, page))
    }

    /// The row of page `page` shows its version `n` (« v2 ▾ » of its bar): served, and reloaded where
    /// it is. The agent's next version shows the last again.
    pub(crate) fn page_show_version(
        self: &Arc<Self>,
        page: &str,
        n: u32,
    ) -> Result<PageRow, String> {
        let lang = crate::i18n::ui();
        if !self.pages_on() {
            return Err(switched_off(lang));
        }
        let dir = {
            let mut b = self.pages.lock();
            if !b.rows.contains_key(page) {
                return Err(tr_in!(
                    lang,
                    "La page {page} n’est pas ouverte.",
                    "Page {page} is not open."
                ));
            }
            let Some(p) = b.list.iter().find(|p| p.id == page) else {
                return Err(gone(lang, page));
            };
            if !p.versions.iter().any(|v| v.n == n) {
                return Err(tr_in!(
                    lang,
                    "La version {n} de {page} n’est plus gardée.",
                    "Version {n} of {page} is no longer kept."
                ));
            }
            let last = p.latest().map(|v| v.n);
            let dir = version_dir(self.data.root(), &p.project_id, &p.id, n);
            if Some(n) == last {
                b.shown.remove(page);
            } else {
                b.shown.insert(page.to_string(), n);
            }
            dir
        };
        self.page_serve(page, &dir);
        self.page_row(page).ok_or_else(|| gone(lang, page))
    }

    /// The pages of project `project` whose row is open, in the order they were shown, with where the
    /// window shows each and whether its last load finished.
    pub(crate) fn page_rows(&self, project: &str) -> Vec<(AgentPage, Placement, bool)> {
        let shelter = self.shelter();
        let b = self.pages.lock();
        b.list
            .iter()
            .filter(|p| p.project_id == project)
            .filter_map(|p| {
                let place = *b.rows.get(&p.id)?;
                let loaded = shelter
                    .as_ref()
                    .and_then(|s| s.view(&p.id))
                    .is_some_and(|v| v.loaded);
                Some((p.clone(), place, loaded))
            })
            .collect()
    }

    /// Where the window shows the row of page `page` (`agentpage_place`).
    pub(crate) fn page_placed(&self, page: &str, placement: Placement) {
        if let Some(p) = self.pages.lock().rows.get_mut(page) {
            *p = placement;
        }
    }

    /// What `drive_browser` needs of page `page` of agent `agent` (ESC-679): its row, the version it shows, its
    /// versions and the anchors of the one shown (a template's; none for an html page).
    pub(crate) fn page_drive_info(&self, agent: &str, page: &str) -> Result<DriveInfo, String> {
        let seen = self.page_get(agent, page)?;
        let p = &seen.page;
        let latest = p.latest().map_or(1, |v| v.n);
        let shown = self
            .pages
            .lock()
            .shown
            .get(&p.id)
            .copied()
            .unwrap_or(latest);
        let root = self.data.root().to_path_buf();
        let anchors = std::fs::read(anchors_file(&version_dir(
            &root,
            &p.project_id,
            &p.id,
            shown,
        )))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<serde_json::Value>>(&b).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| {
            Some((
                a["key"].as_str()?.to_string(),
                a["selector"].as_str()?.to_string(),
            ))
        })
        .collect();
        Ok(DriveInfo {
            page: p.id.clone(),
            template: p.template.clone(),
            row: seen.row,
            shown,
            anchors,
        })
    }

    /// Page `page` of agent `agent` shows its version `n` (`drive_browser`, `version`): what « v2 ▾ » of its bar
    /// does, and the window reads its row again (`version`). A version it no longer keeps is refused with those it keeps.
    pub(crate) fn page_drive_version(
        self: &Arc<Self>,
        agent: &str,
        page: &str,
        n: u32,
    ) -> Result<(), String> {
        let lang = crate::i18n::ui();
        let seen = self.page_get(agent, page)?;
        let p = &seen.page;
        if !p.versions.iter().any(|v| v.n == n) {
            let kept = p
                .versions
                .iter()
                .map(|v| v.n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(tr_in!(
                lang,
                "La page {page} n’a pas de version {n} : ses versions gardées sont {kept}.",
                "Page {page} has no version {n}: its kept versions are {kept}.",
                page = p.id
            ));
        }
        self.page_show_version(&p.id, n)?;
        self.page_tell(&p.id, "version");
        Ok(())
    }

    /// « Pages des agents » or the MCP server turned off: every row closes; nothing is erased.
    pub(crate) fn pages_close_rows(&self) {
        let rows: Vec<String> = {
            let mut b = self.pages.lock();
            b.quiet.clear();
            b.shown.clear();
            b.rows.drain().map(|(k, _)| k).collect()
        };
        for page in rows {
            self.page_close_view(&page);
            self.page_tell(&page, "closed");
        }
    }

    /// What goes by itself: a page whose project went, at once; one whose agent was archived or deleted,
    /// after the days of the box's closed files.
    pub(crate) fn pages_sweep(self: &Arc<Self>) {
        let now = now_ms();
        let days = crate::asks::kept_days(self.settings.read().inbox_keep_days);
        let projects: HashSet<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let agents: HashMap<String, bool> = self
            .agents
            .read()
            .values()
            .map(|h| {
                let rt = h.lock();
                (rt.meta.id.clone(), rt.meta.archived)
            })
            .collect();
        let (erased, changed) = {
            let mut b = self.pages.lock();
            let mut changed = false;
            for p in b.list.iter_mut() {
                let here = agents.get(&p.agent_id).is_some_and(|archived| !archived);
                match (here, p.orphaned_at) {
                    (true, Some(_)) => {
                        p.orphaned_at = None;
                        changed = true;
                    }
                    (false, None) => {
                        p.orphaned_at = Some(now);
                        changed = true;
                    }
                    _ => {}
                }
            }
            let mut erased = Vec::new();
            let mut kept = Vec::with_capacity(b.list.len());
            for p in std::mem::take(&mut b.list) {
                let gone = !projects.contains(&p.project_id)
                    || p.orphaned_at.is_some_and(|t| t + days * DAY_MS <= now);
                if gone {
                    erased.push(p);
                } else {
                    kept.push(p);
                }
            }
            b.list = kept;
            for p in &erased {
                b.rows.remove(&p.id);
                b.quiet.remove(&p.id);
                b.shown.remove(&p.id);
            }
            let changed = changed || !erased.is_empty();
            (erased, changed)
        };
        let root = self.data.root().to_path_buf();
        for p in &erased {
            log::info!("page {} erased: its agent or project went", p.id);
            self.page_close_view(&p.id);
            self.page_erase_files(&root, p);
        }
        if changed {
            self.request_save();
        }
    }

    /// A page's files gone, and its shelter serving it no more.
    fn page_erase_files(&self, root: &Path, p: &AgentPage) {
        if let Some(s) = self.shelter() {
            s.remove(&p.id);
        }
        let _ = std::fs::remove_dir_all(folder(root, &p.project_id, &p.id));
        // Its row, if it had one, goes; its cards say it is no longer kept.
        self.page_tell(&p.id, "erased");
    }

    /// The row of page `page` is on screen (alone, beside or under something else).
    pub(crate) fn page_on_screen(&self, page: &str) -> bool {
        self.pages
            .lock()
            .rows
            .get(page)
            .is_some_and(|p| *p != Placement::Hidden)
    }

    /// The window has a shelter to show pages in (the tests have none).
    pub(crate) fn pages_shown(&self) -> bool {
        self.shelter().is_some()
    }

    /// The window is told what became of page `page`'s row.
    fn page_tell(&self, page: &str, phase: &'static str) {
        self.hub.emit(UiEvent::AgentPage {
            page: agentpage::PageEvent {
                page: page.to_string(),
                phase,
                url: String::new(),
                title: String::new(),
            },
        });
    }

    /// Page `page` served from `dir` in its shelter, and its webview opened (hidden, its row waiting) or
    /// reloaded. Without a shelter (the tests), nothing opens.
    fn page_serve(self: &Arc<Self>, page: &str, dir: &Path) {
        let Some(shelter) = self.shelter() else {
            return;
        };
        if let Err(e) = shelter.put(page, dir.to_path_buf()) {
            log::warn!("page {page} not served: {e}");
            return;
        }
        #[cfg(feature = "desktop")]
        self.page_open_view(page, &shelter);
    }

    /// The webview of page `page`, served by `shelter`, opened (hidden, its row waiting) or reloaded.
    #[cfg(feature = "desktop")]
    fn page_open_view(self: &Arc<Self>, page: &str, shelter: &Arc<Shelter>) {
        let dev = self
            .app()
            .and_then(|a| a.try_state::<crate::navigation::Home>())
            .and_then(|h| h.dev.clone());
        let own: crate::devpage::Own = Arc::new(move |u: &tauri::Url| {
            crate::navigation::destination(u, dev.as_ref()) == crate::navigation::Destination::App
        });
        let core = Arc::downgrade(self);
        let tell: agentpage::Tell = Arc::new(move |page| {
            if let Some(core) = core.upgrade() {
                // Loaded again (opened, reloaded, a version): it is told its answer once more.
                if page.phase == "finished" {
                    core.pages.lock().told.remove(&page.page);
                    core.page_tell_answer(&page.page);
                }
                core.hub.emit(UiEvent::AgentPage { page });
            }
        });
        // Without a window (a host of the remote API's tests), no webview opens.
        let Some(app) = self.app() else {
            return;
        };
        let opened = agentpage::open(
            app,
            shelter,
            agentpage::Open {
                page,
                bounds: None,
                visible: false,
                data_root: self.data.root(),
            },
            own,
            tell,
        );
        if let Err(e) = opened {
            log::warn!("page {page}: its webview did not open: {e}");
        }
    }

    /// A proposal of page `page` (`_/choose`, from its own webview: the shelter checked it), kept
    /// for the band when its request takes it (`checked_proposal`): true then. Nothing is answered:
    /// the band of the window sends it, on the user's gesture.
    pub(crate) fn page_propose(&self, page: &str, proposal: agentpage::Proposal) -> bool {
        if !self.pages_on() {
            return false;
        }
        // A page its agent is driving proposes nothing: a choice made by a gesture of the agent's (a declared tab of
        // its own page whose script proposes) is never the user's (ESC-679).
        let driven = self.browsers().is_some_and(|p| {
            p.drives.acted_lately(
                &key(page),
                crate::browserdrive::CHOICE_QUIET,
                std::time::Instant::now(),
            )
        });
        if driven {
            log::info!("page {page}: a proposal refused, made while its agent drove it");
            return false;
        }
        let Some(ask_id) = self.page_ask(page) else {
            return false;
        };
        let Ok(ask) = self.ask(&ask_id) else {
            return false;
        };
        let Some(kept) = checked_proposal(&ask, proposal, now_ms()) else {
            log::info!("page {page}: a proposal for {ask_id} refused");
            return false;
        };
        let changed = {
            let mut b = self.pages.lock();
            if !b.list.iter().any(|p| p.id == page) {
                return false;
            }
            let same = b
                .proposals
                .get(page)
                .is_some_and(|x| x.options == kept.options && x.note == kept.note);
            b.proposals.insert(page.to_string(), kept);
            !same
        };
        if changed {
            self.page_tell(page, "proposed");
        }
        true
    }

    /// The request page `page` carries, if any.
    pub(crate) fn page_ask(&self, page: &str) -> Option<String> {
        self.pages
            .lock()
            .list
            .iter()
            .find(|p| p.id == page)
            .and_then(|p| p.ask.clone())
    }

    /// « Annuler » of the band: what page `page` proposed is let go (the page may propose again).
    pub(crate) fn page_drop_proposal(&self, page: &str) -> bool {
        let gone = self.pages.lock().proposals.remove(page).is_some();
        if gone {
            self.page_tell(page, "proposed");
        }
        gone
    }

    /// The requests of the box changed: each page that carries one is told its answer (its options
    /// checked, the others frozen), or that it has none any more (the answer taken back); what it
    /// proposed goes once its request no longer waits.
    pub(crate) fn pages_asks_changed(&self) {
        let carried: Vec<(String, String)> = {
            let b = self.pages.lock();
            b.list
                .iter()
                .filter_map(|p| p.ask.clone().map(|a| (p.id.clone(), a)))
                .collect()
        };
        for (page, ask) in carried {
            let waiting = self.ask(&ask).is_ok_and(|a| a.state.waiting());
            if !waiting && self.pages.lock().proposals.remove(&page).is_some() {
                self.page_tell(&page, "proposed");
            }
            self.page_tell_answer(&page);
        }
    }

    /// Page `page`'s webview is told the answer of its request, once (`answer_script`): a fixed
    /// script, with nothing in it but the options' ids.
    pub(crate) fn page_tell_answer(&self, page: &str) {
        let Some(ask) = self.page_ask(page) else {
            return;
        };
        let options = self
            .ask(&ask)
            .ok()
            .map(|a| answered_options(&a))
            .unwrap_or_default();
        {
            let mut b = self.pages.lock();
            let before = b.told.get(page);
            let same = before == Some(&options) || (before.is_none() && options.is_empty());
            b.told.insert(page.to_string(), options.clone());
            if same {
                return;
            }
        }
        #[cfg(feature = "desktop")]
        if let Some(w) = self
            .app()
            .and_then(|a| a.get_webview(&agentpage::label(page)))
        {
            if let Err(e) = w.eval(answer_script(&options)) {
                log::warn!("page {page}: its answer not told: {e}");
            }
        }
    }

    /// The webview of page `page` closed, if it is open.
    fn page_close_view(&self, page: &str) {
        #[cfg(feature = "desktop")]
        if let Some(app) = self.app() {
            if let Some(s) = app.try_state::<Arc<Shelter>>() {
                agentpage::close(app, &s, page);
            }
        }
        // Its driving falls with its row (`drive_browser`, ESC-679).
        if let Some(p) = self.browsers() {
            p.drives.forget(&key(page));
        }
    }
}

/// The refusal while « Pages des agents » (or the MCP server) is off.
pub(crate) fn switched_off(lang: Lang) -> String {
    tr_in!(
        lang,
        "Les pages des agents sont coupées (Réglages › Agents › « Pages des agents ») : aucun de leurs outils ne répond. L’utilisateur seul les rallume.",
        "The agents' pages are off (Settings › Agents › “Agents' pages”): none of their tools answers. Only the user turns them back on."
    )
}

/// What `drive_browser` knows of an agent's page (ESC-679).
#[derive(Debug, Clone)]
pub(crate) struct DriveInfo {
    pub page: String,
    pub template: String,
    /// Where its row is shown; none when it is closed.
    pub row: Option<Placement>,
    /// The version it shows.
    pub shown: u32,
    /// The anchors of the version shown: key, selector.
    pub anchors: Vec<(String, String)>,
}

impl DriveInfo {
    /// The selector of a key of its template, or what keys there are.
    pub(crate) fn anchor(&self, key: &str) -> Result<String, String> {
        if self.template == "diagram" {
            if let Some(node) = key.strip_prefix("nodes.") {
                let ok = !node.is_empty()
                    && node.len() <= 64
                    && node
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
                if ok {
                    return Ok(format!("#diagram [data-node=\"{node}\"]"));
                }
            }
        }
        self.anchors
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, s)| s.clone())
            .ok_or_else(|| {
                let mut keys: Vec<&str> = self.anchors.iter().map(|(k, _)| k.as_str()).collect();
                if self.template == "diagram" {
                    keys.push("nodes.<id>");
                }
                keys.join(", ")
            })
    }
}

/// The refusal of a page no longer kept (its agent or its project gone, or room made for others).
pub(crate) fn gone(lang: Lang, page: &str) -> String {
    tr_in!(
        lang,
        "La page {page} n’est plus gardée.",
        "Page {page} is no longer kept."
    )
}

/// A page's proposal checked against its request `ask`: kept when the request waits, is no
/// go-ahead (`gate`: never proposed from a page), and its options are the request's own (a choice:
/// one, or several when `multi`; a confirm: `yes` or `no`); its note on its lines, 2 000
/// characters at most. None for anything else: an answer or a task, an unknown option, two for one.
pub(crate) fn checked_proposal(
    ask: &crate::asks::Ask,
    p: agentpage::Proposal,
    at: i64,
) -> Option<PageProposal> {
    use crate::asks::AskKind;
    if !ask.state.waiting() || ask.gate {
        return None;
    }
    let mut options: Vec<String> = Vec::new();
    for o in p.options {
        let o = o.trim().to_ascii_lowercase();
        if !options.contains(&o) {
            options.push(o);
        }
    }
    let fits = match ask.kind {
        AskKind::Choice => {
            !options.is_empty()
                && (ask.multi || options.len() == 1)
                && options
                    .iter()
                    .all(|o| ask.options.iter().any(|x| x.id == *o))
        }
        AskKind::Confirm => options.len() == 1 && (options[0] == "yes" || options[0] == "no"),
        _ => false,
    };
    if !fits {
        return None;
    }
    let note = p
        .note
        .map(|n| {
            n.chars()
                .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|n| !n.is_empty());
    if note
        .as_ref()
        .is_some_and(|n| n.chars().count() > agentpage::NOTE_MAX)
    {
        return None;
    }
    Some(PageProposal {
        ask: ask.id.clone(),
        options,
        note,
        at,
    })
}

/// The options a request's answer chose, as a page shows them: a choice's, `yes` or `no` for a
/// confirm; none before it is answered (or once the answer was taken back).
pub(crate) fn answered_options(ask: &crate::asks::Ask) -> Vec<String> {
    let Some(a) = &ask.answer else {
        return Vec::new();
    };
    match a.yes {
        Some(true) => vec!["yes".into()],
        Some(false) => vec!["no".into()],
        None => a.options.clone(),
    }
}

/// The fixed script that tells a page its answer: the options chosen checked (`aria-pressed`,
/// `.on`), every option frozen once there is an answer, `data-answered` on the request's block,
/// and the event `escouade:answered` for a page of its own. Nothing goes in but ids made of letters,
/// digits, `-` and `_` (a request's are `a` to `f`, `yes`, `no`).
pub(crate) fn answer_script(options: &[String]) -> String {
    let ids: Vec<&str> = options
        .iter()
        .map(String::as_str)
        .filter(|o| {
            !o.is_empty()
                && o.len() <= 16
                && o.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        })
        .collect();
    let list = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into());
    format!(
        "(function(a){{try{{var on=a.length>0;document.querySelectorAll('[data-ask]').forEach(function(h){{if(on)h.setAttribute('data-answered','1');else h.removeAttribute('data-answered')}});document.querySelectorAll('[data-option]').forEach(function(b){{var c=a.indexOf(b.getAttribute('data-option'))>=0;b.setAttribute('aria-pressed',String(c));b.classList.toggle('on',c);b.disabled=on}});window.dispatchEvent(new CustomEvent('escouade:answered',{{detail:{{options:a.slice()}}}}))}}catch(e){{}}}})({list});"
    )
}

/// Page `p` as the window draws it.
fn row_of(b: &PageBox, p: &AgentPage, loaded: bool) -> PageRow {
    let last = p.latest().map_or(1, |v| v.n);
    PageRow {
        page: p.id.clone(),
        project_id: p.project_id.clone(),
        agent_id: p.agent_id.clone(),
        agent_name: p.agent_name.clone(),
        ticket_id: p.ticket_id.clone(),
        title: p.title.clone(),
        template: p.template.clone(),
        versions: p
            .versions
            .iter()
            .map(|v| RowVersion { n: v.n, at: v.at })
            .collect(),
        shown: b.shown.get(&p.id).copied().unwrap_or(last),
        open: b.rows.contains_key(&p.id),
        quiet: b.quiet.contains(&p.id),
        loaded,
        ask: p.ask.clone(),
        proposal: b.proposals.get(&p.id).cloned(),
    }
}
