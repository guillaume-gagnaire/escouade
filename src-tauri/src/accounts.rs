//! The Claude accounts: each one is Claude Code run with a configuration folder of its own
//! (`CLAUDE_CONFIG_DIR`), so with its own sign-in, sessions and quota. Principal is the user's own
//! Claude Code, launched as it always was: never given a `CLAUDE_CONFIG_DIR`, since setting one,
//! even to `~/.claude`, changes the name of its macOS keychain entry and signs the user out.
//!
//! The order of `Settings::accounts` is their priority: new agents go to the first one usable.

use crate::board;
use crate::claude;
use crate::model::{Account, AccountUsage, Settings};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The id of the user's own account.
pub const PRINCIPAL: &str = "principal";

/// The variable that gives Claude Code its configuration folder.
const CONFIG_DIR_VAR: &str = "CLAUDE_CONFIG_DIR";

/// Principal as it is made the first time: the user's own Claude Code, active.
pub fn principal() -> Account {
    Account {
        id: PRINCIPAL.into(),
        name: tr!("Principal", "Main"),
        config_dir: String::new(),
        claude_path: String::new(),
        active: true,
    }
}

/// Puts the accounts right: Principal among them (first when it was missing, as on the first
/// load), with no folder of its own; every id unique; at least one account active.
pub fn normalize(settings: &mut Settings) {
    let accounts = &mut settings.accounts;
    for a in accounts.iter_mut() {
        a.id = a.id.trim().to_string();
        // Exactly the folder its processes are given (`launch_env`).
        a.config_dir = a.config_dir.trim().to_string();
    }
    // An id given twice: the first keeps it, the others are told apart (their agents, if any,
    // stay with the first). Every id given counts as taken from the start: one given once keeps
    // it even when another is told apart before it comes.
    let mut taken: HashSet<String> = accounts
        .iter()
        .filter(|a| !a.id.is_empty())
        .map(|a| a.id.clone())
        .collect();
    let mut seen = HashSet::new();
    for a in accounts.iter_mut().filter(|a| !a.id.is_empty()) {
        if !seen.insert(a.id.clone()) {
            a.id = unique(&a.id, &taken);
            taken.insert(a.id.clone());
            seen.insert(a.id.clone());
        }
    }
    // None (written by hand): one after its name, never Principal's.
    taken.insert(PRINCIPAL.to_string());
    for a in accounts.iter_mut().filter(|a| a.id.is_empty()) {
        let base = match crate::core::slugify(&a.name) {
            s if s.is_empty() => "account".to_string(),
            s => s,
        };
        a.id = unique(&base, &taken);
        taken.insert(a.id.clone());
    }
    let at = match accounts.iter().position(|a| a.id == PRINCIPAL) {
        Some(at) => at,
        None => {
            accounts.insert(0, principal());
            0
        }
    };
    let main = &mut accounts[at];
    // Whatever a hand-edited file says: a folder would sign the user out of it.
    main.config_dir.clear();
    if main.name.trim().is_empty() {
        main.name = principal().name;
    }
    if !accounts.iter().any(|a| a.active) {
        accounts[at].active = true;
    }
}

/// `base`, else `base-2`, `base-3`… the first that is not `taken`.
fn unique(base: &str, taken: &HashSet<String>) -> String {
    let mut id = base.to_string();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// The account `id` names, else Principal (an id unknown to the settings: an account removed, a
/// state saved by a later version), with a line in the log. None only when the settings have no
/// Principal, which `normalize` prevents.
pub fn find<'a>(settings: &'a Settings, id: &str) -> Option<&'a Account> {
    if let Some(a) = settings.accounts.iter().find(|a| a.id == id) {
        return Some(a);
    }
    // Empty: an agent made by code that names none.
    if !id.is_empty() && id != PRINCIPAL {
        log::warn!("Claude account {id:?} unknown: Principal instead");
    }
    settings.accounts.iter().find(|a| a.id == PRINCIPAL)
}

/// `find`, as a copy that is always there: settings never normalized (written whole by a test)
/// give Principal as it is made.
pub fn get(settings: &Settings, id: &str) -> Account {
    find(settings, id).cloned().unwrap_or_else(principal)
}

/// The account new agents would go to, as the status bar shows it: the first active one, in the
/// settings' order, none of whose quota windows (`usage`) is used `threshold` percent or more at
/// `now` (as the autopilot counts it), else the first active one.
pub fn current(settings: &Settings, usage: &[AccountUsage], threshold: u32, now: i64) -> String {
    let over = |id: &str| {
        usage.iter().find(|u| u.id == id).is_some_and(|u| {
            [u.five_hour, u.seven_day]
                .iter()
                .flatten()
                .any(|w| board::over_threshold(w, threshold, now))
        })
    };
    let mut active = settings.accounts.iter().filter(|a| a.active);
    let first = active.clone().next();
    active
        .find(|a| !over(&a.id))
        .or(first)
        .map_or_else(|| PRINCIPAL.to_string(), |a| a.id.clone())
}

