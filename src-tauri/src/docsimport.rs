//! Importing a spec the project already has (ESC-258, `docs/FACTORY.md`, « Réponses de
//! l'utilisateur » 1): Escouade offers it, shows how it would cut it by subject, and writes
//! nothing until the user agrees.
//!
//! - **The cut follows the titles.** In a file, the level of title that comes more than once
//!   (a lone `# Spec` on top is the document's name) gives the subjects, the next level the pages;
//!   a subject without titles under it is one page. What comes before the first subject is an
//!   « Introduction ». In a folder, each Markdown file is a subject, cut into pages the same way.
//! - **Only what `Space::detect` offers** is read: a path the window did not get from it is refused,
//!   and no link is followed on the way.
//! - **Nothing is overwritten**: a page already in the spec stays as it is and is said skipped.

use anyhow::{bail, Result};
use serde::Serialize;

use crate::docs::{self, Page, Space};
use crate::i18n::Lang;

/// The most pages one import writes.
const MAX_PAGES: usize = 500;

/// The most bytes one folder import reads, all its files together.
const MAX_IMPORT: usize = 16 * docs::MAX_TEXT as usize;

/// Where the imports are kept, in `.escouade/spec/` (shared like the pages, which skip dot files).
const IMPORTS: &str = ".imports.json";

/// An import: where the spec came from (relative to the project) and the day.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct ImportedFrom {
    pub path: String,
    pub at: String,
}

#[derive(Serialize, serde::Deserialize)]
struct ImportsFile {
    imports: Vec<ImportedFrom>,
}

/// A page as an import would write it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    /// `subject/page`, without `.md`.
    pub path: String,
    pub subject: String,
    pub title: String,
    /// The file it comes from, relative to the project.
    pub source: String,
    /// Its lines (blank ones left out): what the preview shows of its size.
    pub lines: u32,
    /// A page of that path is already in the spec: it is left alone.
    pub exists: bool,
    #[serde(skip)]
    pub body: String,
}

/// What an import wrote.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub written: Vec<String>,
    pub skipped: Vec<String>,
}

/// A title line outside the code blocks: its level and its text.
fn title_of(line: &str) -> Option<(usize, String)> {
    let t = line.trim_end();
    let level = t.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &t[level..];
    if !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim().trim_end_matches('#').trim().to_string();
    (!text.is_empty()).then_some((level, text))
}

/// The lines of a text with, for each, its title when it is one (code blocks left alone).
fn lines_of(text: &str) -> Vec<(&str, Option<(usize, String)>)> {
    mark(text.lines())
}

fn mark<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<(&'a str, Option<(usize, String)>)> {
    let mut fence = false;
    lines
        .into_iter()
        .map(|l| {
            let t = l.trim_start();
            if t.starts_with("```") || t.starts_with("~~~") {
                fence = !fence;
                return (l, None);
            }
            (l, if fence { None } else { title_of(l) })
        })
        .collect()
}

/// A file's front matter (`---` … `---` on top) left out.
fn without_front(text: &str) -> &str {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    {
        if let Some(end) = rest.find("\n---") {
            let after = &rest[end + 4..];
            return after.split_once('\n').map_or("", |(_, b)| b);
        }
    }
    text
}

fn tidy(lines: &[&str]) -> String {
    let body = lines.join("\n");
    body.trim_matches(['\n', '\r']).trim_end().to_string()
}

fn count(body: &str) -> u32 {
    body.lines().filter(|l| !l.trim().is_empty()).count() as u32
}

/// A section: its title and its lines (its own title left out).
struct Section<'a> {
    title: String,
    lines: Vec<&'a str>,
}

