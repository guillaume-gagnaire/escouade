//! The board's rules, without I/O: ticket keys, the report an agent ends each turn with
//! (```escouade block), what a turn's end does to its ticket, which tickets start, the messages
//! sent to the agents.

// Used by the orchestrator (tickets.rs) as the tasks go; this allowance goes in task 17.
#![allow(dead_code)]

use crate::claude::truncate;
use crate::core::slugify;
use crate::model::*;
use serde_json::Value;

/// A ticket's criteria when none was given.
pub const DEFAULT_CRITERIA: [&str; 2] = ["Implémentation conforme au ticket", "Tests verts"];
/// Sent once when a turn ended without its report.
pub const REMINDER: &str = "Termine par le bilan des critères (bloc escouade).";
const FENCE: &str = "```escouade";

/// How an agent's turn ended, as the core tells the board.
#[derive(Debug, Clone, PartialEq)]
pub enum TurnEnd {
    /// Stopped by the user.
    Interrupted,
    /// Stopped by the usage limit: the automatic resume takes it up.
    Limited,
    Error(String),
    /// The assistant's text of the turn.
    Finished(String),
}

/// The first three letters of the project's name, upper case (accents folded); TIC without any.
pub fn key_prefix(project_name: &str) -> String {
    let letters: String = slugify(project_name)
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .take(3)
        .collect();
    if letters.is_empty() {
        "TIC".into()
    } else {
        letters.to_uppercase()
    }
}

/// The name of a ticket's agent: `atl-42-limiter-les-tentatives`.
pub fn agent_name(key: &str, title: &str) -> String {
    slugify(&format!("{key} {title}"))
}

/// The branch of a ticket: `ticket/atl-42`.
pub fn branch_of(key: &str) -> String {
    format!("ticket/{}", key.to_lowercase())
}

/// One criterion per non-empty line; the default two without any.
pub fn criteria_from(lines: &[String]) -> Vec<Criterion> {
    let mut texts: Vec<String> = lines
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if texts.is_empty() {
        texts = DEFAULT_CRITERIA.iter().map(|s| s.to_string()).collect();
    }
    texts
        .into_iter()
        .map(|text| Criterion {
            text,
            ..Default::default()
        })
        .collect()
}

/// 3, 5 or 8 loops; 5 for anything else.
pub fn max_loops(n: u32) -> u32 {
    if [3, 5, 8].contains(&n) {
        n
    } else {
        5
    }
}

/// What an agent's last ```escouade block said.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// (n from 1, reached, note); None when the block had no list of criteria.
    pub criteria: Option<Vec<(usize, bool, String)>>,
    pub recipe: Option<TestRecipe>,
}

/// The content of the last ```escouade block of `text`, if it is closed.
fn last_block(text: &str) -> Option<&str> {
    let start = text.rfind(FENCE)?;
    let after = &text[start + FENCE.len()..];
    let content = &after[after.find('\n')? + 1..];
    Some(content[..content.find("```")?].trim())
}

/// A folder of the worktree, written relative: no drive, no root, no `..`.
fn safe_dir(d: &str) -> bool {
    let d = d.trim();
    !(d.starts_with(['/', '\\'])
        || d.get(1..2) == Some(":")
        || d.split(['/', '\\']).any(|s| s == ".."))
}

fn recipe_is_safe(r: &TestRecipe) -> bool {
    r.prepare
        .iter()
        .all(|s| safe_dir(&s.dir) && !s.command.trim().is_empty())
        && r.processes
            .iter()
            .all(|p| safe_dir(&p.dir) && !p.command.trim().is_empty())
        && (!r.prepare.is_empty() || !r.processes.is_empty())
}

