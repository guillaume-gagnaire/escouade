//! The states of an external ticket that look like the board's columns (the app's `integrations::text` writes the
//! texts of the sync).

use escouade_base::text::slugify;
use escouade_model::model::{Column, ExternalState};
use std::collections::BTreeMap;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn state(id: &str, name: &str) -> ExternalState {
        ExternalState {
            id: id.into(),
            name: name.into(),
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
}
