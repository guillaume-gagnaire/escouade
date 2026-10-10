//! The texts of external tickets: Jira's documents (ADF) as markdown, the acceptance criteria a
//! description lists, the states that look like the board's columns, and the comments the sync
//! writes.

use crate::core::slugify;
use crate::i18n::Lang;
use crate::model::{Column, ExternalState, Ticket};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// At most this many criteria come from a description, each cut to `CRITERION_CHARS`.
const MAX_CRITERIA: usize = 20;
const CRITERION_CHARS: usize = 300;

/// A Jira document (Atlassian Document Format) as markdown: paragraphs, headings, lists, task
/// lists, code, quotes, tables; what has no text (images) is left out.
pub fn adf_to_markdown(doc: &Value) -> String {
    let mut blocks = Vec::new();
    adf_blocks(doc, "", &mut blocks);
    blocks.join("\n\n")
}

/// The blocks of `node`'s content, each as markdown, the lines of list items under `indent`.
fn adf_blocks(node: &Value, indent: &str, out: &mut Vec<String>) {
    for n in children(node) {
        let block = match kind(n) {
            "paragraph" => inline(n),
            "heading" => {
                let level = n["attrs"]["level"].as_u64().unwrap_or(1).clamp(1, 6) as usize;
                format!("{} {}", "#".repeat(level), inline(n))
            }
            "bulletList" | "orderedList" | "taskList" => list(n, indent),
            "codeBlock" => format!(
                "```{}\n{}\n```",
                n["attrs"]["language"].as_str().unwrap_or(""),
                inline(n)
            ),
            "blockquote" => {
                let mut inner = Vec::new();
                adf_blocks(n, "", &mut inner);
                inner
                    .join("\n\n")
                    .lines()
                    .map(|l| format!("> {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "rule" => "---".into(),
            "table" => children(n)
                .map(|row| {
                    let cells: Vec<String> = children(row)
                        .map(|cell| {
                            let mut inner = Vec::new();
                            adf_blocks(cell, "", &mut inner);
                            inner.join(" ").replace('\n', " ")
                        })
                        .collect();
                    format!("| {} |", cells.join(" | "))
                })
                .collect::<Vec<_>>()
                .join("\n"),
            "mediaSingle" | "mediaGroup" | "media" => String::new(),
            // Panels, expands, layouts…: their own blocks.
            _ => {
                adf_blocks(n, indent, out);
                continue;
            }
        };
        if !block.trim().is_empty() {
            out.push(block);
        }
    }
}

fn children(node: &Value) -> impl Iterator<Item = &Value> {
    node["content"].as_array().into_iter().flatten()
}

fn kind(node: &Value) -> &str {
    node["type"].as_str().unwrap_or("")
}

/// A list's items, one per line, a nested list's under them, indented.
fn list(node: &Value, indent: &str) -> String {
    let ordered = kind(node) == "orderedList";
    let start = node["attrs"]["order"].as_u64().unwrap_or(1);
    let mut lines = Vec::new();
    for (i, item) in children(node).enumerate() {
        let bullet = match (kind(item), item["attrs"]["state"].as_str()) {
            ("taskItem", Some("DONE")) => "- [x] ".to_string(),
            ("taskItem", _) => "- [ ] ".to_string(),
            _ if ordered => format!("{}. ", start + i as u64),
            _ => "- ".to_string(),
        };
        let deeper = format!("{indent}  ");
        let mut text = String::new();
        let mut nested = Vec::new();
        if kind(item) == "taskItem" {
            text = inline(item);
        }
        for part in children(item) {
            match kind(part) {
                "bulletList" | "orderedList" | "taskList" => nested.push(list(part, &deeper)),
                "paragraph" if text.is_empty() => text = inline(part),
                "paragraph" => nested.push(format!("{deeper}{}", inline(part))),
                _ => {}
            }
        }
        lines.push(format!("{indent}{bullet}{text}"));
        lines.extend(nested);
    }
    lines.join("\n")
}

/// The text of a node's inline content, with its bold, code and links.
fn inline(node: &Value) -> String {
    let mut s = String::new();
    for n in children(node) {
        let attrs = &n["attrs"];
        match kind(n) {
            "text" => {
                let mut t = n["text"].as_str().unwrap_or("").to_string();
                for m in n["marks"].as_array().into_iter().flatten() {
                    t = match kind(m) {
                        "strong" => format!("**{t}**"),
                        "code" => format!("`{t}`"),
                        "link" => match m["attrs"]["href"].as_str() {
                            Some(href) => format!("[{t}]({href})"),
                            None => t,
                        },
                        _ => t,
                    };
                }
                s.push_str(&t);
            }
            "hardBreak" => s.push('\n'),
            "mention" | "emoji" | "status" => s.push_str(
                attrs["text"]
                    .as_str()
                    .or(attrs["shortName"].as_str())
                    .unwrap_or(""),
            ),
            "inlineCard" => s.push_str(attrs["url"].as_str().unwrap_or("")),
            _ => s.push_str(&inline(n)),
        }
    }
    s
}

/// A text as a Jira document: a paragraph per block of lines, its lines kept apart.
pub fn markdown_to_adf(text: &str) -> Value {
    let paragraphs: Vec<Value> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut content = Vec::new();
            for (i, line) in p.lines().enumerate() {
                if i > 0 {
                    content.push(json!({ "type": "hardBreak" }));
                }
                if !line.is_empty() {
                    content.push(json!({ "type": "text", "text": line }));
                }
            }
            json!({ "type": "paragraph", "content": content })
        })
        .collect();
    json!({ "type": "doc", "version": 1, "content": paragraphs })
}

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
    let slug = slugify(&name);
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

/// What looks like « en cours », then « à tester », by priority.
const DOING: &[&str] = &[
    "in-progress",
    "en-cours",
    "doing",
    "wip",
    "in-development",
    "en-developpement",
    "progress",
    "started",
];
const TO_TEST: &[&str] = &[
    "a-tester",
    "to-test",
    "to-be-tested",
    "testing",
    "qa",
    "in-review",
    "review",
    "recette",
    "a-valider",
    "validation",
    "test",
];

/// The first state whose name holds one of `patterns` (as whole words), in their order.
fn looks_like<'a>(states: &'a [ExternalState], patterns: &[&str]) -> Option<&'a ExternalState> {
    patterns.iter().find_map(|p| {
        states.iter().find(|s| {
            let slug = slugify(&s.name);
            slug == *p
                || slug.starts_with(&format!("{p}-"))
                || slug.ends_with(&format!("-{p}"))
                || slug.contains(&format!("-{p}-"))
        })
    })
}

