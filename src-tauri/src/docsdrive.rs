//! The drive of the documents (ESC-260, `docs/FACTORY.md` § « Les documents : un drive »): its
//! folders, a file dropped read and indexed on this machine then filed by itself, the search, and
//! what the agents did with each file (read, cited).
//!
//! **Filing** (`place`). A file dropped in a folder stays there: the user chose. One dropped on the
//! drive itself goes, in this order, to:
//! 1. the folder whose name its name or its title (its first line) bears (« Compte rendu du 12 »
//!    → « Comptes rendus »);
//! 2. the folder of its sort, when it is of a known sort (a report, requirements, a mock-up, a legal
//!    paper, a quote, a presentation, a planning: by its name and its first lines) and the drive has
//!    a folder of that sort;
//! 3. the folder whose files are nearest by the meaning, when the vectors' model is there (each
//!    folder as near as its nearest file, past a bar set per model);
//! 4. the folder whose names and texts share the most words with it (tf-idf), past a bar;
//! 5. else a new folder, made for it: the one of its sort, « Images » for an image, else « Divers ».
//!
//! The window says where it went and lets it be moved. Nothing leaves the machine: the text is
//! read here (`docsextract`), kept in Escouade's folder, and its vectors are computed here.
//!
//! **Read and cited**: a file is read by the agents each time one reads it by `read_document`
//! (counted in Escouade's folder, `docs-reads.json`: not in the repository); it is cited by the
//! pages of the spec whose sources name it and by the decisions linked to it.
//!
//! **Heavy files**: the repository's LFS rules apply when it has some (git does it); without one
//! that takes the file, past 50 MB the window asks before adding it (`LARGE_FILE`).
//!
//! A private project's documents are in its own index and answered to those who see the project
//! only (`mcp::resolve`): no other project's agent finds them.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Result};
use parking_lot::Mutex;
use serde::Serialize;

use crate::core::Core;
use crate::docs::{DocInfo, Folder, Space, DOCUMENTS, LARGE_FILE, ROOT};
use crate::docsextract::{extract, Extracted};
use crate::vectors::chunk::Format;
use crate::vectors::embed::{dot, Embedder};
use crate::vectors::index::{Index, Source};
use crate::vectors::model::{Store, MODEL};

/// Past this tf-idf likeness (a cosine) a folder's words fit a file.
const WORDS_FIT: f32 = 0.12;
/// What is read of a file's text to file it: its head says what it is.
const HEAD: usize = 6000;

// ---------- words ----------

fn fold(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' | 'œ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' | 'ý' => 'y',
        'ñ' => 'n',
        'æ' => 'a',
        c => c,
    }
}

/// `s` lowercase, without accents, every run of what is not a letter or a digit one space, with a
/// space at each end: a phrase is found in it as ` mot mot `.
fn plain(s: &str) -> String {
    let mut out = String::from(" ");
    for c in s.chars().flat_map(char::to_lowercase).map(fold) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    if !out.ends_with(' ') {
        out.push(' ');
    }
    out
}

const STOP: &[&str] = &[
    "le", "la", "les", "un", "une", "des", "du", "de", "et", "ou", "en", "au", "aux", "a", "pour",
    "par", "sur", "dans", "avec", "sans", "que", "qui", "ce", "ces", "cet", "cette", "est", "sont",
    "il", "elle", "on", "nous", "vous", "ils", "se", "sa", "son", "ses", "leur", "leurs", "pas",
    "ne", "plus", "the", "an", "and", "or", "of", "to", "in", "on", "for", "by", "with", "is",
    "are", "be", "this", "that", "it", "as", "at", "from", "not", "md", "txt", "pdf", "docx",
    "odt", "doc", "png", "jpg", "jpeg",
];

/// A word as it is compared: a final `s` or `x` taken off (« comptes » and « compte » match).
fn stem(w: &str) -> String {
    if w.chars().count() > 3 && (w.ends_with('s') || w.ends_with('x')) {
        w[..w.len() - 1].to_string()
    } else {
        w.to_string()
    }
}

/// The words of `s` that say something: no stop word, each stemmed.
pub fn words(s: &str) -> Vec<String> {
    plain(s)
        .split(' ')
        .filter(|w| !w.is_empty() && !STOP.contains(w))
        .map(stem)
        .collect()
}

