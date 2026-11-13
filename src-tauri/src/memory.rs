//! The shared memory of a project's agents (ESC-255, `docs/FACTORY.md` § 2): their notebook of how to work in it,
//! the ways that work and the traps to avoid. Not the product: the spec and the decisions speak of the product.
//!
//! It lives in the project's folder, `.escouade/memory/` (in its first repository when it has several, as the shared
//! tickets), not in each agent's worktree: every agent of the project reads and writes the same one, whatever its
//! branch or its harness. One Markdown file per memory with a header (name, description, kind), and a short index,
//! `MEMORY.md`, written again by Escouade at each change, as Claude Code keeps its own: agents that know it find their
//! way. The team shares it by git; Escouade never commits it (`git::stage_all` leaves it out).
//!
//! Never read nor written through a symbolic link, nor out of the project (`workflowfiles::unsafe_dir`): a cloned
//! repository can hold a link to anywhere. Found by the words, and by the meaning when the vectors' model is there
//! (`vectors`, `search`).
//!
//! Claude Code's own project memory (`<its folder>/projects/<the project's path>/memory/`) is offered once per project
//! (`claude_offer`), memory by memory, and only what the user ticks is taken (`claude_take`): read, never written.

use crate::model::Project;
use crate::vectors::embed::Embedder;
use crate::vectors::index::{Index, Source};
use anyhow::{bail, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where the memory is, in the project's folder (or in its first repository).
pub const DIR: &str = ".escouade/memory";
/// Its index, written again at each change.
pub const INDEX: &str = "MEMORY.md";
/// A memory's file above this size is left unread.
const MAX_FILE: u64 = 256 * 1024;
/// The longest a memory's text may be, in characters.
pub const MAX_BODY: usize = 20_000;
/// The longest description, in characters (one line of the index).
pub const MAX_DESCRIPTION: usize = 300;
/// The longest name, in characters.
const MAX_NAME: usize = 64;

// What a memory is about: `escouade-mcp`, whose `write_memory` arguments name it.
pub use escouade_mcp::memory::Kind;

/// A memory: its file's header, and its text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    /// Its name, which is its file's (`<name>.md`): lower case, digits and dashes.
    pub name: String,
    /// One line that says what it is about, as the index lists it.
    pub description: String,
    pub kind: Kind,
    pub body: String,
}

/// The folder of the memory of `project`.
pub fn dir_of(project: &Project) -> PathBuf {
    crate::sharing::dir_of(project)
        .parent()
        .map_or_else(|| PathBuf::from(DIR), |p| p.join("memory"))
}

/// Why the folder `dir` is not to be read nor written, if it is not.
fn refused(dir: &Path) -> Result<()> {
    match crate::workflowfiles::unsafe_dir(dir) {
        Some(why) => bail!(why),
        None => Ok(()),
    }
}

/// The name `asked` as a file's: lower case letters and digits, the rest dashes, at most `MAX_NAME` characters.
pub fn name_of(asked: &str) -> Result<String> {
    let mut out = String::new();
    for c in asked.trim().chars().flat_map(char::to_lowercase) {
        let c = fold(c);
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let mut out: String = out.trim_end_matches('-').chars().take(MAX_NAME).collect();
    out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        bail!(tr!(
            "Donne un nom au souvenir (des lettres ou des chiffres).",
            "Give the memory a name (letters or digits)."
        ));
    }
    // A device's name under Windows, or the index's (`memory.md` is `MEMORY.md` there).
    if crate::workflowfiles::reserved_stem(&out) || out == "memory" {
        out.push_str("-note");
    }
    Ok(out)
}

/// A letter with an accent as the letter without it (the French ones and their neighbours).
fn fold(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' | 'ý' => 'y',
        'ñ' => 'n',
        'œ' => 'o',
        'æ' => 'a',
        c => c,
    }
}

/// The text of a header's value: on one line, quoted when YAML would read it otherwise.
fn header_value(s: &str) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let plain = !s.is_empty()
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.ends_with(':')
        && !s.starts_with(|c: char| "-?:,[]{}#&*!|>'\"%@`".contains(c));
    if plain {
        s
    } else {
        serde_json::to_string(&s).unwrap_or_default()
    }
}

/// A header's value as written: plain, or quoted (double, as JSON; single, YAML's `''`).
fn read_value(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') {
        if let Ok(v) = serde_json::from_str::<String>(s) {
            return v;
        }
    }
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        return s[1..s.len() - 1].replace("''", "'");
    }
    s.to_string()
}