/// The states an external ticket gets by default when it comes into each column: the one that
/// looks like « en cours » for "En cours" and "À tester", the one that looks like « à tester » for
/// "Terminé"; "À faire" leaves it as it is.
pub fn default_states(states: &[ExternalState]) -> BTreeMap<Column, ExternalState> {
    let mut out = BTreeMap::new();
    let doing = looks_like(states, DOING);
    if let Some(d) = doing {
        out.insert(Column::Doing, d.clone());
        out.insert(Column::Review, d.clone());
    }
    let others: Vec<ExternalState> = states
        .iter()
        .filter(|s| Some(*s) != doing)
        .cloned()
        .collect();
    if let Some(t) = looks_like(&others, TO_TEST) {
        out.insert(Column::Done, t.clone());
    }
    out
}

/// The criteria with their state, one per line (✓ met, ○ not yet, and the agent's note).
fn criteria_lines(t: &Ticket) -> String {
    t.criteria
        .iter()
        .map(|c| {
            let mark = if c.ok { '✓' } else { '○' };
            if c.note.is_empty() {
                format!("{mark} {}", c.text)
            } else {
                format!("{mark} {} — {}", c.text, c.note)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// What the sync writes on the external ticket when its ticket comes into `column` (`branch`: its
/// agent's, once it has one), in `lang` (the language of the texts for Claude: what is published
/// for the team). The outcome of a ticket done is quoted as it was written in that language
/// (`outcome_claude`), the card's when the ticket has no other.
pub fn column_comment(lang: Lang, t: &Ticket, column: Column, branch: Option<&str>) -> String {
    let key = &t.key;
    match column {
        Column::Todo => tr_in!(
            lang,
            "Escouade : {key} est revenu « À faire ».",
            "Escouade: {key} is back in “To do”."
        ),
        Column::Doing => match branch {
            Some(b) => tr_in!(
                lang,
                "Escouade : {key} est pris par un agent (branche {b}).",
                "Escouade: {key} is picked up by an agent (branch {b})."
            ),
            None => tr_in!(
                lang,
                "Escouade : {key} est pris par un agent.",
                "Escouade: {key} is picked up by an agent."
            ),
        },
        Column::Review => {
            let mut s = if t.partial {
                tr_in!(
                    lang,
                    "Escouade : {key} est prêt à tester (objectif partiel).",
                    "Escouade: {key} is ready to review (partial goal)."
                )
            } else {
                tr_in!(
                    lang,
                    "Escouade : {key} est prêt à tester.",
                    "Escouade: {key} is ready to review."
                )
            };
            if !t.criteria.is_empty() {
                let lines = criteria_lines(t);
                s.push_str(&tr_in!(
                    lang,
                    "\n\nCritères :\n{lines}",
                    "\n\nCriteria:\n{lines}"
                ));
            }
            if !t.progress.is_empty() {
                let done: Vec<String> = t.progress.iter().map(|p| format!("- {p}")).collect();
                let done = done.join("\n");
                s.push_str(&tr_in!(
                    lang,
                    "\n\nCe qui a été fait :\n{done}",
                    "\n\nWhat was done:\n{done}"
                ));
            }
            s
        }
        Column::Done => {
            // Its outcome in `lang`'s language (a ticket done before 1.7 only has the card's).
            let outcome = t
                .outcome_claude
                .as_deref()
                .filter(|o| !o.is_empty())
                .or(t.outcome.as_deref().filter(|o| !o.is_empty()));
            let mut s = match outcome {
                Some(o) => tr_in!(
                    lang,
                    "Escouade : {key} est terminé — {o}",
                    "Escouade: {key} is done — {o}"
                ),
                None => tr_in!(
                    lang,
                    "Escouade : {key} est terminé.",
                    "Escouade: {key} is done."
                ),
            };
            if let Some(url) = t.outcome_url.as_deref().filter(|u| !u.is_empty()) {
                s.push_str(&format!("\n{url}"));
            }
            s
        }
    }
}

/// What "Publier un résumé à chaque boucle" writes when the ticket begins a loop, in `lang`.
pub fn loop_comment(lang: Lang, t: &Ticket) -> String {
    let met = t.criteria.iter().filter(|c| c.ok).count();
    tr_in!(
        lang,
        "Escouade : {key}, boucle {i}/{max} — {met}/{n} critères atteints.\n\n{lines}",
        "Escouade: {key}, loop {i}/{max} — {met}/{n} criteria met.\n\n{lines}",
        key = t.key,
        i = t.iteration,
        max = t.max_loops,
        n = t.criteria.len(),
        lines = criteria_lines(t)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Criterion;

    fn state(id: &str, name: &str) -> ExternalState {
        ExternalState {
            id: id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn a_jira_document_reads_as_markdown() {
        let doc = json!({ "type": "doc", "version": 1, "content": [
            { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Contexte" }] },
            { "type": "paragraph", "content": [
                { "type": "text", "text": "Le token " },
                { "type": "text", "text": "expire", "marks": [{ "type": "strong" }] },
                { "type": "text", "text": " trop tôt" },
                { "type": "hardBreak" },
                { "type": "mention", "attrs": { "text": "@Ada" } },
                { "type": "text", "text": " voir " },
                { "type": "text", "text": "la doc", "marks": [{ "type": "link", "attrs": { "href": "https://x.dev" } }] },
            ]},
            { "type": "bulletList", "content": [
                { "type": "listItem", "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "un" }] },
                    { "type": "orderedList", "content": [
                        { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "sous" }] }] }
                    ]}
                ]},
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "deux" }] }] },
            ]},
            { "type": "taskList", "content": [
                { "type": "taskItem", "attrs": { "state": "DONE" }, "content": [{ "type": "text", "text": "fait" }] },
                { "type": "taskItem", "attrs": { "state": "TODO" }, "content": [{ "type": "text", "text": "à faire" }] },
            ]},
            { "type": "codeBlock", "attrs": { "language": "ts" }, "content": [{ "type": "text", "text": "const a = 1;" }] },
            { "type": "mediaSingle", "content": [{ "type": "media", "attrs": { "id": "x" } }] },
            { "type": "panel", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Note" }] }] },
        ]});
        assert_eq!(
            adf_to_markdown(&doc),
            "## Contexte\n\n\
             Le token **expire** trop tôt\n@Ada voir [la doc](https://x.dev)\n\n\
             - un\n  1. sous\n- deux\n\n\
             - [x] fait\n- [ ] à faire\n\n\
             ```ts\nconst a = 1;\n```\n\n\
             Note"
        );
        assert_eq!(adf_to_markdown(&Value::Null), "");
    }

    #[test]
    fn a_text_becomes_a_jira_document_with_its_lines_kept() {
        let doc = markdown_to_adf("Escouade : prêt\n\n✓ un\n○ deux");
        assert_eq!(
            doc,
            json!({ "type": "doc", "version": 1, "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "Escouade : prêt" }] },
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "✓ un" }, { "type": "hardBreak" }, { "type": "text", "text": "○ deux" }
                ]},
            ]})
        );
    }

    #[test]
    fn the_criteria_are_the_items_under_the_line_that_names_them() {
        let md = "Contexte du ticket.\n\n- pas un critère\n\n## Critères d'acceptation\n\nCe qui compte :\n\n- Refresh avant expiration\n* Aucune erreur 401 visible\n\n1. Tests unitaires\n\n## Notes\n- rien";
        assert_eq!(
            criteria_of(md),
            [
                "Refresh avant expiration",
                "Aucune erreur 401 visible",
                "Tests unitaires"
            ]
        );
        let jira = "**Acceptance criteria:**\n- [ ] Emails avec + acceptés\n- [x] Test de non-régression\nMerci";
        assert_eq!(
            criteria_of(jira),
            ["Emails avec + acceptés", "Test de non-régression"]
        );
        let dod = "Definition of Done\n+ Doc OpenAPI";
        assert_eq!(criteria_of(dod), ["Doc OpenAPI"]);
    }

    #[test]
    fn without_such_a_line_the_criteria_are_the_task_list_items() {
        let md = "Faire la page.\n\n- une puce\n- [ ] Liste filtrable\n  - [X] Page détail\n";
        assert_eq!(criteria_of(md), ["Liste filtrable", "Page détail"]);
        assert!(criteria_of("Juste du texte.\n- une puce").is_empty());
        assert!(criteria_of("").is_empty());
    }

    #[test]
    fn the_criteria_are_counted_and_cut() {
        let long = "x".repeat(400);
        let mut md = String::from("## Acceptance criteria\n");
        for i in 0..30 {
            md.push_str(&format!("- {i} {long}\n"));
        }
        let c = criteria_of(&md);
        assert_eq!(c.len(), 20);
        assert_eq!(c[0].chars().count(), 300);
        assert!(c[0].ends_with('…'));
    }

    #[test]
    fn lines_that_name_the_criteria() {
        for l in [
            "## Critères d'acceptation",
            "Critères d’acceptation :",
            "**Acceptance Criteria:**",
            "*Definition of done*",
            "AC:",
            "Critères",
        ] {
            assert!(names_criteria(l), "{l}");
        }
        for l in [
            "Contexte",
            "- Critères d'acceptation clairs",
            "Le critère principal est X",
        ] {
            assert!(!names_criteria(l), "{l}");
        }
    }

    #[test]
    fn the_default_states_look_like_in_progress_then_to_test() {
        let jira = [
            state("1", "To Do"),
            state("3", "In Progress"),
            state("4", "In Review"),
            state("5", "Done"),
        ];
        let d = default_states(&jira);
        assert_eq!(d.get(&Column::Todo), None);
        assert_eq!(d[&Column::Doing].id, "3");
        assert_eq!(d[&Column::Review].id, "3");
        assert_eq!(d[&Column::Done].id, "4");
        let trello = [
            state("a", "Backlog"),
            state("b", "En cours"),
            state("c", "À tester"),
            state("d", "Terminé"),
        ];
        let d = default_states(&trello);
        assert_eq!(
            (d[&Column::Doing].id.as_str(), d[&Column::Done].id.as_str()),
            ("b", "c")
        );
        let github = [
            state("open", "Ouverte"),
            state("closed", "Fermée"),
            state("label:bug", "bug"),
        ];
        assert!(default_states(&github).is_empty());
        let only_progress = [state("1", "To Do"), state("2", "Doing")];
        let d = default_states(&only_progress);
        assert_eq!(d.len(), 2);
        assert_eq!(d.get(&Column::Done), None);
    }

    fn ticket() -> Ticket {
        Ticket {
            key: "ESC-12".into(),
            title: "Limiter les tentatives".into(),
            iteration: 2,
            max_loops: 5,
            criteria: vec![
                Criterion {
                    text: "Blocage après 5 essais".into(),
                    ok: true,
                    note: String::new(),
                },
                Criterion {
                    text: "Tests verts".into(),
                    ok: false,
                    note: "reste un test".into(),
                },
            ],
            progress: vec!["Compteur en base".into(), "Message d'erreur".into()],
            ..Default::default()
        }
    }

    #[test]
    fn each_column_has_its_comment() {
        let mut t = ticket();
        assert_eq!(
            column_comment(Lang::Fr, &t, Column::Doing, Some("ticket/esc-12")),
            "Escouade : ESC-12 est pris par un agent (branche ticket/esc-12)."
        );
        assert_eq!(
            column_comment(Lang::Fr, &t, Column::Review, None),
            "Escouade : ESC-12 est prêt à tester.\n\n\
             Critères :\n✓ Blocage après 5 essais\n○ Tests verts — reste un test\n\n\
             Ce qui a été fait :\n- Compteur en base\n- Message d'erreur"
        );
        t.partial = true;
        assert!(column_comment(Lang::Fr, &t, Column::Review, None)
            .starts_with("Escouade : ESC-12 est prêt à tester (objectif partiel)."));
        t.outcome = Some("⇡ PR #12 → main".into());
        t.outcome_url = Some("https://github.com/acme/api/pull/12".into());
        assert_eq!(
            column_comment(Lang::Fr, &t, Column::Done, None),
            "Escouade : ESC-12 est terminé — ⇡ PR #12 → main\nhttps://github.com/acme/api/pull/12"
        );
        t.outcome_url = None;
        assert_eq!(
            column_comment(Lang::Fr, &t, Column::Done, None),
            "Escouade : ESC-12 est terminé — ⇡ PR #12 → main"
        );
        assert_eq!(
            column_comment(Lang::Fr, &t, Column::Todo, None),
            "Escouade : ESC-12 est revenu « À faire »."
        );
    }

    #[test]
    fn a_done_comment_quotes_the_outcome_written_in_its_own_language() {
        let mut t = ticket();
        t.outcome = Some("⤵ Mergé dans main · squash".into());
        // Done before 1.7, a ticket has only the card's outcome: quoted as it is.
        assert_eq!(
            column_comment(Lang::En, &t, Column::Done, None),
            "Escouade: ESC-12 is done — ⤵ Mergé dans main · squash"
        );
        t.outcome_claude = Some(String::new());
        assert_eq!(
            column_comment(Lang::En, &t, Column::Done, None),
            "Escouade: ESC-12 is done — ⤵ Mergé dans main · squash"
        );
        // With the other, no language is mixed with the comment's.
        t.outcome_claude = Some("⤵ Merged into main · squash".into());
        assert_eq!(
            column_comment(Lang::En, &t, Column::Done, None),
            "Escouade: ESC-12 is done — ⤵ Merged into main · squash"
        );
        t.outcome = None;
        assert_eq!(
            column_comment(Lang::En, &t, Column::Done, None),
            "Escouade: ESC-12 is done — ⤵ Merged into main · squash"
        );
    }

    #[test]
    fn a_loop_summary_counts_the_criteria_met() {
        assert_eq!(
            loop_comment(Lang::Fr, &ticket()),
            "Escouade : ESC-12, boucle 2/5 — 1/2 critères atteints.\n\n\
             ✓ Blocage après 5 essais\n○ Tests verts — reste un test"
        );
    }

    #[test]
    fn the_comments_published_read_in_english() {
        use crate::i18n::check::french_in;
        use Lang::En;
        let mut t = Ticket {
            title: "Limit the retries".into(),
            criteria: vec![
                Criterion {
                    text: "Blocked after 5 tries".into(),
                    ok: true,
                    note: String::new(),
                },
                Criterion {
                    text: "Tests pass".into(),
                    ok: false,
                    note: "one test left".into(),
                },
            ],
            progress: vec!["Counter in the database".into(), "Error message".into()],
            ..ticket()
        };
        let mut said = vec![
            column_comment(En, &t, Column::Todo, None),
            column_comment(En, &t, Column::Doing, Some("ticket/esc-12")),
            column_comment(En, &t, Column::Doing, None),
            column_comment(En, &t, Column::Review, None),
        ];
        t.partial = true;
        said.push(column_comment(En, &t, Column::Review, None));
        said.push(column_comment(En, &t, Column::Done, None));
        t.outcome = Some("⇡ Pushed to ticket/esc-12".into());
        t.outcome_url = Some("https://github.com/acme/api/pull/12".into());
        said.push(column_comment(En, &t, Column::Done, None));
        said.push(loop_comment(En, &t));
        assert_eq!(
            said,
            [
                "Escouade: ESC-12 is back in “To do”.",
                "Escouade: ESC-12 is picked up by an agent (branch ticket/esc-12).",
                "Escouade: ESC-12 is picked up by an agent.",
                "Escouade: ESC-12 is ready to review.\n\n\
                 Criteria:\n✓ Blocked after 5 tries\n○ Tests pass — one test left\n\n\
                 What was done:\n- Counter in the database\n- Error message",
                "Escouade: ESC-12 is ready to review (partial goal).\n\n\
                 Criteria:\n✓ Blocked after 5 tries\n○ Tests pass — one test left\n\n\
                 What was done:\n- Counter in the database\n- Error message",
                "Escouade: ESC-12 is done.",
                "Escouade: ESC-12 is done — ⇡ Pushed to ticket/esc-12\nhttps://github.com/acme/api/pull/12",
                "Escouade: ESC-12, loop 2/5 — 1/2 criteria met.\n\n\
                 ✓ Blocked after 5 tries\n○ Tests pass — one test left",
            ]
        );
        for s in &said {
            assert_eq!(french_in(s), None, "{s}");
        }
    }
}