/// A file's name without its extension, its `_` and `-` as spaces.
fn title_of(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    stem.replace(['_', '-'], " ")
}

// ---------- sorts of documents ----------

struct Sort {
    fr: &'static str,
    en: &'static str,
    /// Phrases (plain, see `plain`) that say a file is of this sort.
    keys: &'static [&'static str],
}

const SORTS: &[Sort] = &[
    Sort {
        fr: "Comptes rendus",
        en: "Meeting notes",
        keys: &[
            "compte rendu",
            "comptes rendus",
            "cr",
            "reunion",
            "proces verbal",
            "pv",
            "minutes",
            "meeting",
            "atelier",
            "comite",
            "ordre du jour",
            "presents",
            "kick off",
            "kickoff",
        ],
    },
    Sort {
        fr: "Cahiers des charges",
        en: "Requirements",
        keys: &[
            "cahier des charges",
            "cahiers des charges",
            "cdc",
            "specification",
            "specifications",
            "exigence",
            "exigences",
            "requirements",
            "expression des besoins",
            "rfp",
            "appel d offres",
            "besoins fonctionnels",
        ],
    },
    Sort {
        fr: "Maquettes",
        en: "Mock-ups",
        keys: &[
            "maquette",
            "maquettes",
            "mockup",
            "mockups",
            "mock up",
            "wireframe",
            "wireframes",
            "prototype",
            "figma",
            "ecran",
            "ecrans",
            "screen",
            "screens",
            "design",
        ],
    },
    Sort {
        fr: "Juridique",
        en: "Legal",
        keys: &[
            "contrat",
            "contrats",
            "contract",
            "cgv",
            "cgu",
            "rgpd",
            "gdpr",
            "juridique",
            "legal",
            "mentions legales",
            "conditions generales",
            "licence",
            "license",
            "nda",
            "avenant",
            "confidentialite",
            "clause",
            "clauses",
        ],
    },
    Sort {
        fr: "Devis et factures",
        en: "Quotes and invoices",
        keys: &[
            "devis",
            "facture",
            "factures",
            "invoice",
            "invoices",
            "quote",
            "quotation",
            "bon de commande",
            "purchase order",
        ],
    },
    Sort {
        fr: "Présentations",
        en: "Presentations",
        keys: &[
            "presentation",
            "presentations",
            "slides",
            "pitch",
            "deck",
            "keynote",
        ],
    },
    Sort {
        fr: "Planning",
        en: "Planning",
        keys: &[
            "planning",
            "calendrier",
            "jalon",
            "jalons",
            "roadmap",
            "feuille de route",
            "echeancier",
            "gantt",
            "milestone",
            "milestones",
            "retroplanning",
        ],
    },
];

impl Sort {
    fn label(&self) -> &'static str {
        if crate::i18n::ui() == crate::i18n::Lang::Fr {
            self.fr
        } else {
            self.en
        }
    }

    /// Whether a folder of this name is of this sort: its name is one of its labels, or one of its
    /// phrases.
    fn names(&self, folder: &str) -> bool {
        let f = plain(folder);
        [self.fr, self.en].iter().any(|l| plain(l) == f)
            || self.keys.iter().any(|k| f == format!(" {k} "))
    }
}

/// The sort of a file, by its name (a phrase there counts 3) and its head (1 each, once per
/// phrase): the best past 2, else none.
fn sort_of(name: &str, head: &str) -> Option<&'static Sort> {
    let n = plain(&title_of(name));
    let h = plain(head);
    let mut best: Option<(&Sort, u32)> = None;
    for s in SORTS {
        let score: u32 = s
            .keys
            .iter()
            .map(|k| {
                let k = format!(" {k} ");
                3 * u32::from(n.contains(&k)) + u32::from(h.contains(&k))
            })
            .sum();
        if score >= 2 && best.is_none_or(|(_, b)| score > b) {
            best = Some((s, score));
        }
    }
    best.map(|(s, _)| s)
}

// ---------- filing ----------

/// A folder of the drive as it is compared with a file: its path, and the names and heads of the
/// texts of the files right in it.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub folder: String,
    pub text: String,
}