/// Cuts `lines` at the titles of `level`: what comes before the first, then each section.
fn split_at<'a>(
    lines: &[(&'a str, Option<(usize, String)>)],
    level: usize,
) -> (Vec<&'a str>, Vec<Section<'a>>) {
    let mut head = Vec::new();
    let mut out: Vec<Section<'a>> = Vec::new();
    for (l, t) in lines {
        match t {
            Some((lv, text)) if *lv == level => out.push(Section {
                title: text.clone(),
                lines: Vec::new(),
            }),
            _ => match out.last_mut() {
                Some(s) => s.lines.push(l),
                None => head.push(*l),
            },
        }
    }
    (head, out)
}

/// The level of title that cuts `lines`: the highest, unless it comes once (a document's own name),
/// then the next one present. None without titles.
fn cut_level(lines: &[(&str, Option<(usize, String)>)]) -> Option<(usize, Option<String>)> {
    let levels: Vec<usize> = lines
        .iter()
        .filter_map(|(_, t)| t.as_ref().map(|t| t.0))
        .collect();
    let top = *levels.iter().min()?;
    let tops = levels.iter().filter(|l| **l == top).count();
    let next = levels.iter().filter(|l| **l > top).min().copied();
    match (tops, next) {
        (1, Some(next)) => {
            let name = lines
                .iter()
                .find_map(|(_, t)| t.as_ref().filter(|t| t.0 == top).map(|t| t.1.clone()));
            Some((next, name))
        }
        _ => Some((top, None)),
    }
}

/// The pages of one subject: its sections at the next level of title, or itself whole.
fn pages_of(subject: &str, lines: &[&str], source: &str, out: &mut Vec<Draft>) {
    let marked = mark(lines.iter().copied());
    let level = marked
        .iter()
        .filter_map(|(_, t)| t.as_ref().map(|t| t.0))
        .min();
    let whole = |out: &mut Vec<Draft>| {
        let body = tidy(lines);
        if !body.is_empty() {
            out.push(draft(subject, subject, &body, source));
        }
    };
    let Some(level) = level else {
        whole(out);
        return;
    };
    let (head, sections) = split_at(&marked, level);
    let intro = tidy(&head);
    if !intro.is_empty() {
        out.push(draft(subject, subject, &intro, source));
    }
    for s in sections {
        out.push(draft(subject, &s.title, &tidy(&s.lines), source));
    }
}

fn draft(subject: &str, title: &str, body: &str, source: &str) -> Draft {
    Draft {
        path: String::new(),
        subject: subject.to_string(),
        title: title.to_string(),
        source: source.to_string(),
        lines: count(body),
        exists: false,
        body: body.to_string(),
    }
}

/// The pages a file's text gives, by subject (`subject`: the file is one subject, a folder's).
pub fn cut(text: &str, source: &str, subject: Option<&str>, intro: &str) -> Vec<Draft> {
    let text = without_front(text);
    let marked = lines_of(text);
    let mut out = Vec::new();
    if let Some(subject) = subject {
        // One subject: the document's own name (a lone top title) is left out, its sections are pages.
        let skip = match cut_level(&marked) {
            Some((_, Some(name))) => marked
                .iter()
                .position(|(_, t)| t.as_ref().is_some_and(|t| t.1 == name)),
            _ => None,
        };
        let lines: Vec<&str> = marked
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != skip)
            .map(|(_, (l, _))| *l)
            .collect();
        pages_of(subject, &lines, source, &mut out);
        return out;
    }
    let Some((level, name)) = cut_level(&marked) else {
        let body = tidy(&marked.iter().map(|(l, _)| *l).collect::<Vec<_>>());
        if !body.is_empty() {
            out.push(draft(intro, name_or(intro, None), &body, source));
        }
        return out;
    };
    let (head, sections) = split_at(&marked, level);
    // The document's own name, a lone title on top, is the introduction's title, not its text.
    let named = head.iter().position(|l| {
        name.as_ref()
            .is_some_and(|n| title_of(l).is_some_and(|t| &t.1 == n))
    });
    let head: Vec<&str> = head
        .into_iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != named)
        .map(|(_, l)| l)
        .collect();
    let body = tidy(&head);
    if !body.is_empty() {
        out.push(draft(intro, name_or(intro, name.as_deref()), &body, source));
    }
    for s in sections {
        pages_of(&s.title, &s.lines, source, &mut out);
    }
    out
}

