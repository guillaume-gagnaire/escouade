//! Cutting a document into pieces that each say one thing: Markdown by its headings, text by its paragraphs, code by its
//! blocks (the lines between two blank ones). A piece holds about `TARGET` characters, never more than `MAX`.

/// A piece is filled up to this many characters by joining small neighbours…
const TARGET: usize = 900;
/// …and never goes past this one (about 300 tokens: the models read 512, and a long piece blurs its meaning).
const MAX: usize = 1400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Text,
    Code,
}

impl Format {
    pub fn of_path(path: &str) -> Self {
        let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
        match ext.as_str() {
            // Word and OpenDocument read as Markdown (their headings made titles, `docsextract`).
            "md" | "markdown" | "mdx" | "docx" | "odt" => Self::Markdown,
            "txt" | "text" | "rst" | "adoc" | "pdf" | "" => Self::Text,
            _ => Self::Code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// The titles above the piece, `A › B` (Markdown only): what the piece is about, read with it.
    pub title: String,
    pub text: String,
}

impl Chunk {
    /// What is given to the model: the titles and the text.
    pub fn embedded(&self) -> String {
        if self.title.is_empty() {
            self.text.clone()
        } else {
            format!("{}\n{}", self.title, self.text)
        }
    }
}

pub fn chunks(text: &str, format: Format) -> Vec<Chunk> {
    let text = text.replace("\r\n", "\n");
    match format {
        Format::Markdown => markdown(&text),
        Format::Text => pack(paragraphs(&text), ""),
        Format::Code => pack(blocks(&text), ""),
    }
}

/// The blocks of `text`, cut at the blank lines (not those inside a code fence).
fn paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if line.trim().is_empty() && !fenced {
            if !cur.trim().is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            cur.clear();
            continue;
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out.into_iter().map(|p| p.trim_end().to_string()).collect()
}

/// Code: the same, but a block that starts less indented than the last one ends it too would need a parser;
/// blank lines are what most code uses between its functions.
fn blocks(text: &str) -> Vec<String> {
    paragraphs(text)
}

/// Pieces of at most `MAX` characters from `parts`, small neighbours joined up to `TARGET`, a big part cut at its lines.
fn pack(parts: Vec<String>, title: &str) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut Vec<Chunk>| {
        if !cur.trim().is_empty() {
            out.push(Chunk {
                title: title.into(),
                text: std::mem::take(cur).trim().to_string(),
            });
        }
        cur.clear();
    };
    for part in parts {
        for piece in cut(&part) {
            let joined = cur.chars().count() + piece.chars().count() + 2;
            if !cur.is_empty() && joined > TARGET {
                flush(&mut cur, &mut out);
            }
            if !cur.is_empty() {
                cur.push_str("\n\n");
            }
            cur.push_str(&piece);
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// `part` as it is if it is short, else cut at its lines, and its words if a line is longer than `MAX`.
fn cut(part: &str) -> Vec<String> {
    if part.chars().count() <= MAX {
        return vec![part.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for line in part.lines() {
        for piece in split_words(line) {
            if !cur.is_empty() && cur.chars().count() + piece.chars().count() + 1 > MAX {
                out.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push('\n');
            }
            cur.push_str(&piece);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn split_words(line: &str) -> Vec<String> {
    if line.chars().count() <= MAX {
        return vec![line.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for w in line.split(' ') {
        if !cur.is_empty() && cur.chars().count() + w.chars().count() + 1 > MAX {
            out.push(std::mem::take(&mut cur));
        }
        // A "word" longer than a piece (a minified file, a base64): cut anywhere.
        let mut w = w;
        while w.chars().count() > MAX {
            let at = w.char_indices().nth(MAX).map_or(w.len(), |(i, _)| i);
            out.push(w[..at].to_string());
            w = &w[at..];
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Markdown: a piece never crosses a heading; each is written with the titles above it.
fn markdown(text: &str) -> Vec<Chunk> {
    // (level, title) of the headings above the current line.
    let mut path: Vec<(usize, String)> = Vec::new();
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut body = String::new();
    let mut title = String::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        let level = line.chars().take_while(|&c| c == '#').count();
        let is_heading = !fenced
            && (1..=6).contains(&level)
            && line[level..].starts_with(' ')
            && !line[level..].trim().is_empty();
        if is_heading {
            sections.push((std::mem::take(&mut title), std::mem::take(&mut body)));
            let name = line[level..]
                .trim()
                .trim_end_matches('#')
                .trim()
                .to_string();
            path.retain(|(l, _)| *l < level);
            path.push((level, name));
            title = path
                .iter()
                .map(|(_, n)| n.as_str())
                .collect::<Vec<_>>()
                .join(" › ");
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    sections.push((title, body));
    sections
        .into_iter()
        .flat_map(|(title, body)| pack(paragraphs(&body), &title))
        .collect()
}