/// Where a file dropped went, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// Under `documents/`; empty: the root.
    pub folder: String,
    /// The folder was made for it.
    pub new: bool,
    /// chosen (dropped in it), name, sort, meaning, words, or new.
    pub by: String,
}

fn placed(folder: &str, new: bool, by: &str) -> Placement {
    Placement {
        folder: folder.to_string(),
        new,
        by: by.to_string(),
    }
}

fn last(folder: &str) -> &str {
    folder.rsplit('/').next().unwrap_or(folder)
}

/// The tf-idf likeness of `doc` with each folder's text: a cosine for each, in their order. The
/// numbers are left out: a year or a date says when, not what about.
fn words_fit(doc: &[String], folders: &[Vec<String>]) -> Vec<f32> {
    let says = |ws: &[String]| -> Vec<String> {
        ws.iter()
            .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
            .cloned()
            .collect()
    };
    let doc = says(doc);
    let folders: Vec<Vec<String>> = folders.iter().map(|f| says(f)).collect();
    let (doc, folders) = (doc.as_slice(), folders.as_slice());
    let n = folders.len() as f32;
    let mut df: HashMap<&str, f32> = HashMap::new();
    for f in folders {
        let seen: HashSet<&str> = f.iter().map(String::as_str).collect();
        for w in seen {
            *df.entry(w).or_default() += 1.0;
        }
    }
    let idf = |w: &str| (1.0 + n / df.get(w).copied().unwrap_or(1.0)).ln();
    let vector = |ws: &[String]| -> HashMap<String, f32> {
        let mut tf: HashMap<String, f32> = HashMap::new();
        for w in ws {
            *tf.entry(w.clone()).or_default() += 1.0;
        }
        tf.into_iter()
            .map(|(w, c)| {
                let x = (1.0 + c.ln()) * idf(&w);
                (w, x)
            })
            .collect()
    };
    let d = vector(doc);
    let dn = d.values().map(|x| x * x).sum::<f32>().sqrt();
    folders
        .iter()
        .map(|f| {
            let v = vector(f);
            let vn = v.values().map(|x| x * x).sum::<f32>().sqrt();
            if dn == 0.0 || vn == 0.0 {
                return 0.0;
            }
            d.iter()
                .map(|(w, x)| x * v.get(w).copied().unwrap_or(0.0))
                .sum::<f32>()
                / (dn * vn)
        })
        .collect()
}

/// Where a file named `name`, of text `text` (None: not read, an image), goes among the folders
/// `profiles`, given how near each folder is by the meaning (`meaning`, empty without the model)
/// and the bar past which a folder is near (`related`). See the module's comment.
pub fn place(
    name: &str,
    text: Option<&str>,
    profiles: &[Profile],
    meaning: &[(String, f32)],
    related: f32,
) -> Placement {
    let head: String = text.unwrap_or_default().chars().take(HEAD).collect();
    let first_line = head
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();
    let title: HashSet<String> = words(&format!("{} {first_line}", title_of(name)))
        .into_iter()
        .collect();

    // 1. Its name says the folder: every word of the folder's name, the most words the best.
    let mut by_name: Option<(&str, usize)> = None;
    for p in profiles {
        let fw = words(last(&p.folder));
        let says = !fw.is_empty()
            && fw.iter().any(|w| !w.chars().all(|c| c.is_ascii_digit()))
            && fw.iter().all(|w| title.contains(w));
        if says && by_name.is_none_or(|(_, n)| fw.len() > n) {
            by_name = Some((&p.folder, fw.len()));
        }
    }
    if let Some((f, _)) = by_name {
        return placed(f, false, "name");
    }

    // 2. Its sort, when the drive has a folder of that sort.
    let sort = sort_of(name, &head);
    if let Some(s) = sort {
        if let Some(p) = profiles.iter().find(|p| s.names(last(&p.folder))) {
            return placed(&p.folder, false, "sort");
        }
    }

    // 3. The meaning.
    if let Some((f, score)) = meaning
        .iter()
        .filter(|(f, _)| profiles.iter().any(|p| p.folder == *f))
        .max_by(|a, b| a.1.total_cmp(&b.1))
    {
        if *score >= related {
            return placed(f, false, "meaning");
        }
    }

    // 4. The words.
    if text.is_some() && !profiles.is_empty() {
        let doc = words(&format!("{} {} {head}", title_of(name), title_of(name)));
        let folders: Vec<Vec<String>> = profiles
            .iter()
            .map(|p| {
                let n = last(&p.folder);
                words(&format!("{n} {n} {n} {}", p.text))
            })
            .collect();
        let fit = words_fit(&doc, &folders);
        if let Some((i, s)) = fit.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)) {
            if *s >= WORDS_FIT {
                return placed(&profiles[i].folder, false, "words");
            }
        }
    }

    // 5. A new folder.
    let new = match sort {
        Some(s) => s.label().to_string(),
        None if crate::docs::kind_of(name) == "image" => tr!("Images", "Images"),
        None => tr!("Divers", "Other"),
    };
    // A folder of that name already there (of another sort of name, or empty) takes it.
    if let Some(p) = profiles
        .iter()
        .find(|p| plain(last(&p.folder)) == plain(&new))
    {
        return placed(&p.folder, false, "new");
    }
    placed(&new, true, "new")
}