/// The report of the last ```escouade block of a turn's text: None when it is missing or is not
/// a JSON object. `criteres` (or `criteria`) items without a valid `n` are skipped; a recipe whose
/// folder could leave the worktree is dropped.
pub fn parse_report(text: &str) -> Option<Report> {
    let v: Value = serde_json::from_str(last_block(text)?).ok()?;
    if !v.is_object() {
        return None;
    }
    let criteria = v
        .get("criteres")
        .or_else(|| v.get("criteria"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|c| {
                    let n = c["n"].as_u64().filter(|n| *n >= 1)? as usize;
                    let ok = c["ok"].as_bool().unwrap_or(false);
                    let note = c["note"].as_str().unwrap_or_default().trim().to_string();
                    Some((n, ok, note))
                })
                .collect()
        });
    let recipe = v
        .get("lancement")
        .and_then(|l| serde_json::from_value::<TestRecipe>(l.clone()).ok())
        .filter(recipe_is_safe);
    Some(Report { criteria, recipe })
}

/// What the end of a turn asks of the orchestrator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Next {
    /// The message to send the agent.
    pub send: Option<String>,
    /// The ticket reached "À tester" (to notify).
    pub ready: bool,
    /// The ticket got blocked (to notify).
    pub blocked: bool,
}

fn first_line(e: &str) -> String {
    let line = e
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("erreur inconnue");
    truncate(line, 200)
}

/// What the end of its agent's turn does to a ticket "En cours" (nothing to any other).
pub fn turn_end(t: &mut Ticket, end: &TurnEnd, report: Option<&Report>, now: i64) -> Next {
    let mut next = Next::default();
    if t.column != Column::Doing {
        return next;
    }
    t.blocked = None;
    t.conflict = false;
    match end {
        TurnEnd::Interrupted => {
            t.blocked = Some("Interrompu".into());
            next.blocked = true;
        }
        TurnEnd::Limited => {}
        TurnEnd::Error(e) => {
            t.blocked = Some(format!("Erreur : {}", first_line(e)));
            next.blocked = true;
        }
        TurnEnd::Finished(_) => match report.and_then(|r| r.criteria.as_ref()) {
            None if !t.reminded => {
                t.reminded = true;
                next.send = Some(REMINDER.into());
            }
            None => {
                t.reminded = false;
                t.blocked = Some("Bilan des critères manquant".into());
                next.blocked = true;
            }
            Some(reported) => {
                t.reminded = false;
                for (i, c) in t.criteria.iter_mut().enumerate() {
                    match reported.iter().rev().find(|(n, _, _)| *n == i + 1) {
                        Some((_, ok, note)) => {
                            c.ok = *ok;
                            c.note = note.clone();
                        }
                        None => {
                            c.ok = false;
                            c.note.clear();
                        }
                    }
                }
                let all = t.criteria.iter().all(|c| c.ok);
                if all || t.iteration >= t.max_loops {
                    t.column = Column::Review;
                    t.partial = !all;
                    t.review_at = Some(now);
                    next.ready = true;
                } else {
                    t.iteration += 1;
                    next.send = Some(loop_message(t));
                }
            }
        },
    }
    next
}

/// The tickets of `project_id` to start now, in order: "À faire" by rank (with the autopilot, or
/// launched by hand), while tickets "En cours" and not blocked leave places; none while an agent
/// waits for its quota.
pub fn to_start(
    tickets: &[Ticket],
    project_id: &str,
    s: &BoardSettings,
    quota_paused: bool,
) -> Vec<String> {
    if quota_paused {
        return Vec::new();
    }
    let mine = || tickets.iter().filter(|t| t.project_id == project_id);
    let busy = mine()
        .filter(|t| t.column == Column::Doing && t.blocked.is_none())
        .count();
    let free = (s.max_parallel.clamp(1, 6) as usize).saturating_sub(busy);
    let mut todo: Vec<&Ticket> = mine()
        .filter(|t| t.column == Column::Todo && (s.autopilot || t.forced))
        .collect();
    todo.sort_by_key(|t| (t.rank, t.created_at));
    todo.into_iter().take(free).map(|t| t.id.clone()).collect()
}

/// Back to "À faire" from scratch: its agent was archived or deleted by hand.
pub fn back_to_todo(t: &mut Ticket) {
    t.column = Column::Todo;
    t.agent_id = None;
    t.iteration = 0;
    t.partial = false;
    t.blocked = None;
    t.conflict = false;
    t.step = None;
    t.reminded = false;
    t.forced = false;
    t.started_at = None;
    t.review_at = None;
    for c in &mut t.criteria {
        c.ok = false;
        c.note.clear();
    }
}

