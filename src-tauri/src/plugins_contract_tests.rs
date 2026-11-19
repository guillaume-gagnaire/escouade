//! Tests of the contract of a plugin's manifest (ESC-525, docs/PLUGINS-CONTRACT-STUDY.md on `release/2.3`): what the
//! app reads but does not serve yet is never served by half, the plugin loaded disabled with the reason; what it serves
//! of it already (`repo: false`, an imported value) is held; a manifest of today reads as it did.

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins::manifest::{self, Hand};
use crate::plugins::{Plugins, State};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("contract-{name}-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn base() -> Value {
    json!({
        "id": "dev.exemple.tunnel",
        "name": "Tunnel",
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.2.0" }
    })
}

fn read(v: &Value) -> manifest::Manifest {
    manifest::check(v, Hand::Third).manifest.unwrap()
}

fn folder(dir: &Path, manifest: &Value) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(manifest::FILE),
        serde_json::to_string_pretty(manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn a_manifest_of_today_and_one_of_what_is_served_ask_for_nothing_unserved() {
    // Today's: no scope, no group, one line, 4 programs, no joker.
    let mut today = base();
    today["permissions"] =
        json!({ "commands": ["isola ls --json", "isola down"], "secrets": ["token"] });
    today["contributes"] = json!({
        "settings": [{ "key": "channel", "type": "string", "title": "Salon" }],
        "secrets": [{ "key": "token", "title": "Jeton" }]
    });
    let m = read(&today);
    assert_eq!(m.unserved(Lang::Fr), None);
    assert_eq!(m.limits, manifest::Limits::default());
    // What reads and is shown as before on an app that does not draw it yet: groups, descriptions, several lines, an
    // import, a scope `app`, a project setting, `repo`, up to 4 programs.
    let mut more = today.clone();
    more["contributes"]["groups"] =
        json!([{ "id": "a", "title": "A", "folded": true, "fields": ["token", "channel"] }]);
    more["contributes"]["settings"][0]["scope"] = json!("project");
    more["contributes"]["settings"][0]["repo"] = json!(false);
    more["contributes"]["secrets"][0] = json!({
        "key": "token", "title": "Jeton", "scope": "app", "multiline": true, "lines": 8,
        "import": [".ovpn"], "description": "Le fichier."
    });
    more["limits"] = json!({ "processes": 4 });
    let m = read(&more);
    assert_eq!(m.limits.processes, Some(4));
    assert_eq!(m.unserved(Lang::En), None);
}

#[test]
fn each_point_not_served_yet_is_said_in_both_languages() {
    let cases = [
        (
            json!({ "contributes": {
                "profiles": [{ "id": "vpn", "title": "VPN", "nameField": "name",
                    "fields": [{ "key": "name", "type": "string", "title": "Nom" }] }]
            } }),
            "des profils (`contributes.profiles`)",
            "profiles (`contributes.profiles`)",
        ),
        (
            json!({ "contributes": { "secrets": [{ "key": "p", "title": "P", "scope": "project" }] } }),
            "un secret propre à chaque projet",
            "a secret of each project",
        ),
        (
            json!({ "permissions": { "commands": ["slirp4netns --configure {pid} tap0"] } }),
            "des jokers dans ses commandes",
            "jokers in its commands",
        ),
        (
            json!({ "limits": { "processes": 8, "reason": "Un tunnel par projet." } }),
            "8 programmes à la fois (`limits.processes`, 4 au plus ici)",
            "8 programs at once (`limits.processes`, 4 at most here)",
        ),
    ];
    for (more, fr, en) in cases {
        let mut v = base();
        for (k, x) in more.as_object().unwrap() {
            v[k] = x.clone();
        }
        let m = read(&v);
        let said = m.unserved(Lang::Fr).unwrap();
        assert!(
            said.starts_with(
                "Ce plugin demande ce que cette version d'Escouade ne sert pas encore : "
            ),
            "{said}"
        );
        assert!(said.contains(fr), "{said}");
        assert!(said.ends_with("Il reste coupé plutôt que servi à moitié ; une version plus récente d'Escouade le servira."), "{said}");
        assert!(m.unserved(Lang::En).unwrap().contains(en));
    }
    // Several at once: each named, in one sentence.
    let mut v = base();
    v["permissions"] = json!({ "commands": ["tool {int}"] });
    v["limits"] = json!({ "processes": 16, "reason": "x" });
    let said = read(&v).unserved(Lang::En).unwrap();
    assert!(
        said.contains("jokers in its commands (`{int}`, `{path:temp}`…), 16 programs at once"),
        "{said}"
    );
}

#[test]
fn a_plugin_that_asks_for_a_point_not_served_loads_disabled_with_the_reason_and_says_it_at_its_install(
) {
    let base_dir = temp_dir("unserved");
    let data = DataDir::new(base_dir.join("data"));
    let src = base_dir.join("src");
    let mut v = base();
    v["contributes"] =
        json!({ "secrets": [{ "key": "password", "title": "Mot de passe", "scope": "project" }] });
    folder(&src, &v);
    let plugins = Plugins::load(data.clone(), "2.2.4");
    // The install window says it before anything is installed.
    let preview = plugins.inspect_folder(&src, Lang::Fr).unwrap();
    let refusal = preview.refusal.clone().unwrap();
    assert!(
        refusal.contains("un secret propre à chaque projet"),
        "{refusal}"
    );
    plugins.discard(&preview.token);
    // Installed, it stays disabled, the reason said; never Ready.
    let view = plugins.install_folder(&src, Lang::Fr).unwrap();
    let State::Disabled { reason } = &view.state else {
        panic!("{:?}", view.state)
    };
    assert_eq!(reason, &refusal);
    let again = Plugins::load(data, "2.2.4");
    assert!(matches!(
        again.list(Lang::En)[0].state,
        State::Disabled { .. }
    ));

    // Today's manifest beside it: Ready, as before.
    let other = base_dir.join("other");
    let mut today = base();
    today["id"] = json!("dev.exemple.slack");
    today["contributes"] = json!({ "secrets": [{ "key": "webhook", "title": "Webhook" }] });
    folder(&other, &today);
    assert_eq!(
        plugins.install_folder(&other, Lang::Fr).unwrap().state,
        State::Ready
    );
}

#[test]
fn a_setting_its_author_keeps_off_the_file_or_imported_never_comes_from_escouade_yml() {
    let mut v = base();
    v["contributes"] = json!({
        "settings": [
            { "key": "route", "type": "string", "title": "Route" },
            { "key": "port", "type": "number", "title": "Port", "repo": true },
            { "key": "server", "type": "string", "title": "Serveur", "repo": false },
            { "key": "conf", "type": "string", "title": "Conf", "multiline": true, "import": [".conf"] }
        ]
    });
    let m = read(&v);
    let mine = json!({ "server": "vpn.exemple.dev" });
    let file = json!({ "route": "10.0.0.0/8", "port": 1194, "server": "evil.exemple", "conf": "up /bin/sh" });
    let got =
        crate::plugins::project_settings(&m, mine.as_object().unwrap(), file.as_object().unwrap());
    assert_eq!(
        Value::Object(got),
        json!({ "route": "10.0.0.0/8", "port": 1194, "server": "vpn.exemple.dev" })
    );
}

#[test]
fn the_secrets_of_a_profile_are_no_key_of_the_plugin_s_page() {
    let mut v = base();
    v["permissions"] = json!({ "secrets": ["profile:vpn", "token"] });
    v["contributes"] = json!({
        "profiles": [{ "id": "vpn", "title": "VPN", "nameField": "name", "fields": [
            { "key": "name", "type": "string", "title": "Nom" },
            { "key": "password", "secret": true, "title": "Mot de passe" }
        ] }],
        "secrets": [{ "key": "token", "title": "Jeton" }]
    });
    assert_eq!(crate::plugins::secret_keys(&read(&v)), ["token"]);
}

#[test]
fn the_grammar_of_the_jokers_holds_word_by_word() {
    for ok in [
        "isola ls --json",
        "tool {int} {port} {name} {pid} {path:temp} {path:plugin}",
        "nsenter -t {pid} *",
        "tool -- {name}",
    ] {
        assert!(manifest::command_holds(ok), "{ok}");
    }
    for refused in [
        "{int} x",
        "tool --pid={int}",
        "tool {float}",
        "tool {path:project}",
        "tool {path:agent}",
        "tool a{b",
        "tool * {int}",
        "isola\u{feff}{int}",
    ] {
        assert!(!manifest::command_holds(refused), "{refused}");
    }
    assert!(manifest::has_joker("tool {pid}"));
    assert!(!manifest::has_joker("tool {pid}x"));
    // The messages of the new codes, as the store's library says them.
    let r = manifest::check(
        &json!({ "id": "dev.exemple.t", "name": "T", "version": "1.0.0", "escouade": { "api": "1", "min": "2.2" },
            "limits": { "processes": 17 },
            "contributes": { "settings": [{ "key": "a", "type": "string", "title": "A", "multiline": true, "lines": 2 }] } }),
        Hand::Third,
    );
    let said: Vec<String> = r
        .manifest
        .unwrap_err()
        .iter()
        .map(|p| p.message(Lang::Fr))
        .collect();
    assert!(
        said.contains(&"`limits.processes` doit être un nombre entier de 1 à 16.".to_string()),
        "{said:?}"
    );
    assert!(
        said.contains(
            &"`contributes.settings[0].lines` doit être un nombre entier de 3 à 20.".to_string()
        ),
        "{said:?}"
    );
}