fn name_or<'a>(intro: &'a str, name: Option<&'a str>) -> &'a str {
    name.unwrap_or(intro)
}

/// Gives each page its path, `subject/page`, unique.
fn place(drafts: &mut [Draft]) {
    let mut taken: Vec<String> = Vec::new();
    for d in drafts.iter_mut() {
        let subject = Some(docs::slug(&d.subject))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "general".into());
        let page = Some(docs::slug(&d.title))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "page".into());
        let mut path = format!("{subject}/{page}");
        let mut n = 2;
        while taken.contains(&path) {
            path = format!("{subject}/{page}-{n}");
            n += 1;
        }
        taken.push(path.clone());
        d.path = path;
    }
}

fn refused(path: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "« {path} » n’est pas une spec qu’Escouade a trouvée dans ce projet.",
        "“{path}” isn’t a spec Escouade found in this project."
    ))
}

impl Space {
    /// How the spec at `path` (one of `detect`'s) would be cut: nothing written.
    pub fn import_plan(&self, path: &str) -> Result<Vec<Draft>> {
        let found = self
            .detect()
            .into_iter()
            .find(|c| c.path == path)
            .ok_or_else(|| refused(path))?;
        let intro = tr!("Introduction", "Introduction");
        let mut drafts = Vec::new();
        if found.kind == "file" {
            let text = self.read_found(path)?;
            drafts = cut(&text, path, None, &intro);
        } else {
            let mut total = 0usize;
            for rel in self.found_files(path) {
                let text = self.read_found(&rel)?;
                total += text.len();
                if total > MAX_IMPORT {
                    bail!(tr!(
                        "« {path} » est trop gros pour être importé",
                        "“{path}” is too big to be imported"
                    ));
                }
                let stem = rel
                    .rsplit('/')
                    .next()
                    .unwrap_or(&rel)
                    .trim_end_matches(".md")
                    .trim_end_matches(".MD");
                // A file is a subject: its own top title names it when it has one.
                let named = lines_of(without_front(&text))
                    .into_iter()
                    .find_map(|(_, t)| t.filter(|t| t.0 == 1).map(|t| t.1))
                    .unwrap_or_else(|| stem.replace(['-', '_'], " "));
                drafts.extend(cut(&text, &rel, Some(&named), &intro));
            }
        }
        drafts.truncate(MAX_PAGES);
        place(&mut drafts);
        let known: Vec<String> = self
            .spec_tree()
            .into_iter()
            .flat_map(|s| s.pages.into_iter().map(|p| p.path))
            .collect();
        for d in &mut drafts {
            d.exists = known.contains(&d.path);
        }
        Ok(drafts)
    }

    /// Writes the pages of `path`'s cut, those of `only` when given; a page already there is left
    /// alone. Each page cites the file it came from and keeps its place in the spec it came from.
    pub fn import(&self, path: &str, only: Option<&[String]>) -> Result<Imported> {
        let plan = self.import_plan(path)?;
        if plan.is_empty() {
            bail!(tr!(
                "« {path} » ne contient rien à importer.",
                "“{path}” has nothing to import."
            ));
        }
        let mut out = Imported {
            written: Vec::new(),
            skipped: Vec::new(),
        };
        for (i, d) in plan.into_iter().enumerate() {
            if only.is_some_and(|o| !o.contains(&d.path)) {
                continue;
            }
            if d.exists {
                out.skipped.push(d.path);
                continue;
            }
            let order = i as u32 + 1;
            self.spec_write(&d.path, |p: &mut Page| {
                p.title = d.title.clone();
                p.subject = d.subject.clone();
                p.sources = vec![d.source.clone()];
                p.order = order;
                p.body = d.body.clone();
            })?;
            out.written.push(d.path);
        }
        if !out.written.is_empty() {
            self.record_import(path)?;
        }
        Ok(out)
    }

    /// What was imported into this spec: the paths (files or folders) it came from, with the day.
    pub fn imported(&self) -> Vec<ImportedFrom> {
        self.raw_read(docs::SPEC, IMPORTS)
            .ok()
            .and_then(|t| serde_json::from_str::<ImportsFile>(&t).ok())
            .map(|f| f.imports)
            .unwrap_or_default()
    }