fn numbered(t: &Ticket) -> String {
    t.criteria
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {}", i + 1, c.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first message of a ticket's agent.
pub fn first_message(t: &Ticket) -> String {
    let mut s = format!("Ticket {} : {}\n", t.key, t.title);
    if !t.description.trim().is_empty() {
        s.push_str(&format!("\n{}\n", t.description.trim()));
    }
    s.push_str(&format!(
        "\nCritères d'acceptation :\n{}\n\nBoucle 1/{}. Travaille jusqu'à atteindre tous les critères, puis termine par le bilan.",
        numbered(t),
        t.max_loops
    ));
    s
}

/// The message of the next loop: the criteria still missing, with the agent's notes.
pub fn loop_message(t: &Ticket) -> String {
    let missing: Vec<String> = t
        .criteria
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.ok)
        .map(|(i, c)| {
            if c.note.is_empty() {
                (i + 1).to_string()
            } else {
                format!("{} ({})", i + 1, c.note)
            }
        })
        .collect();
    format!(
        "Boucle {}/{}. Critères non atteints : {}. Continue jusqu'à les atteindre, puis termine par le bilan.",
        t.iteration,
        t.max_loops,
        missing.join(", ")
    )
}

pub fn resume_message(key: &str) -> String {
    format!("Reprends le ticket {key} là où tu en étais, puis termine par le bilan.")
}

pub fn restart_message(key: &str) -> String {
    format!("L'app a redémarré pendant ton travail sur {key} : reprends là où tu en étais, puis termine par le bilan.")
}

pub fn reject_message(key: &str, comment: &str) -> String {
    let comment = comment.trim().trim_end_matches('.');
    format!("Retour de test sur {key} : {comment}. Corrige, revérifie tous les critères, puis termine par le bilan.")
}

pub fn tests_failed_message(command: &str, tail: &str) -> String {
    format!("Les tests (`{command}`) échouent :\n\n```\n{tail}\n```\n\nCorrige, revérifie les critères, puis termine par le bilan.")
}

pub fn conflict_message(target: &str, files: &[String]) -> String {
    if files.is_empty() {
        return format!("J'ai mergé {target} dans ta branche, sans conflit. Revérifie les critères, puis termine par le bilan.");
    }
    format!(
        "Le merge de {target} dans ta branche a des conflits sur : {}. Résous-les, commite le merge, revérifie les critères, puis termine par le bilan.",
        files.join(", ")
    )
}

pub fn rebase_message(target: &str) -> String {
    format!("Rebase ta branche sur {target} et résous ses conflits, puis revérifie les critères et termine par le bilan.")
}

/// Text on one line (a command-line argument), cut to `max` characters.
fn one_line(s: &str, max: usize) -> String {
    truncate(&s.split_whitespace().collect::<Vec<_>>().join(" "), max)
}

/// What the protocol may weigh as a command-line argument, once escaped (see `escaped_len`).
/// cmd.exe takes 8191 characters for the whole command line; the rest goes to the other arguments.
const PROTOCOL_BUDGET: usize = 6000;
/// Ends the criteria when some did not fit.
const MORE: &str = " ; …";

/// What `s` weighs as an argument of a `.cmd`: Rust escapes it for cmd.exe, where `%` becomes a
/// longer sequence (8 characters) and `"` is doubled. Other characters count their UTF-8 bytes,
/// which is never less than their UTF-16 units.
fn escaped_len(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '%' => 8,
            '"' => 2,
            c => c.len_utf8(),
        })
        .sum()
}

