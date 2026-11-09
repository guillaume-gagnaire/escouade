//! Tests of the sort of the seed (ESC-373, docs/PLUGINS.md §8.7): the start installs, of the plugins of the first hand,
//! what the user's configuration uses and nothing else; an update from 2.1 keeps everything that worked; a first start
//! installs nothing; a gesture installs what it needs, with no network, even a plugin the user uninstalled, which the
//! start never puts back; what does not read installs everything.

use crate::i18n::Lang;
use crate::integrations::Accounts;
use crate::model::{Account, EngineId, Project, Settings};
use crate::paths::DataDir;
use crate::plugins::seed::{Entry, Seed};
use crate::plugins::sort::{self, Config, Sorted};
use crate::plugins::{builtin, Origin, Plugins};
use crate::plugins_store_tests::{temp_dir, Fake};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

const DEEPGRAM: &str = "dev.escouade.deepgram";

/// The seed the tests use: the plugins of a configuration of 2.1, and some it does not use.
const SEEDED: &[&str] = &[
    builtin::CLAUDE,
    builtin::CODEX,
    builtin::COPILOT,
    builtin::GITHUB,
    builtin::LINEAR,
    DEEPGRAM,
    sort::ISOLA,
    builtin::ACP,
    builtin::TRELLO,
];

async fn seed_of(fake: &Fake, name: &str) -> Seed {
    let dir = temp_dir(name);
    let list: Vec<Value> = SEEDED
        .iter()
        .map(|id| json!({ "id": id, "version": "2.2.0" }))
        .collect();
    let answer = fake
        .post("/_fake/seed", json!({ "dir": dir, "plugins": list }))
        .await;
    let entries: Vec<Entry> = serde_json::from_value(answer["plugins"].clone()).unwrap();
    Seed {
        dir: Some(dir),
        entries,
        unsigned: false,
    }
}

fn data_of(name: &str) -> DataDir {
    let data = DataDir::new(temp_dir(name).join("data"));
    data.ensure().unwrap();
    data
}

/// The app started on `data` with `seed`, as `core` does it, the sort being `sorted`.
fn started(data: &DataDir, seed: Seed, sorted: &Sorted) -> std::sync::Arc<Plugins> {
    let list = seed.revocations();
    let plugins = Plugins::load_seeded(data.clone(), "2.2.0", seed);
    plugins.apply_store(list, Default::default());
    plugins.sync_seed(Lang::Fr, &|id| sorted.wants(id));
    plugins
}