/// The configuration folder the account's Claude Code reads: its own, or for Principal (no folder)
/// the app's `CLAUDE_CONFIG_DIR` when it has one, which its processes inherit, else `~/.claude`.
// Allowed unused until the accounts' settings read each one's folder (then drop the allow): the
// quota goes by `usage::sign_in`, which takes the app's environment as `config_dir_with` does.
#[allow(dead_code)]
pub fn config_dir(account: &Account) -> PathBuf {
    let env = std::env::var(CONFIG_DIR_VAR).ok();
    let home = dirs::home_dir().unwrap_or_default();
    config_dir_with(account, env.as_deref(), &home)
}

/// `config_dir` with the app's `CLAUDE_CONFIG_DIR` (`env`) and home folder given.
pub fn config_dir_with(account: &Account, env: Option<&str>, home: &Path) -> PathBuf {
    match own_dir(account) {
        Some(dir) => PathBuf::from(dir),
        None => match env.filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => home.join(".claude"),
        },
    }
}

/// The account's own folder, the `CLAUDE_CONFIG_DIR` its processes get: none for Principal,
/// whatever its settings say.
pub(crate) fn own_dir(account: &Account) -> Option<&str> {
    let dir = account.config_dir.trim();
    (account.id != PRINCIPAL && !dir.is_empty()).then_some(dir)
}

/// What the account's Claude Code is launched with besides the network settings
/// (`Settings::claude_env`): its `CLAUDE_CONFIG_DIR`, exactly the folder saved, when it has one.
/// Never for Principal.
pub fn launch_env(account: &Account) -> Vec<(String, String)> {
    own_dir(account)
        .map(|dir| (CONFIG_DIR_VAR.to_string(), dir.to_string()))
        .into_iter()
        .collect()
}