    /// Keeps `path` among the imports, once: a second import of it changes nothing. The file lives
    /// with the spec (`.escouade/spec/`), so the team's agents know it too. The original is not touched.
    fn record_import(&self, path: &str) -> Result<()> {
        let mut imports = self.imported();
        if imports.iter().any(|i| i.path == path) {
            return Ok(());
        }
        imports.push(ImportedFrom {
            path: path.to_string(),
            at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        });
        let text = serde_json::to_string_pretty(&ImportsFile { imports })?;
        self.raw_write(docs::SPEC, IMPORTS, &format!("{text}\n"))
    }

    /// What every agent of a project whose spec is in Escouade (one page at least) is told: the spec and
    /// the decisions are there and the only ones to follow, and the old spec files (`imported`) are not
    /// the spec any more. None for a project with no spec in Escouade.
    pub fn follow_rule(&self, lang: Lang) -> Option<String> {
        if self.spec_tree().is_empty() {
            return None;
        }
        let mut rule = tr_in!(
            lang,
            "La spec et les décisions de ce projet sont dans Escouade (get_spec_tree, read_spec_page, search_spec, \
             list_decisions, read_decision) : ce sont les seules à suivre.",
            "This project's spec and decisions are in Escouade (get_spec_tree, read_spec_page, search_spec, \
             list_decisions, read_decision): they are the only ones to follow."
        );
        let imports = self.imported();
        if !imports.is_empty() {
            let from = imports
                .iter()
                .map(|i| format!("{} ({})", i.path, i.at))
                .collect::<Vec<_>>()
                .join(", ");
            rule.push_str(&tr_in!(
                lang,
                " Ancienne spec, importée dans Escouade : {from} ; ne la lis pas comme spec, ne la mets pas à jour.",
                " Old spec, imported into Escouade: {from}; don’t read it as the spec, don’t update it."
            ));
        }
        Some(rule)
    }

    /// The Markdown files of a folder `detect` found, relative to the project.
    fn found_files(&self, folder: &str) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        let base = self.root.join(folder);
        fn go(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>, depth: usize) {
            if depth > 6 || out.len() >= 1000 {
                return;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let Ok(t) = e.file_type() else { continue };
                let name = e.file_name().to_string_lossy().to_string();
                if t.is_symlink() || name.starts_with('.') {
                    continue;
                }
                if t.is_dir() {
                    go(base, &e.path(), out, depth + 1);
                } else if t.is_file() && name.to_lowercase().ends_with(".md") {
                    if let Ok(r) = e.path().strip_prefix(base) {
                        out.push(r.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
        }
        go(&base, &base, &mut files, 0);
        files.into_iter().map(|f| format!("{folder}/{f}")).collect()
    }

    /// A file of a spec found in the project: no link on its way, inside the project.
    fn read_found(&self, rel: &str) -> Result<String> {
        if rel.split('/').any(|s| s == ".." || s.is_empty())
            || rel.contains(':')
            || rel.contains('\\')
        {
            return Err(refused(rel));
        }
        let mut at = self.root.clone();
        for seg in rel.split('/') {
            at.push(seg);
            if std::fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(refused(rel));
            }
        }
        let root = std::fs::canonicalize(&self.root)?;
        let real = std::fs::canonicalize(&at)?;
        if !real.starts_with(&root) {
            return Err(refused(rel));
        }
        // The same cap as a page (`docs::MAX_TEXT`); `take` holds it if the file grows meanwhile.
        let meta = std::fs::metadata(&real)?;
        if !meta.is_file() {
            return Err(refused(rel));
        }
        if meta.len() > docs::MAX_TEXT {
            bail!(tr!(
                "« {rel} » est trop gros pour être importé",
                "“{rel}” is too big to be imported"
            ));
        }
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(
            &mut std::io::Read::take(std::fs::File::open(&real)?, docs::MAX_TEXT),
            &mut bytes,
        )?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}