/// The protocol around `criteria`, already joined.
fn protocol(t: &Ticket, criteria: &str, ports: Option<u16>) -> String {
    let mut p = format!(
        "Tu travailles en autonomie sur le ticket {key} « {title} » d'Escouade. \
         Critères d'acceptation ({n}) : {criteria}. \
         Vérifie toi-même chaque critère (tests, exécution) avant de le déclarer atteint. \
         S'il te faut une décision, pose la question avec l'outil de question. \
         À la fin de CHAQUE réponse, termine par un bloc de code ouvert par ```escouade et fermé par ``` \
         qui contient un JSON {{\"criteres\": [{{\"n\": 1, \"ok\": true, \"note\": \"vérifié par …\"}}, \
         {{\"n\": 2, \"ok\": false, \"note\": \"ce qui manque\"}}]}} avec un élément par critère (n à partir de 1).",
        key = t.key,
        title = one_line(&t.title, 200),
        n = t.criteria.len(),
    );
    if let Some(base) = ports {
        let (end, web) = (base.saturating_add(9), base.saturating_add(1));
        p.push_str(&format!(
            " Ports réservés à ce worktree : {base} à {end} (ESCOUADE_PORT_BASE et ESCOUADE_PORT_END dans ses lancements de test) ; \
             ne les écris pas dans des fichiers versionnés, passe-les en arguments ou en variables d'environnement. \
             Au plus tard quand tous les critères sont atteints, ajoute au JSON une clé \"lancement\" qui dit comment lancer \
             ce worktree pour le tester, par exemple \"lancement\": {{\"preparation\": [{{\"commande\": \"npm install\", \"dossier\": \"web\"}}], \
             \"processus\": [{{\"nom\": \"web\", \"commande\": \"npm run dev -- --port {web}\", \"dossier\": \"web\", \
             \"env\": {{\"PORT\": \"{web}\"}}, \"url\": \"http://localhost:{web}\"}}], \"ouvrir\": \"http://localhost:{web}/page-de-la-fonctionnalite\"}} : \
             « dossier » est relatif au worktree, « url » répond quand le processus est prêt, « ouvrir » est l'adresse qui montre \
             directement la fonctionnalité développée (la page, l'écran, l'état précis à tester)."
        ));
    }
    p
}