/// The file of `m`: its header (as Claude Code writes its own), then its text.
pub fn render(m: &Memory) -> String {
    let body = m.body.trim();
    format!(
        "---\nname: {}\ndescription: {}\nmetadata:\n  type: {}\n---\n\n{}{}",
        m.name,
        header_value(&m.description),
        m.kind.as_str(),
        body,
        if body.is_empty() { "" } else { "\n" }
    )
}

/// A memory's file read: its header (`name`, `description`, and `type` at the top or under `metadata`, as Claude
/// Code wrote it in turn) and its text. A file without a header is all text, named `stem`.
pub fn parse(stem: &str, text: &str) -> Memory {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut m = Memory {
        name: stem.to_string(),
        description: String::new(),
        kind: Kind::default(),
        body: text.trim().to_string(),
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return m;
    }
    let mut header = Vec::new();
    let mut closed = false;
    for l in lines.by_ref() {
        if l.trim_end() == "---" {
            closed = true;
            break;
        }
        header.push(l);
    }
    if !closed {
        return m;
    }
    for l in header {
        let Some((key, value)) = l.split_once(':') else {
            continue;
        };
        let value = read_value(value);
        match key.trim() {
            "name" if !value.is_empty() => m.name = value,
            "description" => m.description = value,
            "type" => m.kind = Kind::parse(&value).unwrap_or_default(),
            _ => {}
        }
    }
    m.body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    m
}

/// Whether `path` is a plain file, not a link (`symlink_metadata` does not follow one).
fn plain_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file())
}

/// The `.md` files of `dir` but its index, by name: (stem, path). None when it is not there.
fn files(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let stem = name.strip_suffix(".md")?;
            // `file_type` of an entry does not follow a link: one is never read (it can point anywhere).
            (name != INDEX && e.file_type().is_ok_and(|t| t.is_file()))
                .then(|| (stem.to_string(), path))
        })
        .collect();
    out.sort();
    out
}

fn read_file(stem: &str, path: &Path) -> Option<Memory> {
    if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_FILE) {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let mut m = parse(stem, &text);
    // The file's name is the memory's: a header that says another is not followed.
    m.name = stem.to_string();
    Some(m)
}

/// The memories of `dir`, by name. None when it is not there; refused through a link or out of the project.
pub fn list(dir: &Path) -> Result<Vec<Memory>> {
    refused(dir)?;
    Ok(files(dir)
        .into_iter()
        .filter_map(|(stem, path)| read_file(&stem, &path))
        .collect())
}

/// The memory `name` of `dir`.
pub fn read(dir: &Path, name: &str) -> Result<Memory> {
    refused(dir)?;
    let name = name_of(name)?;
    let path = dir.join(format!("{name}.md"));
    if !plain_file(&path) {
        bail!(not_found(&name));
    }
    read_file(&name, &path).ok_or_else(|| anyhow::anyhow!(not_found(&name)))
}

fn not_found(name: &str) -> String {
    tr!(
        "Aucun souvenir « {name} » dans la mémoire du projet.",
        "No memory “{name}” in the project's memory."
    )
}

/// What `write` is given.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub name: String,
    pub description: String,
    pub kind: Kind,
    pub body: String,
}

/// Writes the memory `d` in `dir` (made if need be), in place of the one of its name if there is one, and the index
/// again. Answers the memory as written, and whether it is new.
pub fn write(dir: &Path, d: Draft) -> Result<(Memory, bool)> {
    refused(dir)?;
    let name = name_of(&d.name)?;
    let description = d
        .description
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if description.is_empty() {
        bail!(tr!(
            "Donne une description d'une ligne au souvenir.",
            "Give the memory a one-line description."
        ));
    }
    if description.chars().count() > MAX_DESCRIPTION {
        bail!(tr!(
            "La description dépasse {max} caractères : garde-la sur une ligne courte.",
            "The description is over {max} characters: keep it to a short line.",
            max = MAX_DESCRIPTION
        ));
    }
    let body = d.body.trim().to_string();
    if body.chars().count() > MAX_BODY {
        bail!(tr!(
            "Le souvenir dépasse {max} caractères : coupe-le en plusieurs.",
            "The memory is over {max} characters: split it.",
            max = MAX_BODY
        ));
    }
    std::fs::create_dir_all(dir)?;
    // Made now: the folder it is in can be told only once it is there.
    refused(dir)?;
    let path = dir.join(format!("{name}.md"));
    let there = std::fs::symlink_metadata(&path);
    if there.as_ref().is_ok_and(|m| !m.file_type().is_file()) {
        bail!(tr!(
            "{path} n'est pas un fichier (un lien symbolique ?) : ignoré",
            "{path} is not a file (a symbolic link?): ignored",
            path = path.display()
        ));
    }
    let m = Memory {
        name,
        description,
        kind: d.kind,
        body,
    };
    crate::paths::write_atomic(&path, render(&m).as_bytes())?;
    write_index(dir)?;
    Ok((m, there.is_err()))
}