/// The bar past which a folder is near a file by the meaning, for the model `id`: the cosines of
/// multilingual E5 sit high (0.75 for two texts that have nothing in common), those of the tests'
/// model low.
pub fn related(id: &str) -> f32 {
    if id == "fake" {
        0.35
    } else {
        0.82
    }
}

// ---------- read and cited ----------

/// The path of a document a citation names, among `docs`: its path under `documents/` (with or
/// without `.escouade/documents/` or `documents/` before it), the path the window shows
/// (`Documents › A › b`), or its name with or without its extension when only one file has it.
pub fn cited_doc<'a>(cite: &str, docs: &'a [DocInfo]) -> Option<&'a DocInfo> {
    let c = cite.trim().replace(" › ", "/").replace('\\', "/");
    let c = c
        .trim_start_matches("./")
        .trim_start_matches(".escouade/")
        .trim_start_matches("documents/")
        .trim_start_matches("Documents/");
    if c.is_empty() {
        return None;
    }
    if let Some(d) = docs.iter().find(|d| d.path == c) {
        return Some(d);
    }
    let lc = c.to_lowercase();
    let named: Vec<&DocInfo> = docs
        .iter()
        .filter(|d| {
            let n = d.name.to_lowercase();
            let path = d.path.to_lowercase();
            let bare = path.rsplit_once('.').map_or(path.as_str(), |(p, _)| p);
            n == lc
                || title_of(&n) == lc
                || title_of(&n) == title_of(&lc)
                || lc.ends_with(&format!("/{n}"))
                || bare == lc
        })
        .collect();
    (named.len() == 1).then(|| named[0])
}

/// How many pages of the spec and decisions cite each document (each page or decision once).
pub fn citations(space: &Space, docs: &[DocInfo]) -> HashMap<String, u32> {
    let mut out: HashMap<String, u32> = HashMap::new();
    let mut count = |cites: &[String]| {
        let hit: HashSet<&str> = cites
            .iter()
            .filter_map(|c| cited_doc(c, docs))
            .map(|d| d.path.as_str())
            .collect();
        for p in hit {
            *out.entry(p.to_string()).or_default() += 1;
        }
    };
    for s in space.spec_tree() {
        for p in s.pages {
            if let Ok(page) = space.spec_read(&p.path) {
                count(&page.sources);
            }
        }
    }
    for d in space.decisions() {
        count(&d.documents);
    }
    out
}

/// One write of the reads at a time.
static READS: Mutex<()> = Mutex::new(());

/// How many times the agents read each document of a project, kept in `file`.
pub fn reads(file: &Path) -> HashMap<String, u32> {
    std::fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// One read more of `path`.
pub fn note_read(file: &Path, path: &str) {
    let _one = READS.lock();
    let mut all = reads(file);
    *all.entry(path.to_string()).or_default() += 1;
    save_reads(file, &all);
}

/// The reads of `from` follow it to `to` (moved).
pub fn move_reads(file: &Path, from: &str, to: &str) {
    let _one = READS.lock();
    let mut all = reads(file);
    if let Some(n) = all.remove(from) {
        *all.entry(to.to_string()).or_default() += n;
        save_reads(file, &all);
    }
}

fn save_reads(file: &Path, all: &HashMap<String, u32>) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(b) = serde_json::to_vec(all) {
        let _ = crate::paths::write_atomic(file, &b);
    }
}