fn requests(fake_state: &Value) -> usize {
    fake_state["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| !r.as_str().unwrap().contains("/_fake/"))
        .count()
}

/// The plugins of the seed installed from it.
fn from_seed(plugins: &Plugins) -> BTreeSet<String> {
    plugins
        .list(Lang::Fr)
        .into_iter()
        .filter(|v| matches!(v.origin, Origin::Store { seed: true, .. }))
        .map(|v| v.id)
        .collect()
}

fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

fn account(engine: EngineId, id: &str) -> Account {
    Account {
        id: id.into(),
        name: id.into(),
        engine,
        ..Default::default()
    }
}

/// A project in a folder of its own, a git repository whose remote is `remote` (none: no repository).
fn project(name: &str, remote: Option<&str>, links: &[&str]) -> Project {
    let dir = temp_dir(name);
    if let Some(url) = remote {
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(
            dir.join(".git/config"),
            format!(
                "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"
            ),
        )
        .unwrap();
    }
    let links: Vec<Value> = links
        .iter()
        .map(|s| json!({ "service": s, "container": "x", "name": "x" }))
        .collect();
    serde_json::from_value(json!({
        "id": name,
        "name": name,
        "path": dir.to_string_lossy(),
        "color": "#000",
        "integrations": { "links": links },
    }))
    .unwrap()
}

/// The sort of `settings` and `projects`, no agent, no account of a source, no key.
fn sorted(settings: &Settings, projects: &[Project]) -> Sorted {
    sort::sort(&Config {
        settings,
        projects,
        agents: &[],
        accounts: &Accounts::default(),
        voice_keys: &[],
        readable: true,
    })
}

/// The settings of a user of 2.1: a Claude Code account and a Codex one, the dictation on Deepgram.
fn settings_of_2_1() -> Settings {
    let mut s = Settings {
        accounts: vec![
            account(EngineId::CLAUDE, "principal"),
            account(EngineId::CODEX, "codex"),
        ],
        ..Default::default()
    };
    s.voice.transcription = "online".into();
    s.voice.online_service = "deepgram".into();
    s
}

#[tokio::test]
async fn an_update_from_2_1_installs_only_what_the_settings_use_and_keeps_what_worked() {
    let fake = Fake::start().trusted();
    let data = data_of("sort-update");
    // A user of 2.1: every plugin given by the app, Trello switched off.
    let given = Plugins::load_for_core(data.clone(), "2.2.0");
    for id in SEEDED {
        assert_eq!(given.get(id, Lang::Fr).unwrap().origin, Origin::Builtin);
    }
    given
        .choose(builtin::TRELLO, |c| c.enabled = false)
        .unwrap();
    drop(given);

    let seed = seed_of(&fake, "sort-update").await;
    let before = requests(&fake.state().await);
    let projects = [project(
        "notes",
        Some("git@github.com:ada/notes.git"),
        &["linear"],
    )];
    let sorted = sorted(&settings_of_2_1(), &projects);
    let plugins = started(&data, seed.clone(), &sorted);
    assert_eq!(
        requests(&fake.state().await),
        before,
        "no request: the seed reads its files"
    );

    // These five, and nothing else of the seed.
    assert_eq!(
        from_seed(&plugins),
        set(&[
            builtin::CLAUDE,
            builtin::CODEX,
            builtin::GITHUB,
            builtin::LINEAR,
            DEEPGRAM
        ])
    );
    // The others of the seed left it, proposed, never noted as uninstalled.
    for id in [builtin::COPILOT, sort::ISOLA, builtin::ACP, builtin::TRELLO] {
        assert!(plugins.get(id, Lang::Fr).is_none(), "{id} not installed");
    }
    let offers: BTreeSet<String> = plugins
        .seed_offers(Lang::Fr)
        .into_iter()
        .inspect(|o| assert_eq!(o.removed, None, "{}: the user did not take it away", o.id))
        .map(|o| o.id)
        .collect();
    assert_eq!(
        offers,
        set(&[builtin::COPILOT, sort::ISOLA, builtin::ACP, builtin::TRELLO])
    );
    let trello = plugins
        .seed_offers(Lang::Fr)
        .into_iter()
        .find(|o| o.id == builtin::TRELLO)
        .unwrap();
    assert_eq!(trello.name, "Trello", "named as its manifest names it");
    // What the seed does not hold is given by the app as before: nothing that worked is lost.
    assert_eq!(
        plugins.get(builtin::GITLAB, Lang::Fr).unwrap().origin,
        Origin::Builtin
    );
    drop(plugins);

    // The next start changes nothing.
    let plugins = started(&data, seed, &sorted);
    assert_eq!(from_seed(&plugins).len(), 5);
    assert!(plugins.get(builtin::TRELLO, Lang::Fr).is_none());

    // Its gesture brings Trello back, with the choice the user had in 2.1.
    let view = plugins.need(builtin::TRELLO, Lang::Fr).unwrap().unwrap();
    assert!(matches!(view.origin, Origin::Store { seed: true, .. }));
    assert!(!view.choices.enabled, "its settings were kept");
    assert!(plugins.need(builtin::TRELLO, Lang::Fr).unwrap().is_none());
}

#[tokio::test]
async fn a_first_start_installs_nothing_and_an_account_installs_its_harness_with_no_network() {
    let fake = Fake::start().trusted();
    let data = data_of("sort-first");
    let seed = seed_of(&fake, "sort-first").await;
    let before = requests(&fake.state().await);
    let sorted = sorted(&Settings::default(), &[]);
    let plugins = started(&data, seed, &sorted);
    assert!(
        from_seed(&plugins).is_empty(),
        "nothing installed by itself"
    );
    assert_eq!(
        plugins.seed_offers(Lang::Fr).len(),
        SEEDED.len(),
        "all proposed"
    );

    // « Ajouter un compte… » on Copilot CLI: its plugin, from the seed.
    let view = plugins.need(builtin::COPILOT, Lang::Fr).unwrap().unwrap();
    assert_eq!(view.id, builtin::COPILOT);
    assert!(matches!(
        view.origin,
        Origin::Store {
            seed: true,
            verified: true,
            ..
        }
    ));
    assert_eq!(from_seed(&plugins), set(&[builtin::COPILOT]));
    assert_eq!(requests(&fake.state().await), before, "no network");
    // Not in the seed: nothing to do, nothing said.
    assert!(plugins
        .need("dev.exemple.ailleurs", Lang::Fr)
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn a_plugin_uninstalled_is_not_put_back_at_the_start_but_on_the_gesture_that_needs_it() {
    let fake = Fake::start().trusted();
    let data = data_of("sort-removed");
    let seed = seed_of(&fake, "sort-removed").await;
    let sorted = sorted(&settings_of_2_1(), &[]);
    let plugins = started(&data, seed.clone(), &sorted);
    assert!(plugins.get(builtin::CODEX, Lang::Fr).is_some());
    assert!(plugins.remove(builtin::CODEX, true).unwrap());
    drop(plugins);

    // The settings still have a Codex account: the start does not put it back.
    let plugins = started(&data, seed.clone(), &sorted);
    assert!(plugins.get(builtin::CODEX, Lang::Fr).is_none());
    assert!(plugins.uninstalled(builtin::CODEX));

    // An account of Codex added: the gesture installs it again, and the note goes.
    assert!(plugins.need(builtin::CODEX, Lang::Fr).unwrap().is_some());
    assert!(!plugins.uninstalled(builtin::CODEX));
    drop(plugins);
    let plugins = started(&data, seed, &sorted);
    assert!(plugins.get(builtin::CODEX, Lang::Fr).is_some());
}

#[tokio::test]
async fn settings_that_do_not_read_install_every_plugin() {
    let fake = Fake::start().trusted();
    let data = data_of("sort-broken");
    std::fs::write(data.settings_file(), "{ \"accounts\": [ oops").unwrap();
    assert!(!sort::reads(&data.settings_file()));
    assert!(sort::reads(&data.state_file()), "absent: a first start");
    std::fs::write(data.state_file(), "{}").unwrap();
    assert!(sort::reads(&data.state_file()));

    let seed = seed_of(&fake, "sort-broken").await;
    let settings = Settings::default();
    let sorted = sort::sort(&Config {
        settings: &settings,
        projects: &[],
        agents: &[],
        accounts: &Accounts::default(),
        voice_keys: &[],
        readable: sort::reads(&data.settings_file()),
    });
    let plugins = started(&data, seed, &sorted);
    assert_eq!(from_seed(&plugins), set(SEEDED), "nothing lost");
}

#[test]
fn the_harnesses_read_in_the_accounts_the_agents_and_the_projects() {
    let mut settings = Settings {
        accounts: vec![account(EngineId::VIBE, "vibe")],
        ..Default::default()
    };
    settings.writer_engine = Some(EngineId::GEMINI);
    let mut p = project("harness", None, &[]);
    p.engine = Some(EngineId::OPENCODE);
    let s = sort::sort(&Config {
        settings: &settings,
        projects: &[p],
        agents: &[
            EngineId::AGY,
            EngineId::new("goose"),
            EngineId::new("autre"),
        ],
        accounts: &Accounts::default(),
        voice_keys: &[],
        readable: true,
    });
    assert_eq!(
        s.used,
        set(&[
            builtin::MISTRAL,
            builtin::GEMINI,
            builtin::OPENCODE,
            builtin::ANTIGRAVITY,
            sort::GOOSE,
            builtin::ACP,
        ])
    );
    assert!(!s.wants(builtin::CLAUDE) && !s.wants(builtin::CODEX));
}

#[test]
fn the_sources_and_the_code_hosts_read_in_the_projects_and_the_accounts() {
    let mut accounts = Accounts {
        trello: Some(Default::default()),
        // Served by a plugin of another hand: Escouade's is not needed.
        jira: Some(crate::integrations::Account {
            plugin: "dev.exemple.jira".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    accounts.forges.push(crate::forge::accounts::ForgeAccount {
        kind: "gitea".into(),
        url: "https://git.acme.dev".into(),
        ..Default::default()
    });
    let projects = [
        project("lab", Some("https://gitlab.com/ada/lab.git"), &["clickup"]),
        // Self-hosted: known by the account connected for it.
        project(
            "acme",
            Some("ssh://git@git.acme.dev:2222/ada/acme.git"),
            &[],
        ),
        // A server Escouade does not read, no account: no code host.
        project("plain", Some("git@git.example.org:ada/plain.git"), &[]),
        // No repository: no code host.
        project("bare", None, &[]),
    ];
    let s = sort::sort(&Config {
        settings: &Settings::default(),
        projects: &projects,
        agents: &[],
        accounts: &accounts,
        voice_keys: &[],
        readable: true,
    });
    assert_eq!(
        s.used,
        set(&[
            builtin::TRELLO,
            builtin::GITEA,
            builtin::GITLAB,
            builtin::CLICKUP
        ])
    );
}

#[test]
fn a_project_out_of_reach_wants_every_code_host_and_isola() {
    let mut p = project("gone", None, &[]);
    p.path = temp_dir("gone").join("not-there").to_string_lossy().into();
    let s = sorted(&Settings::default(), &[p]);
    for id in [
        builtin::GITHUB,
        builtin::GITLAB,
        builtin::BITBUCKET,
        builtin::GITEA,
        builtin::AZURE_DEVOPS,
        sort::ISOLA,
    ] {
        assert!(s.wants(id), "{id}");
    }
    // A git configuration that does not read: every code host too.
    let p = project("unread", None, &[]);
    std::fs::write(Path::new(&p.path).join(".git"), "gitdir: nowhere").unwrap();
    assert!(sorted(&Settings::default(), &[p]).wants(builtin::BITBUCKET));
}

#[test]
fn isola_reads_its_file_and_the_voice_its_settings_and_its_keys() {
    let p = project("isola", None, &[]);
    std::fs::write(Path::new(&p.path).join(".isola.toml"), "").unwrap();
    let mut settings = Settings::default();
    settings.voice.speech.engine = "online".into();
    settings.voice.speech.online_service = "elevenlabs".into();
    // The service of the dictation, kept while it is local, says nothing.
    settings.voice.online_service = "openai".into();
    let s = sort::sort(&Config {
        settings: &settings,
        projects: &[p],
        agents: &[],
        accounts: &Accounts::default(),
        voice_keys: &["groq".to_string()],
        readable: true,
    });
    assert_eq!(
        s.used,
        set(&[sort::ISOLA, "dev.escouade.elevenlabs", "dev.escouade.groq"])
    );
    assert!(!s.wants("dev.escouade.openai"));
}

#[test]
fn what_the_rule_does_not_know_installs_and_what_is_on_request_does_not() {
    let s = sorted(&Settings::default(), &[]);
    assert!(s.wants("dev.escouade.une-autre"), "no rule: installed");
    for id in [
        "dev.escouade.discord",
        "dev.escouade.slack",
        "dev.escouade.high-contrast",
        "dev.escouade.team-rules",
    ] {
        assert!(!s.wants(id), "{id}: proposed");
        assert!(
            sort::name_on_request(id, Lang::Fr).is_some(),
            "{id}: named in the view"
        );
    }
    // Every id the seed may hold is known to the rule, but the demonstration.
    for e in builtin::REGISTRY.iter().filter(|e| builtin::seeds(e.id)) {
        assert!(!s.wants(e.id), "{} has a rule", e.id);
    }
}