/// The `claude` the account runs: its own when it names one, else the settings' (found as
/// `claude::resolve_binary` finds it). None when it cannot be found.
pub fn program(account: &Account, settings: &Settings) -> Option<PathBuf> {
    let own = account.claude_path.trim();
    claude::resolve_binary(if own.is_empty() {
        &settings.claude_path
    } else {
        own
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    fn account(id: &str, dir: &str) -> Account {
        Account {
            id: id.into(),
            name: id.to_uppercase(),
            config_dir: dir.into(),
            claude_path: String::new(),
            active: true,
        }
    }

    fn ids(s: &Settings) -> Vec<&str> {
        s.accounts.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn settings_without_accounts_get_principal_active_with_no_folder() {
        let mut s = Settings::default();
        normalize(&mut s);
        assert_eq!(
            s.accounts,
            vec![Account {
                id: "principal".into(),
                name: "Principal".into(),
                config_dir: String::new(),
                claude_path: String::new(),
                active: true,
            }]
        );
        assert_eq!(principal(), s.accounts[0]);
        // Normalized again, nothing changes.
        let before = s.accounts.clone();
        normalize(&mut s);
        assert_eq!(s.accounts, before);
    }

    #[test]
    fn a_list_without_principal_gets_it_first_and_keeps_the_others_as_they_are() {
        let pro = account("pro", r"C:\Users\x\.escouade\claude\pro");
        let mut s = Settings {
            accounts: vec![pro.clone()],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(ids(&s), ["principal", "pro"]);
        assert_eq!(s.accounts[1], pro);
    }

    #[test]
    fn principal_keeps_its_place_and_never_has_a_folder_of_its_own() {
        let mut s = Settings {
            accounts: vec![
                account("pro", "/Users/x/.escouade/claude/pro"),
                Account {
                    name: "  ".into(),
                    ..account("principal", "/Users/x/.claude")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        // The order is the user's priority.
        assert_eq!(ids(&s), ["pro", "principal"]);
        assert_eq!(s.accounts[1].config_dir, "");
        assert_eq!(s.accounts[1].name, "Principal");
        // A name of the user's own stays.
        s.accounts[1].name = "Perso".into();
        normalize(&mut s);
        assert_eq!(s.accounts[1].name, "Perso");
    }

    #[test]
    fn ids_given_twice_or_none_are_made_unique() {
        let mut s = Settings {
            accounts: vec![
                account("principal", ""),
                account("pro", "/a"),
                account("pro", "/b"),
                account("principal", "/c"),
                Account {
                    name: "Équipe".into(),
                    ..account("", "/d")
                },
                Account {
                    name: "Principal".into(),
                    ..account(" ", "/e")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(
            ids(&s),
            [
                "principal",
                "pro",
                "pro-2",
                "principal-2",
                "equipe",
                "principal-3"
            ]
        );
        // The first one keeps the id; the others keep their folder.
        let dirs: Vec<&str> = s.accounts.iter().map(|a| a.config_dir.as_str()).collect();
        assert_eq!(dirs, ["", "/a", "/b", "/c", "/d", "/e"]);
    }

    #[test]
    fn an_id_given_once_keeps_it_when_another_is_told_apart() {
        // `x-2` is already some account's: the second `x` takes the next one free, and `x-2`
        // stays as it is (its agents with it).
        let mut s = Settings {
            accounts: vec![account("x", "/a"), account("x", "/b"), account("x-2", "/c")],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(ids(&s), ["principal", "x", "x-3", "x-2"]);
        let dirs: Vec<&str> = s.accounts.iter().map(|a| a.config_dir.as_str()).collect();
        assert_eq!(dirs, ["", "/a", "/b", "/c"]);
    }

    #[test]
    fn principal_is_made_active_when_no_account_is() {
        let mut s = Settings {
            accounts: vec![
                Account {
                    active: false,
                    ..account("pro", "/a")
                },
                Account {
                    active: false,
                    ..account("principal", "")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        let active: Vec<bool> = s.accounts.iter().map(|a| a.active).collect();
        assert_eq!(active, [false, true]);
        // Another account active: Principal may rest.
        s.accounts[0].active = true;
        s.accounts[1].active = false;
        normalize(&mut s);
        let active: Vec<bool> = s.accounts.iter().map(|a| a.active).collect();
        assert_eq!(active, [true, false]);
    }

    #[test]
    fn an_account_saved_with_only_part_of_its_fields_is_active() {
        let a: Account = serde_json::from_value(serde_json::json!({ "id": "pro" })).unwrap();
        assert_eq!(a, account_named("pro", ""));
        let v = serde_json::to_value(account("pro", "/a")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "id": "pro", "name": "PRO", "configDir": "/a",
                                "claudePath": "", "active": true })
        );
    }

    fn account_named(id: &str, name: &str) -> Account {
        Account {
            name: name.into(),
            ..account(id, "")
        }
    }

    #[test]
    fn principal_is_launched_without_claude_config_dir_another_account_with_its_folder() {
        assert!(launch_env(&principal()).is_empty());
        // Even with a folder written by hand in its settings.
        assert!(launch_env(&account("principal", "/Users/x/.claude")).is_empty());
        let dir = r"C:\Users\x\.escouade\claude\pro";
        assert_eq!(
            launch_env(&account("pro", dir)),
            vec![("CLAUDE_CONFIG_DIR".to_string(), dir.to_string())]
        );
        // Another account with no folder runs as Principal does.
        assert!(launch_env(&account("pro", "")).is_empty());
        assert!(launch_env(&account("pro", "  ")).is_empty());
    }

    #[test]
    fn principals_folder_is_the_apps_claude_config_dir_else_the_home_one() {
        let home = Path::new("/Users/x");
        assert_eq!(
            config_dir_with(&principal(), None, home),
            home.join(".claude")
        );
        assert_eq!(
            config_dir_with(&principal(), Some("/tmp/claude-e2e"), home),
            PathBuf::from("/tmp/claude-e2e")
        );
        // Set empty: as if it were not.
        assert_eq!(
            config_dir_with(&principal(), Some(""), home),
            home.join(".claude")
        );
        // Another account's folder is its own, whatever the app's.
        let pro = account("pro", "/Users/x/.escouade/claude/pro");
        assert_eq!(
            config_dir_with(&pro, Some("/tmp/claude-e2e"), home),
            PathBuf::from("/Users/x/.escouade/claude/pro")
        );
        assert_eq!(
            config_dir_with(&pro, None, home),
            PathBuf::from("/Users/x/.escouade/claude/pro")
        );
    }

    #[test]
    fn an_account_runs_its_own_claude_else_the_settings_one() {
        let dir = test_dir("accounts-program");
        let (own, set) = (dir.join("claude-pro.cmd"), dir.join("claude.cmd"));
        std::fs::write(&own, "").unwrap();
        std::fs::write(&set, "").unwrap();
        let settings = Settings {
            claude_path: set.to_string_lossy().into(),
            ..Default::default()
        };
        let path = |p: &Path| p.to_string_lossy().to_string();
        let with = |claude_path: String| Account {
            claude_path,
            ..account("pro", "/a")
        };
        assert_eq!(program(&with(path(&own)), &settings), Some(own.clone()));
        assert_eq!(program(&with(String::new()), &settings), Some(set.clone()));
        assert_eq!(program(&with("  ".into()), &settings), Some(set));
        // Its own named and not there: not found, rather than another one.
        let gone = path(&dir.join("absent.cmd"));
        assert_eq!(program(&with(gone), &settings), None);
    }

    #[test]
    fn an_unknown_account_is_principal() {
        let mut s = Settings {
            accounts: vec![account("pro", "/a")],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(find(&s, "pro").map(|a| a.id.as_str()), Some("pro"));
        assert_eq!(
            find(&s, "principal").map(|a| a.id.as_str()),
            Some("principal")
        );
        assert_eq!(find(&s, "parti").map(|a| a.id.as_str()), Some("principal"));
        assert_eq!(find(&s, "").map(|a| a.id.as_str()), Some("principal"));
        // Settings never normalized have none.
        assert_eq!(find(&Settings::default(), "pro"), None);
        // A copy, always there.
        assert_eq!(get(&s, "pro"), s.accounts[1]);
        assert_eq!(get(&s, "parti"), s.accounts[0]);
        assert_eq!(get(&Settings::default(), "pro"), principal());
    }
}