// ---------- search ----------

/// A document found by `search_documents` or the window's search.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocHit {
    pub path: String,
    pub name: String,
    pub folder: String,
    pub kind: String,
    /// The passage that answers, on one line.
    pub excerpt: String,
}

fn one_line(s: &str, max: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= max {
        return one;
    }
    let mut out: String = one.chars().take(max).collect();
    out.push('…');
    out
}

/// What is indexed of a document: its name, then its text or what is known of it.
fn indexed_text(d: &DocInfo, got: &Extracted) -> String {
    match got {
        Extracted::Text(t) => format!("{}\n\n{t}", title_of(&d.name)),
        Extracted::Meta(m) => format!("{}\n\n{m}", title_of(&d.name)),
        Extracted::None => title_of(&d.name),
    }
}

/// Brings the index's documents up to date with the drive: a file changed read again, one gone
/// forgotten.
pub fn sync_index(index: &mut Index, space: &Space, docs: &[DocInfo]) -> Result<()> {
    for d in docs {
        let got = space.extracted(&d.path).unwrap_or(Extracted::None);
        index.put(Source::Document, &d.path, &indexed_text(d, &got))?;
    }
    let keep: Vec<&str> = docs.iter().map(|d| d.path.as_str()).collect();
    index.retain(Source::Document, &keep)?;
    Ok(())
}

fn hit_of(d: &DocInfo, excerpt: String) -> DocHit {
    DocHit {
        path: d.path.clone(),
        name: d.name.clone(),
        folder: d.folder.clone(),
        kind: d.kind.clone(),
        excerpt,
    }
}

/// The documents whose name or text holds the words of `query`: a word in the name counts more
/// than in the text. Each with the line that holds the first word found.
pub fn search_words(space: &Space, docs: &[DocInfo], query: &str, limit: usize) -> Vec<DocHit> {
    let asked = words(query);
    if asked.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(f32, DocHit)> = Vec::new();
    for d in docs {
        let name: HashSet<String> = words(&title_of(&d.name)).into_iter().collect();
        let got = space.extracted(&d.path).unwrap_or(Extracted::None);
        let text = match &got {
            Extracted::Text(t) | Extracted::Meta(t) => t.as_str(),
            Extracted::None => "",
        };
        let mut score = 0.0;
        let mut line = None;
        for w in &asked {
            let in_name = name.iter().any(|n| n.starts_with(w.as_str()));
            let found = text
                .lines()
                .find(|l| words(l).iter().any(|x| x.starts_with(w.as_str())));
            if in_name {
                score += 3.0;
            }
            if let Some(l) = found {
                score += 1.0;
                line.get_or_insert(l);
            }
        }
        if score > 0.0 {
            let excerpt = line.map_or_else(|| d.folder.clone(), |l| one_line(l, 240));
            scored.push((score, hit_of(d, excerpt)));
        }
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.path.cmp(&b.1.path)));
    scored.into_iter().take(limit).map(|(_, h)| h).collect()
}

/// The documents nearest `query` by the meaning and the words (`Index::search_hybrid`), the index
/// brought up to date first. Each once, best first.
pub fn search_meaning(
    index: &mut Index,
    space: &Space,
    docs: &[DocInfo],
    query: &str,
    limit: usize,
) -> Result<Vec<DocHit>> {
    sync_index(index, space, docs)?;
    let hits = index.search_hybrid(query, limit * 4, &[Source::Document])?;
    let mut out: Vec<DocHit> = Vec::new();
    for h in hits {
        if out.iter().any(|f| f.path == h.path) {
            continue;
        }
        if let Some(d) = docs.iter().find(|d| d.path == h.path) {
            out.push(hit_of(d, one_line(&h.text, 240)));
        }
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

// ---------- the drive, for the window ----------

/// The drive as the window shows it: its folders and its files.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    pub folders: Vec<Folder>,
    pub files: Vec<DocInfo>,
}

/// A file dropped: where it is now, and why there.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dropped {
    pub file: DocInfo,
    pub placement: Placement,
    /// The repository's LFS rules take it.
    pub lfs: bool,
}

/// What the window learns of a file before it is dropped.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub size: u64,
    /// The repository's LFS rules take it.
    pub lfs: bool,
    /// Over 50 MB and not taken by LFS: the window asks first.
    pub large: bool,
}

