//! The acceptance criteria of a ticket's description, as every source reads them.

/// At most this many criteria come from a description, each cut to `CRITERION_CHARS`.
pub const MAX_CRITERIA: usize = 20;
pub const CRITERION_CHARS: usize = 300;

/// The acceptance criteria a description lists: the items under a heading or a line that names
/// them (« Critères d'acceptation », « Acceptance criteria », « Definition of done »), else its
/// task list items (`- [ ]`); none when it has neither.
pub fn criteria_of(markdown: &str) -> Vec<String> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut found = Vec::new();
    if let Some(start) = lines.iter().position(|l| names_criteria(l)) {
        for line in &lines[start + 1..] {
            let t = line.trim();
            if t.starts_with('#') {
                break;
            }
            match list_item(t) {
                Some((text, _)) => found.push(text),
                // A sentence before the list is part of the section; one after it ends it.
                None if !t.is_empty() && !found.is_empty() => break,
                None => {}
            }
        }
    }
    if found.is_empty() {
        found = lines
            .iter()
            .filter_map(|l| list_item(l.trim()))
            .filter(|(_, task)| *task)
            .map(|(text, _)| text)
            .collect();
    }
    found
        .into_iter()
        .filter(|c| !c.is_empty())
        .take(MAX_CRITERIA)
        .map(|c| cut(&c, CRITERION_CHARS))
        .collect()
}

/// The criteria of a field that holds nothing else (Azure Boards' « Acceptance Criteria »), as
/// markdown: its list items, else its lines; a line that names them is left out.
pub fn criteria_in(markdown: &str) -> Vec<String> {
    let lines: Vec<&str> = markdown
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !names_criteria(l))
        .collect();
    let items: Vec<String> = lines
        .iter()
        .filter_map(|l| list_item(l))
        .map(|(text, _)| text)
        .collect();
    let found = if items.is_empty() {
        lines
            .iter()
            .map(|l| l.trim_start_matches('#').trim().to_string())
            .collect()
    } else {
        items
    };
    found
        .into_iter()
        .filter(|c| !c.is_empty())
        .take(MAX_CRITERIA)
        .map(|c| cut(&c, CRITERION_CHARS))
        .collect()
}

/// A list item's text, and whether it is a task (`- [ ]`, `- [x]`).
fn list_item(line: &str) -> Option<(String, bool)> {
    let rest = ["- ", "* ", "+ "]
        .iter()
        .find_map(|b| line.strip_prefix(b))
        .or_else(|| {
            let digits = line.chars().take_while(char::is_ascii_digit).count();
            (digits > 0)
                .then(|| &line[digits..])
                .and_then(|r| r.strip_prefix(". ").or_else(|| r.strip_prefix(") ")))
        })?;
    let rest = rest.trim();
    for box_ in ["[ ]", "[x]", "[X]"] {
        if let Some(text) = rest.strip_prefix(box_) {
            return Some((text.trim().to_string(), true));
        }
    }
    Some((rest.to_string(), false))
}

/// At most `max` characters, the last one an ellipsis when cut.
fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// A line that names the acceptance criteria (a heading, a bold line, a line ending with ":").
pub fn names_criteria(line: &str) -> bool {
    let t = line.trim();
    if list_item(t).is_some() {
        return false;
    }
    let name = t
        .trim_start_matches('#')
        .trim_matches(|c: char| matches!(c, '*' | '_' | ':' | ' '))
        .replace('’', "'");
    let slug = slug_within(&name, 40);
    [
        "criteres-d-accept",
        "critere-d-accept",
        "acceptance-criteri",
        "definition-of-done",
    ]
    .iter()
    .any(|p| slug.starts_with(p))
        && slug.split('-').count() <= 4
        || matches!(slug.as_str(), "ac" | "dod" | "criteres" | "criteria")
}

/// Kebab-case ASCII slug from free text (accents folded), of at most `max` characters: whole
/// words while they fit, the first one cut when it alone is longer.
fn slug_within(s: &str, max: usize) -> String {
    let folded: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let mut out = String::new();
    for part in folded.split('-').filter(|p| !p.is_empty()) {
        if out.len() + part.len() + 1 > max {
            if out.is_empty() {
                // ASCII only by now: any byte ends a character.
                out.push_str(&part[..max.min(part.len())]);
            }
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(part);
    }
    out
}