/// The ticket's protocol, appended to Claude Code's system prompt (`--append-system-prompt`), so
/// that it outlives compaction and resumes. On one line: it is a command-line argument, which a
/// `.cmd` (npm's claude.cmd) cannot take with line breaks. Once escaped for cmd.exe it weighs at
/// most `PROTOCOL_BUDGET`: the criteria that do not fit are replaced by a "…".
pub fn protocol_prompt(t: &Ticket, ports: Option<u16>) -> String {
    let room = PROTOCOL_BUDGET.saturating_sub(escaped_len(&protocol(t, "", ports)));
    let mut criteria = String::new();
    let mut used = 0;
    for (i, c) in t.criteria.iter().enumerate() {
        let item = format!(
            "{}{}) {}",
            if i > 0 { " ; " } else { "" },
            i + 1,
            one_line(&c.text, 200)
        );
        let weight = escaped_len(&item);
        // Room is kept for the "…" unless this is the last criterion.
        let more = if i + 1 < t.criteria.len() {
            escaped_len(MORE)
        } else {
            0
        };
        if used + weight + more > room {
            criteria.push_str(MORE);
            break;
        }
        used += weight;
        criteria.push_str(&item);
    }
    protocol(t, &criteria, ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(n: usize, max: u32) -> Ticket {
        Ticket {
            id: "t1".into(),
            project_id: "p1".into(),
            key: "ATL-42".into(),
            title: "Limiter les tentatives".into(),
            criteria: (1..=n)
                .map(|i| Criterion {
                    text: format!("critère {i}"),
                    ..Default::default()
                })
                .collect(),
            max_loops: max,
            column: Column::Doing,
            iteration: 1,
            ..Default::default()
        }
    }

    fn block(json: &str) -> String {
        format!("Voilà.\n\n```escouade\n{json}\n```\n")
    }

    fn finished(json: &str) -> (TurnEnd, Option<Report>) {
        let text = block(json);
        (TurnEnd::Finished(text.clone()), parse_report(&text))
    }

    #[test]
    fn keys_names_and_branches_come_from_the_project_and_the_title() {
        assert_eq!(key_prefix("atlas-api"), "ATL");
        assert_eq!(key_prefix("Écoute"), "ECO");
        assert_eq!(key_prefix("ab"), "AB");
        assert_eq!(key_prefix("42"), "TIC");
        assert_eq!(
            agent_name("ATL-42", "Limiter les tentatives"),
            "atl-42-limiter-les-tentatives"
        );
        assert_eq!(branch_of("ATL-42"), "ticket/atl-42");
        let texts = |c: Vec<Criterion>| c.into_iter().map(|c| c.text).collect::<Vec<_>>();
        assert_eq!(
            texts(criteria_from(&["  a ".into(), "".into(), "b".into()])),
            ["a", "b"]
        );
        assert_eq!(texts(criteria_from(&[])), DEFAULT_CRITERIA);
        assert_eq!(
            (max_loops(3), max_loops(8), max_loops(0), max_loops(7)),
            (3, 8, 5, 5)
        );
    }

    #[test]
    fn the_last_escouade_block_is_read() {
        let text = format!(
            "{}\nPuis :\n{}",
            block(r#"{"criteres": [{"n": 1, "ok": false}]}"#),
            block(
                r#"{"criteres": [{"n": 1, "ok": true, "note": "vu"}, {"n": 2, "ok": false, "note": "reste"}]}"#
            )
        );
        let r = parse_report(&text).unwrap();
        assert_eq!(
            r.criteria,
            Some(vec![
                (1, true, "vu".to_string()),
                (2, false, "reste".to_string())
            ])
        );
        // `criteria` is understood too.
        let r = parse_report(&block(r#"{"criteria": [{"n": 2, "ok": true}]}"#)).unwrap();
        assert_eq!(r.criteria, Some(vec![(2, true, String::new())]));
    }

    #[test]
    fn a_missing_or_unreadable_block_is_no_report() {
        assert_eq!(parse_report("Fini, tout marche."), None);
        assert_eq!(parse_report(&block("{criteres: oui}")), None);
        assert_eq!(parse_report("```escouade\n{\"criteres\": []}"), None);
        // A readable block without criteria (a launch recipe alone): a report without them.
        assert_eq!(parse_report(&block("{}")).unwrap().criteria, None);
    }

    #[test]
    fn a_recipe_is_kept_unless_a_folder_leaves_the_worktree() {
        let r = parse_report(&block(
            r#"{"criteres": [], "lancement": {"preparation": [{"commande": "npm install", "dossier": "web"}], "processus": [{"nom": "web", "commande": "npm run dev", "dossier": "web", "env": {"PORT": 4111}, "url": "http://localhost:4111"}], "ouvrir": "http://localhost:4111/connexion"}}"#,
        ))
        .unwrap();
        let recipe = r.recipe.unwrap();
        assert_eq!(recipe.processes[0].env["PORT"], "4111");
        assert_eq!(recipe.open, "http://localhost:4111/connexion");
        for bad in ["../x", "web/../../x", "/etc", "C:\\x", "..\\x"] {
            let json = format!(
                r#"{{"lancement": {{"processus": [{{"nom": "x", "commande": "y", "dossier": {}}}]}}}}"#,
                serde_json::to_string(bad).unwrap()
            );
            assert_eq!(parse_report(&block(&json)).unwrap().recipe, None, "{bad}");
        }
    }

    #[test]
    fn a_null_env_in_the_recipe_does_not_cost_the_recipe_or_the_report() {
        let r = parse_report(&block(
            r#"{"criteres": [{"n": 1, "ok": true}], "lancement": {"processus": [{"nom": "web", "commande": "npm run dev", "env": null}, {"nom": "api", "commande": "npm start", "env": {"PORT": 4112, "DEBUG": null}}]}}"#,
        ))
        .unwrap();
        assert_eq!(r.criteria, Some(vec![(1, true, String::new())]));
        let recipe = r.recipe.unwrap();
        assert!(recipe.processes[0].env.is_empty());
        assert_eq!(recipe.processes[1].env.keys().collect::<Vec<_>>(), ["PORT"]);
    }

    #[test]
    fn all_criteria_met_send_the_ticket_to_review() {
        let mut t = ticket(2, 5);
        let (end, r) = finished(
            r#"{"criteres": [{"n": 1, "ok": true}, {"n": 2, "ok": true, "note": "tests verts"}]}"#,
        );
        let next = turn_end(&mut t, &end, r.as_ref(), 100);
        assert_eq!(
            (t.column, t.partial, t.review_at),
            (Column::Review, false, Some(100))
        );
        assert_eq!(t.criteria[1].note, "tests verts");
        assert_eq!(
            next,
            Next {
                ready: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn criteria_still_missing_start_the_next_loop_until_the_last_one() {
        let mut t = ticket(4, 3);
        let (end, r) = finished(
            r#"{"criteres": [{"n": 1, "ok": true}, {"n": 2, "ok": false, "note": "reste la rotation"}, {"n": 3, "ok": true}]}"#,
        );
        let next = turn_end(&mut t, &end, r.as_ref(), 1);
        assert_eq!(t.iteration, 2);
        // A criterion left out counts as missing.
        assert_eq!(
            next.send.as_deref(),
            Some("Boucle 2/3. Critères non atteints : 2 (reste la rotation), 4. Continue jusqu'à les atteindre, puis termine par le bilan.")
        );
        turn_end(&mut t, &end, r.as_ref(), 2);
        assert_eq!(t.iteration, 3);
        let next = turn_end(&mut t, &end, r.as_ref(), 3);
        assert_eq!((t.column, t.partial), (Column::Review, true));
        assert!(next.ready && next.send.is_none());
    }

    #[test]
    fn a_missing_report_is_asked_for_once_then_blocks() {
        let mut t = ticket(1, 5);
        let end = TurnEnd::Finished("Fini.".into());
        let next = turn_end(&mut t, &end, None, 1);
        assert_eq!(next.send.as_deref(), Some(REMINDER));
        assert_eq!((t.iteration, t.blocked.as_deref()), (1, None));
        let next = turn_end(&mut t, &end, None, 2);
        assert_eq!(t.blocked.as_deref(), Some("Bilan des critères manquant"));
        assert!(next.blocked && next.send.is_none());
        // A report with no criteria (a launch recipe alone) is no report either.
        let mut t = ticket(1, 5);
        let (end, r) = finished("{}");
        assert_eq!(
            turn_end(&mut t, &end, r.as_ref(), 1).send.as_deref(),
            Some(REMINDER)
        );
    }

    #[test]
    fn a_stop_or_an_error_blocks_and_the_usage_limit_waits() {
        let mut t = ticket(1, 5);
        assert!(turn_end(&mut t, &TurnEnd::Interrupted, None, 1).blocked);
        assert_eq!(t.blocked.as_deref(), Some("Interrompu"));
        let mut t = ticket(1, 5);
        turn_end(
            &mut t,
            &TurnEnd::Error("\nAPI Error: 500\nretry".into()),
            None,
            1,
        );
        assert_eq!(t.blocked.as_deref(), Some("Erreur : API Error: 500"));
        let mut t = ticket(1, 5);
        turn_end(&mut t, &TurnEnd::Error(String::new()), None, 1);
        assert_eq!(t.blocked.as_deref(), Some("Erreur : erreur inconnue"));
        let mut t = ticket(1, 5);
        assert_eq!(
            turn_end(&mut t, &TurnEnd::Limited, None, 1),
            Next::default()
        );
        assert_eq!((t.column, t.blocked.clone()), (Column::Doing, None));
    }

    #[test]
    fn a_turn_unblocks_its_ticket_but_one_no_longer_in_progress_changes_nothing() {
        let mut t = ticket(1, 5);
        t.blocked = Some("Interrompu".into());
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": true}]}"#);
        turn_end(&mut t, &end, r.as_ref(), 1);
        assert_eq!((t.blocked.clone(), t.column), (None, Column::Review));
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": false}]}"#);
        assert_eq!(turn_end(&mut t, &end, r.as_ref(), 2), Next::default());
        assert_eq!(t.column, Column::Review);
    }

    fn todo(id: &str, rank: i64) -> Ticket {
        Ticket {
            id: id.into(),
            project_id: "p1".into(),
            column: Column::Todo,
            rank,
            ..Default::default()
        }
    }

    #[test]
    fn tickets_start_by_rank_while_places_are_free() {
        let s = BoardSettings::default();
        let mut list = vec![
            todo("c", 3),
            todo("a", 1),
            todo("b", 2),
            Ticket {
                project_id: "p2".into(),
                ..todo("other", 0)
            },
        ];
        assert_eq!(to_start(&list, "p1", &s, false), ["a", "b"]);
        list.push(Ticket {
            column: Column::Doing,
            ..todo("busy", 0)
        });
        assert_eq!(to_start(&list, "p1", &s, false), ["a"]);
        // A blocked ticket gives its place up; one to test holds none.
        list.push(Ticket {
            column: Column::Doing,
            blocked: Some("Interrompu".into()),
            ..todo("blocked", 0)
        });
        list.push(Ticket {
            column: Column::Review,
            ..todo("review", 0)
        });
        assert_eq!(to_start(&list, "p1", &s, false), ["a"]);
        list.push(Ticket {
            column: Column::Doing,
            ..todo("busy2", 0)
        });
        assert!(to_start(&list, "p1", &s, false).is_empty());
    }

    #[test]
    fn without_the_autopilot_only_launched_tickets_start_and_nothing_starts_on_a_quota() {
        let off = BoardSettings {
            autopilot: false,
            max_parallel: 6,
            ..Default::default()
        };
        let list = vec![
            todo("a", 1),
            Ticket {
                forced: true,
                ..todo("b", 2)
            },
        ];
        assert_eq!(to_start(&list, "p1", &off, false), ["b"]);
        assert!(to_start(&list, "p1", &BoardSettings::default(), true).is_empty());
        // A count out of bounds (an old file) still lets one run.
        let zero = BoardSettings {
            max_parallel: 0,
            ..Default::default()
        };
        assert_eq!(to_start(&list, "p1", &zero, false), ["a"]);
    }

    #[test]
    fn a_ticket_sent_back_to_todo_starts_again_from_scratch() {
        let mut t = ticket(2, 5);
        t.agent_id = Some("a1".into());
        t.iteration = 3;
        t.partial = true;
        t.blocked = Some("x".into());
        t.step = Some("Tests…".into());
        t.criteria[0].ok = true;
        t.criteria[0].note = "vu".into();
        back_to_todo(&mut t);
        assert_eq!(
            (
                t.column,
                t.agent_id.clone(),
                t.iteration,
                t.partial,
                t.blocked.clone(),
                t.step.clone()
            ),
            (Column::Todo, None, 0, false, None, None)
        );
        assert!(t.criteria.iter().all(|c| !c.ok && c.note.is_empty()));
    }

    #[test]
    fn messages_name_the_ticket_and_the_loop() {
        let mut t = ticket(2, 5);
        t.description = "Contexte : l'API publique.".into();
        assert_eq!(
            first_message(&t),
            "Ticket ATL-42 : Limiter les tentatives\n\nContexte : l'API publique.\n\nCritères d'acceptation :\n1. critère 1\n2. critère 2\n\nBoucle 1/5. Travaille jusqu'à atteindre tous les critères, puis termine par le bilan."
        );
        assert_eq!(
            resume_message("ATL-42"),
            "Reprends le ticket ATL-42 là où tu en étais, puis termine par le bilan."
        );
        assert_eq!(
            restart_message("ATL-42"),
            "L'app a redémarré pendant ton travail sur ATL-42 : reprends là où tu en étais, puis termine par le bilan."
        );
        assert_eq!(
            reject_message("ATL-42", " le bouton est mal placé. "),
            "Retour de test sur ATL-42 : le bouton est mal placé. Corrige, revérifie tous les critères, puis termine par le bilan."
        );
        assert_eq!(
            tests_failed_message("npm test", "1 failed"),
            "Les tests (`npm test`) échouent :\n\n```\n1 failed\n```\n\nCorrige, revérifie les critères, puis termine par le bilan."
        );
        assert_eq!(
            conflict_message("main", &["src/a.ts".into(), "src/b.ts".into()]),
            "Le merge de main dans ta branche a des conflits sur : src/a.ts, src/b.ts. Résous-les, commite le merge, revérifie les critères, puis termine par le bilan."
        );
        assert_eq!(
            conflict_message("main", &[]),
            "J'ai mergé main dans ta branche, sans conflit. Revérifie les critères, puis termine par le bilan."
        );
        assert_eq!(
            rebase_message("main"),
            "Rebase ta branche sur main et résous ses conflits, puis revérifie les critères et termine par le bilan."
        );
    }

    #[test]
    fn the_protocol_holds_on_one_line_with_the_criteria_and_the_ports() {
        let mut t = ticket(2, 5);
        t.title = "Gérer\nles « % & \" »".into();
        let p = protocol_prompt(&t, Some(4120));
        assert!(!p.contains('\n') && !p.contains('\r'), "{p}");
        assert!(
            p.contains("le ticket ATL-42 « Gérer les « % & \" » »"),
            "{p}"
        );
        assert!(
            p.contains("Critères d'acceptation (2) : 1) critère 1 ; 2) critère 2."),
            "{p}"
        );
        assert!(p.contains("```escouade"));
        assert!(p.contains("4120 à 4129") && p.contains("http://localhost:4121"));
        assert!(!protocol_prompt(&t, None).contains("Ports réservés"));
        // Hundreds of long criteria stay well under what a command line takes.
        t.criteria = (0..300)
            .map(|i| Criterion {
                text: format!("{i} {}", "x".repeat(300)),
                ..Default::default()
            })
            .collect();
        assert!(protocol_prompt(&t, Some(4100)).len() < 7000);
    }

    /// What a `.cmd` argument weighs once Rust has escaped it for cmd.exe: `%` becomes a longer
    /// sequence (8 characters) and `"` is doubled; anything else counts its UTF-8 bytes (at least
    /// its UTF-16 units).
    fn escaped_weight(s: &str) -> usize {
        s.chars()
            .map(|c| match c {
                '%' => 8,
                '"' => 2,
                c => c.len_utf8(),
            })
            .sum()
    }

    #[test]
    fn the_protocol_stays_under_cmds_command_line_once_escaped() {
        // The worst case: a title and criteria made of what a `.cmd` argument escapes the most,
        // long enough to be cut.
        let nasty = "%\"".repeat(150);
        let mut t = ticket(0, 5);
        t.title = nasty.clone();
        t.criteria = (0..50)
            .map(|_| Criterion {
                text: nasty.clone(),
                ..Default::default()
            })
            .collect();
        for ports in [None, Some(4100)] {
            let p = protocol_prompt(&t, ports);
            assert!(!p.contains('\n') && !p.contains('\r'));
            // cmd.exe takes 8191 characters for the whole command line; the protocol keeps to
            // 6000 of them and leaves the rest to the other arguments.
            let weight = escaped_weight(&p);
            assert!(weight <= 6000, "{ports:?}: {weight}");
            // The criteria are cut, not dropped: the first is there, the count is the real one.
            assert!(p.contains("Critères d'acceptation (50) : 1) "), "{ports:?}");
            assert!(p.contains(" ; …"), "{ports:?}");
        }
        // Long accented text weighs its bytes, never less.
        t.title = "é".repeat(300);
        t.criteria = (0..300)
            .map(|_| Criterion {
                text: "é".repeat(300),
                ..Default::default()
            })
            .collect();
        assert!(escaped_weight(&protocol_prompt(&t, Some(4100))) <= 6000);
        // A short list is kept whole.
        let mut t = ticket(30, 5);
        t.title = "Court".into();
        let p = protocol_prompt(&t, Some(4100));
        assert!(p.contains("30) critère 30."), "{p}");
    }
}