/// The files and heads of their texts of each folder, as `place` compares them.
pub fn profiles(space: &Space, folders: &[Folder], docs: &[DocInfo]) -> Vec<Profile> {
    folders
        .iter()
        .map(|f| {
            let mut text = String::new();
            for d in docs.iter().filter(|d| d.folder == f.path) {
                text.push_str(&title_of(&d.name));
                text.push('\n');
                if let Ok(Extracted::Text(t)) = space.extracted(&d.path) {
                    text.extend(t.chars().take(HEAD));
                    text.push('\n');
                }
            }
            Profile {
                folder: f.path.clone(),
                text,
            }
        })
        .collect()
}

/// How near each folder is to `text` by the meaning: as near as its nearest file. The index is
/// brought up to date first.
pub fn meaning_of(
    index: &mut Index,
    space: &Space,
    docs: &[DocInfo],
    name: &str,
    text: &str,
) -> Result<Vec<(String, f32)>> {
    sync_index(index, space, docs)?;
    let head: String = text.chars().take(HEAD).collect();
    let v = index.passage_vector(
        &format!("{}\n\n{head}", title_of(name)),
        Format::of_path(name),
    )?;
    if v.is_empty() {
        return Ok(Vec::new());
    }
    let mut best: HashMap<String, f32> = HashMap::new();
    for (path, dv) in index.doc_vectors(Source::Document)? {
        let Some(d) = docs.iter().find(|d| d.path == path) else {
            continue;
        };
        if d.folder.is_empty() {
            continue;
        }
        let s = dot(&v, &dv);
        let e = best.entry(d.folder.clone()).or_insert(f32::MIN);
        *e = e.max(s);
    }
    Ok(best.into_iter().collect())
}

/// Whether the repository's LFS rules take `rel` (a path from the folder of `.escouade/`).
pub async fn lfs_takes(root: &Path, rel: &str) -> bool {
    let root = root.to_string_lossy().to_string();
    match crate::git::run(
        &root,
        &[
            "-c",
            "core.quotepath=false",
            "check-attr",
            "filter",
            "--",
            rel,
        ],
    )
    .await
    {
        Ok(out) => String::from_utf8_lossy(&out)
            .trim_end()
            .ends_with(": filter: lfs"),
        Err(_) => false,
    }
}

/// Who changed each file of the drive last, as git says.
async fn authors(root: &Path) -> HashMap<String, String> {
    let root = root.to_string_lossy().to_string();
    let prefix = format!("{ROOT}/{DOCUMENTS}/");
    let Ok(out) = crate::git::run(
        &root,
        &[
            "-c",
            "core.quotepath=false",
            "log",
            "-n",
            "500",
            "--format=%x01%an",
            "--name-only",
            "--",
            &format!("{ROOT}/{DOCUMENTS}"),
        ],
    )
    .await
    else {
        return HashMap::new();
    };
    let mut out_map = HashMap::new();
    let mut who = String::new();
    for line in String::from_utf8_lossy(&out).lines() {
        if let Some(a) = line.strip_prefix('\u{1}') {
            who = a.to_string();
        } else if let Some(p) = line.strip_prefix(&prefix) {
            out_map.entry(p.to_string()).or_insert_with(|| who.clone());
        }
    }
    out_map
}

/// The model of the vectors loaded once, when it is downloaded: None otherwise (the search goes by
/// the words, the filing by the names and the words).
static LOADED: Mutex<Option<Arc<dyn Embedder>>> = Mutex::new(None);

impl Core {
    pub(crate) fn vectors_embedder(&self) -> Option<Arc<dyn Embedder>> {
        #[cfg(test)]
        if let Some(e) = self.test_embedder.lock().clone() {
            return Some(e);
        }
        let store = Store {
            dir: self.data.vectors_model(MODEL.id),
        };
        if !store.has(&MODEL) {
            return None;
        }
        let mut loaded = LOADED.lock();
        if loaded.is_none() {
            match store.load(&MODEL) {
                Ok(m) => *loaded = Some(Arc::new(m)),
                Err(e) => log::warn!("vectors: the model not loaded: {e:#}"),
            }
        }
        loaded.clone()
    }