/// Forgets the memory `name` of `dir`: its file goes, and the index is written again. False when there was none.
pub fn forget(dir: &Path, name: &str) -> Result<bool> {
    refused(dir)?;
    let name = name_of(name)?;
    let path = dir.join(format!("{name}.md"));
    if !plain_file(&path) {
        return Ok(false);
    }
    std::fs::remove_file(&path)?;
    write_index(dir)?;
    Ok(true)
}

/// The index of the memories of `dir`: a line each, by name, its link and its description.
pub fn index_text(memories: &[Memory]) -> String {
    let mut out = String::from(
        "# Mémoire du projet\n\n\
         Le carnet de travail des agents d'Escouade : un souvenir par fichier, cet index réécrit à chaque changement.\n\n",
    );
    for m in memories {
        out.push_str(&format!(
            "- [{}]({}.md) — {}\n",
            m.name, m.name, m.description
        ));
    }
    out
}

fn write_index(dir: &Path) -> Result<()> {
    let path = dir.join(INDEX);
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.file_type().is_file()) {
        bail!(tr!(
            "{path} n'est pas un fichier (un lien symbolique ?) : ignoré",
            "{path} is not a file (a symbolic link?): ignored",
            path = path.display()
        ));
    }
    let memories = list(dir)?;
    if memories.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    crate::paths::write_atomic(&path, index_text(&memories).as_bytes())?;
    Ok(())
}

/// A memory found by `search`: what the agent reads to choose the ones to read whole.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub name: String,
    pub description: String,
    pub kind: Kind,
    /// The first lines of its text.
    pub excerpt: String,
}

/// The words of `s`, lower case, without accents.
fn words(s: &str) -> Vec<String> {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(fold)
        .collect::<String>()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}

fn excerpt(body: &str) -> String {
    let one: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= 240 {
        return one;
    }
    format!("{}…", one.chars().take(240).collect::<String>())
}

fn found(m: &Memory) -> Found {
    Found {
        name: m.name.clone(),
        description: m.description.clone(),
        kind: m.kind,
        excerpt: excerpt(&m.body),
    }
}

/// The memories that hold the words of `query`, best first: a word in the name counts most, then in the description,
/// then in the text; every word found counts more than one found often. An empty query lists them all.
pub fn search_words(memories: &[Memory], query: &str, limit: usize) -> Vec<Found> {
    let asked = words(query);
    if asked.is_empty() {
        return memories.iter().take(limit).map(found).collect();
    }
    let mut scored: Vec<(f32, &Memory)> = memories
        .iter()
        .filter_map(|m| {
            let (name, desc, body) = (words(&m.name), words(&m.description), words(&m.body));
            let mut score = 0.0;
            let mut hit = 0;
            for w in &asked {
                let has = |ws: &[String]| ws.iter().any(|x| x.starts_with(w.as_str()));
                let s = 3.0 * f32::from(u8::from(has(&name)))
                    + 2.0 * f32::from(u8::from(has(&desc)))
                    + f32::from(u8::from(has(&body)));
                if s > 0.0 {
                    hit += 1;
                    score += s;
                }
            }
            (hit > 0).then_some((score + 10.0 * hit as f32, m))
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.name.cmp(&b.1.name)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, m)| found(m))
        .collect()
}

