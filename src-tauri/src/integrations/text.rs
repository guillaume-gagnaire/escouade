//! The texts of external tickets: the acceptance criteria a description lists, the states that look like the board's
//! columns, and the comments the sync writes.

use crate::core::slugify;
use crate::i18n::Lang;
use crate::model::{Column, ExternalState, Ticket};
use std::collections::BTreeMap;

// The criteria of a description, as every source reads them, in the core and in the plugins (`escouade_plugin_kit::text`).
#[cfg(test)]
use escouade_plugin_kit::text::criteria_of;
#[cfg(test)]
pub use escouade_plugin_kit::text::{criteria_in, names_criteria};

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
        // Its pull request opened (ESC-59): where it is reviewed.
        Column::Pr => match &t.pr {
            Some(pr) => {
                let (word, n, url) = (&pr.word, pr.number, &pr.url);
                tr_in!(
                    lang,
                    "Escouade : {key} est en revue — {word} #{n}
{url}",
                    "Escouade: {key} is in review — {word} #{n}
{url}"
                )
            }
            None => tr_in!(
                lang,
                "Escouade : {key} est en revue.",
                "Escouade: {key} is in review."
            ),
        },
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
    fn a_field_of_criteria_gives_its_items_else_its_lines() {
        assert_eq!(
            criteria_in("Acceptance criteria:\n- Plus de crash\n- [ ] Un test e2e\n\nNote"),
            ["Plus de crash", "Un test e2e"]
        );
        assert_eq!(
            criteria_in("Plus de crash\n\nUn test e2e\n"),
            ["Plus de crash", "Un test e2e"]
        );
        assert!(criteria_in("").is_empty());
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