    fn docs_reads_file(&self, project_id: &str) -> PathBuf {
        self.data.project_files(project_id).join("docs-reads.json")
    }

    /// An agent read a document (`read_document`).
    pub fn docs_note_read(&self, project_id: &str, path: &str) {
        note_read(&self.docs_reads_file(project_id), path);
    }

    /// The drive of a project, each file with who added it, how many times the agents read it, and
    /// how many pages and decisions cite it.
    pub async fn docs_drive(&self, project_id: &str) -> Result<Drive> {
        let space = self.docs_space(project_id, None).await?;
        self.docs_drive_in(project_id, space).await
    }

    /// `docs_drive` of the docs `space` of a project (an agent's own).
    pub async fn docs_drive_in(&self, project_id: &str, space: Space) -> Result<Drive> {
        let project = self.project(project_id)?;
        let mut files = space.documents("")?;
        let folders = space.folders()?;
        let by = authors(&space.root).await;
        let me = self.me_in(&project).name;
        let read = reads(&self.docs_reads_file(project_id));
        let space2 = space.clone();
        let files2 = files.clone();
        let cited = tokio::task::spawn_blocking(move || citations(&space2, &files2))
            .await
            .unwrap_or_default();
        for f in &mut files {
            f.author = by.get(&f.path).cloned().unwrap_or_else(|| me.clone());
            f.reads = read.get(&f.path).copied().unwrap_or(0);
            f.cited = cited.get(&f.path).copied().unwrap_or(0);
        }
        Ok(Drive { folders, files })
    }

    /// What the window must know of files before they are dropped in `folder`: whether LFS takes
    /// them, whether they are large enough to be asked about.
    pub async fn docs_check(
        &self,
        project_id: &str,
        folder: &str,
        files: Vec<(String, u64)>,
    ) -> Result<Vec<Check>> {
        let space = self.docs_space(project_id, None).await?;
        let mut out = Vec::new();
        for (name, size) in files {
            let lfs = lfs_takes(&space.root, &doc_rel(folder, &name)).await;
            out.push(Check {
                large: size > LARGE_FILE && !lfs,
                name,
                size,
                lfs,
            });
        }
        Ok(out)
    }

    /// A file dropped (`source`, a file of this machine): read and indexed here, then filed in
    /// `folder` when one is given, else where `place` says (a folder made for it if need be). Over
    /// 50 MB without an LFS rule that takes it, it must be `confirmed`.
    pub async fn docs_drop(
        &self,
        project_id: &str,
        folder: Option<&str>,
        source: &Path,
        confirmed: bool,
    ) -> Result<Dropped> {
        let space = self.docs_space(project_id, None).await?;
        self.docs_drop_in(project_id, space, folder, source, None, confirmed)
            .await
    }