/// The memories nearest `query` by the meaning and the words together (`Index::search_hybrid`), the index brought up
/// to date first (a memory changed is read again, one forgotten leaves it). Best first, each once.
pub fn search_meaning(
    index: &mut Index,
    memories: &[Memory],
    query: &str,
    limit: usize,
) -> Result<Vec<Found>> {
    for m in memories {
        index.put(Source::Memory, &m.name, &render(m))?;
    }
    let keep: Vec<&str> = memories.iter().map(|m| m.name.as_str()).collect();
    index.retain(Source::Memory, &keep)?;
    let hits = index.search_hybrid(query, limit * 3, &[Source::Memory])?;
    let mut out: Vec<Found> = Vec::new();
    for h in hits {
        if out.iter().any(|f| f.name == h.path) {
            continue;
        }
        if let Some(m) = memories.iter().find(|m| m.name == h.path) {
            out.push(found(m));
        }
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

/// The search of `search_memory`: by the meaning too when `embedder` is given (the vectors' model is there), its
/// index at `index_path`; by the words otherwise, or when the meaning fails. Answers how it searched too.
pub fn search(
    dir: &Path,
    query: &str,
    limit: usize,
    vectors: Option<(Arc<dyn Embedder>, &Path)>,
) -> Result<(Vec<Found>, &'static str)> {
    let memories = list(dir)?;
    let limit = limit.clamp(1, 50);
    if let Some((embedder, path)) = vectors.filter(|_| !words(query).is_empty()) {
        match Index::open(path, embedder)
            .and_then(|mut i| search_meaning(&mut i, &memories, query, limit))
        {
            Ok(found) => return Ok((found, "meaning")),
            Err(e) => log::warn!("memory: search by meaning failed, by words: {e:#}"),
        }
    }
    Ok((search_words(&memories, query, limit), "words"))
}

/// The folder Claude Code keeps a project's memory in, under its configuration folder `config`: `projects/<path>/memory`,
/// the project's path with every character but a letter or a digit as a dash (as Claude Code names it).
pub fn claude_dir(config: &Path, project_path: &str) -> PathBuf {
    let key: String = project_path
        .trim_end_matches(['/', '\\'])
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    config.join("projects").join(key).join("memory")
}

/// A memory of Claude Code's offered to the project's: what it says, and whether the project has one of that name.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offered {
    #[serde(flatten)]
    pub memory: Memory,
    /// The project's memory already has one of this name: taking it replaces it.
    pub exists: bool,
}

/// The memories Claude Code keeps for the project in the folders `dirs` (one per account's configuration folder), each
/// name once (the first folder's), read only: never a link, never written. `own` is the project's memory folder.
pub fn claude_offer(dirs: &[PathBuf], own: &Path) -> Vec<Offered> {
    let have: Vec<String> = list(own)
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.name)
        .collect();
    let mut out: Vec<Offered> = Vec::new();
    for dir in dirs {
        // The folder itself, or one above it up to `projects`, a link: not followed.
        if [dir.as_path(), dir.parent().unwrap_or(dir)]
            .iter()
            .any(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
        {
            continue;
        }
        for (stem, path) in files(dir) {
            let Some(mut m) = read_file(&stem, &path) else {
                continue;
            };
            let Ok(name) = name_of(&stem) else {
                continue;
            };
            m.name = name;
            if m.description.is_empty() {
                m.description = excerpt(&m.body).chars().take(120).collect();
            }
            if m.body.is_empty() || out.iter().any(|o| o.memory.name == m.name) {
                continue;
            }
            let exists = have.contains(&m.name);
            out.push(Offered { memory: m, exists });
        }
    }
    out.sort_by(|a, b| a.memory.name.cmp(&b.memory.name));
    out
}

/// Writes into `own` the memories of `offered` whose names are in `names`. Answers those written.
pub fn claude_take(offered: &[Offered], names: &[String], own: &Path) -> Result<Vec<Memory>> {
    let mut out = Vec::new();
    for o in offered.iter().filter(|o| names.contains(&o.memory.name)) {
        let m = &o.memory;
        let (written, _) = write(
            own,
            Draft {
                name: m.name.clone(),
                description: m.description.chars().take(MAX_DESCRIPTION).collect(),
                kind: m.kind,
                body: m.body.chars().take(MAX_BODY).collect(),
            },
        )?;
        out.push(written);
    }
    Ok(out)
}

/// What every agent of Escouade is told of the memory, and of the documents of the drive (ESC-260), added to its
/// instructions at its making.
pub fn instruction(lang: crate::i18n::Lang) -> String {
    tr_in!(
        lang,
        "Le projet a une mémoire partagée par tous ses agents, quel que soit leur harness : le carnet de travail \
         (façons de faire, écueils à éviter), pas le produit. Au départ, cherche-y ce qui touche ta tâche \
         (search_memory, read_memory d'Escouade) ; quand tu apprends comment travailler dans ce projet, note-le \
         (write_memory), et oublie ce qui s'avère faux (forget_memory). Les documents du projet (cahiers des charges, \
         comptes rendus…) se cherchent et se lisent par search_documents et read_document ; cite ceux d'où vient ce que \
         tu écris dans la spec (sources de write_spec_page).",
        "The project has a memory shared by all its agents, whatever their harness: the work notebook \
         (ways of doing, traps to avoid), not the product. At the start, look in it for what touches your task \
         (Escouade's search_memory, read_memory); when you learn how to work in this project, note it \
         (write_memory), and forget what turns out wrong (forget_memory). The project's documents (requirements, \
         meeting notes…) are searched and read by search_documents and read_document; cite those what you write in the \
         spec comes from (sources of write_spec_page)."
    )
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