    /// `docs_drop` in the docs `space` of a project, the file named `name` (its own name when
    /// None).
    pub async fn docs_drop_in(
        &self,
        project_id: &str,
        space: Space,
        folder: Option<&str>,
        source: &Path,
        name: Option<&str>,
        confirmed: bool,
    ) -> Result<Dropped> {
        let meta = std::fs::metadata(source)?;
        if !meta.is_file() {
            bail!(tr!(
                "{p} n’est pas un fichier.",
                "{p} is not a file.",
                p = source.display()
            ));
        }
        let name = crate::attachfiles::safe_name(&name.map(str::to_string).unwrap_or_else(|| {
            source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        }));
        let chosen = folder.map(str::trim).filter(|f| !f.is_empty());
        let lfs_at = |f: &str| doc_rel(f, &name);
        if !confirmed
            && meta.len() > LARGE_FILE
            && !lfs_takes(&space.root, &lfs_at(chosen.unwrap_or(""))).await
        {
            bail!(tr!(
                "Ce fichier pèse {mb} Mo et le dépôt n’a pas de règle LFS pour lui : il entre dans le dépôt, que git garde en entier à chaque version. Confirme pour l’ajouter.",
                "This file weighs {mb} MB and the repository has no LFS rule for it: it goes into the repository, and git keeps every version of it in full. Confirm to add it.",
                mb = meta.len() / (1024 * 1024)
            ));
        }
        let embedder = self.vectors_embedder();
        let index_path = self.data.vectors_index(project_id);
        let src = source.to_path_buf();
        let (space2, name2, chosen2) = (space.clone(), name.clone(), chosen.map(str::to_string));
        let embedder2 = embedder.clone();
        let index2 = index_path.clone();
        let (got, placement) =
            tokio::task::spawn_blocking(move || -> Result<(Extracted, Placement)> {
                let got = extract(&src, &name2);
                if let Some(f) = chosen2 {
                    return Ok((got, placed(&f, false, "chosen")));
                }
                let docs = space2.documents("")?;
                let folders = space2.folders()?;
                let profiles = profiles(&space2, &folders, &docs);
                let mut meaning = Vec::new();
                let mut bar = 1.0;
                if let (Some(e), Some(text)) = (embedder2, got.text()) {
                    bar = related(e.id());
                    match Index::open(&index2, e)
                        .and_then(|mut i| meaning_of(&mut i, &space2, &docs, &name2, text))
                    {
                        Ok(m) => meaning = m,
                        Err(e) => {
                            log::warn!("docs: filing by the meaning failed, by the words: {e:#}")
                        }
                    }
                }
                let p = place(&name2, got.text(), &profiles, &meaning, bar);
                Ok((got, p))
            })
            .await??;
        if placement.new {
            space.folder_add(&placement.folder)?;
        }
        let file = space.document_copy(&placement.folder, &name, source)?;
        space.keep_extracted(&file.path, &got);
        let lfs = lfs_takes(&space.root, &doc_rel(&file.folder, &file.name)).await;
        // Its vectors, after: the window has its answer at once.
        if let Some(e) = embedder {
            let (space3, file3) = (space.clone(), file.clone());
            tokio::task::spawn_blocking(move || {
                let r = Index::open(&index_path, e).and_then(|mut i| {
                    i.put(Source::Document, &file3.path, &indexed_text(&file3, &got))
                });
                if let Err(e) = r {
                    log::warn!("docs: {} not indexed: {e:#}", file3.path);
                }
                drop(space3);
            });
        }
        self.docs_changed(project_id).await;
        Ok(Dropped {
            file,
            placement,
            lfs,
        })
    }

    /// A document moved to another folder of the drive.
    pub async fn docs_move(&self, project_id: &str, path: &str, folder: &str) -> Result<DocInfo> {
        let space = self.docs_space(project_id, None).await?;
        let moved = space.document_move(path, folder)?;
        move_reads(&self.docs_reads_file(project_id), path, &moved.path);
        self.docs_changed(project_id).await;
        Ok(moved)
    }

    /// « Nouveau dossier ».
    pub async fn docs_folder_add(&self, project_id: &str, folder: &str) -> Result<String> {
        let space = self.docs_space(project_id, None).await?;
        let made = space.folder_add(folder)?;
        self.docs_changed(project_id).await;
        Ok(made)
    }

    /// The documents that answer `query` in the docs `space` of a project: by the meaning and the
    /// words when the vectors' model is there, by the words otherwise. Answers how it searched.
    pub async fn docs_search_in(
        &self,
        project_id: &str,
        space: Space,
        query: &str,
        limit: usize,
    ) -> Result<(Vec<DocHit>, &'static str)> {
        let embedder = self.vectors_embedder();
        let index = self.data.vectors_index(project_id);
        let query = query.to_string();
        let limit = limit.clamp(1, 50);
        tokio::task::spawn_blocking(move || {
            let docs = space.documents("")?;
            if let Some(e) = embedder.filter(|_| !words(&query).is_empty()) {
                match Index::open(&index, e)
                    .and_then(|mut i| search_meaning(&mut i, &space, &docs, &query, limit))
                {
                    Ok(found) => return Ok((found, "meaning")),
                    Err(e) => log::warn!("docs: search by meaning failed, by words: {e:#}"),
                }
            }
            Ok((search_words(&space, &docs, &query, limit), "words"))
        })
        .await?
    }
}

/// The path of `name` in `folder` from the folder of `.escouade/`, as git's attributes match it.
fn doc_rel(folder: &str, name: &str) -> String {
    let f = folder.trim_matches('/');
    if f.is_empty() {
        format!("{ROOT}/{DOCUMENTS}/{name}")
    } else {
        format!("{ROOT}/{DOCUMENTS}/{f}/{name}")
    }
}
