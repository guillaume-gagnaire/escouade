//! A plugin's profiles and its settings of each project (ESC-530, issues #38 and #39; docs/PLUGINS-CONTRACT-STUDY.md
//! §2.2 to §2.4 and §3.1): the ranks of a setting of each project, the profiles made, chosen, read and deleted, and the
//! refusals of §2.3 one by one (an id Escouade draws, the plugin alone, a project where it is on, a `.escouade.yml` that
//! carries no secret and chooses no profile, a journal without values, the names kept erased with the plugin).

use crate::core_tests::{harness, repo, Harness};
use crate::i18n::Lang;
use crate::plugins::manifest::{self, Hand};
use crate::plugins::profiles::{self, settings_for};
use crate::plugins::Choices;
use serde_json::{json, Map, Value};

const ID: &str = "dev.test.tunnel";
const PASSWORD_A: &str = "client-a-password-0001";
const PASSWORD_B: &str = "client-b-password-0002";

fn tunnel(run: Value) -> Value {
    let mut v = json!({
        "id": ID, "name": "Tunnel", "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.2.0" },
        "run": run,
        "permissions": { "secrets": ["profile:vpn", "token"] },
        "contributes": {
            "profiles": [{
                "id": "vpn", "title": "Profil VPN", "nameField": "name",
                "fields": [
                    { "key": "name", "type": "string", "title": "Nom du profil" },
                    { "key": "server", "type": "string", "title": "Serveur" },
                    { "key": "port", "type": "number", "title": "Port" },
                    { "key": "password", "secret": true, "title": "Mot de passe" },
                    { "key": "ovpn", "secret": true, "multiline": true, "title": "Configuration" }
                ]
            }],
            "secrets": [{ "key": "token", "title": "Jeton" }],
            "settings": [
                { "key": "profile", "type": "profile", "of": "vpn", "scope": "project", "title": "Profil VPN" },
                { "key": "fallback", "type": "profile", "of": "vpn", "title": "Profil par défaut" },
                { "key": "routes", "type": "string", "scope": "project", "title": "Routes", "default": "0.0.0.0/0" },
                { "key": "port", "type": "number", "scope": "project", "title": "Port", "repo": false },
                { "key": "autoStart", "type": "boolean", "title": "Au lancement", "default": true }
            ]
        }
    });
    if run.is_null() {
        v.as_object_mut().unwrap().remove("run");
    }
    v
}

fn read(v: &Value) -> manifest::Manifest {
    let r = manifest::check(v, Hand::Third);
    r.manifest.unwrap_or_else(|p| panic!("{p:?}"))
}

fn map(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

// ── The ranks, without a process ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_setting_of_each_project_goes_by_its_ranks_machine_then_file_then_default() {
    let m = read(&tunnel(json!(null)));
    let mut c = Choices {
        settings: map(json!({ "autoStart": false, "routes": "kept on the page before" })),
        ..Choices::default()
    };
    // Rank 1, the default: no project, or a project with nothing.
    let none = settings_for(Some(&m), &c, None, false, None);
    assert_eq!(
        none["routes"],
        json!("0.0.0.0/0"),
        "a value of the page is no project's"
    );
    assert_eq!(none["autoStart"], json!(false));
    assert_eq!(none["profile"], Value::Null);
    // Rank 3, the file, ahead of the default; `repo: false` keeps it off.
    let file = map(json!({ "routes": "10.20.0.0/16", "port": 4444, "autoStart": true }));
    let got = settings_for(Some(&m), &c, Some("p1"), true, Some(&file));
    assert_eq!(got["routes"], json!("10.20.0.0/16"));
    assert!(
        got.get("port").is_none(),
        "repo: false: the file's port is not taken"
    );
    assert_eq!(
        got["autoStart"],
        json!(true),
        "a setting of the app: the file over the user's, as before"
    );
    // Rank 4, this machine's value for the project, ahead of the file.
    c.project_settings.insert(
        "p1".into(),
        map(json!({ "routes": "10.0.0.0/8", "port": 1194 })),
    );
    let got = settings_for(Some(&m), &c, Some("p1"), true, Some(&file));
    assert_eq!(got["routes"], json!("10.0.0.0/8"));
    assert_eq!(got["port"], json!(1194));
    // Another project has its own; a value of the wrong type is not taken.
    c.project_settings
        .insert("p2".into(), map(json!({ "routes": 12 })));
    let got = settings_for(Some(&m), &c, Some("p2"), true, None);
    assert_eq!(got["routes"], json!("0.0.0.0/0"));
    // Where the plugin is not on, neither the file nor this machine's value: the default alone.
    let got = settings_for(Some(&m), &c, Some("p1"), false, Some(&file));
    assert_eq!(got["routes"], json!("0.0.0.0/0"));
    assert!(got.get("port").is_none());
}

#[test]
fn a_profile_setting_is_the_project_s_choice_never_the_file_s_and_falls_back_to_the_default_profile(
) {
    let m = read(&tunnel(json!(null)));
    let mut c = Choices::default();
    let a = profiles::Profile {
        id: profiles::new_id(),
        values: map(json!({ "name": "Client A" })),
    };
    let b = profiles::Profile {
        id: profiles::new_id(),
        values: map(json!({ "name": "Client B" })),
    };
    assert!(profiles::is_id(&a.id) && a.id != b.id);
    c.profiles.insert("vpn".into(), vec![a.clone(), b.clone()]);
    // A hostile file names a real profile's id, and a name: neither chooses.
    let file = map(json!({ "profile": a.id, "fallback": b.id }));
    let got = settings_for(Some(&m), &c, Some("p1"), true, Some(&file));
    assert_eq!(got["profile"], Value::Null);
    assert_eq!(got["fallback"], Value::Null);
    // The default of the plugin's page serves a project that chose none.
    c.settings.insert("fallback".into(), json!(b.id));
    let got = settings_for(Some(&m), &c, Some("p1"), true, Some(&file));
    assert_eq!(got["profile"], json!(b.id));
    // The project's choice, ahead of it; « Aucun profil » (null) too.
    c.project_settings
        .insert("p1".into(), map(json!({ "profile": a.id })));
    assert_eq!(
        settings_for(Some(&m), &c, Some("p1"), true, None)["profile"],
        json!(a.id)
    );
    c.project_settings
        .insert("p1".into(), map(json!({ "profile": null })));
    assert_eq!(
        settings_for(Some(&m), &c, Some("p1"), true, None)["profile"],
        Value::Null
    );
    // A profile gone, or a project where the plugin is not on: null.
    c.project_settings
        .insert("p1".into(), map(json!({ "profile": "0123456789abcdef" })));
    assert_eq!(
        settings_for(Some(&m), &c, Some("p1"), true, None)["profile"],
        Value::Null
    );
    c.project_settings
        .insert("p1".into(), map(json!({ "profile": a.id })));
    assert_eq!(
        settings_for(Some(&m), &c, Some("p1"), false, None)["profile"],
        Value::Null
    );
    // What the file names is a proposal, said by its name.
    let named = map(json!({ "profile": "client-a" }));
    assert_eq!(
        profiles::proposed(&m, &named),
        vec![("profile".to_string(), "client-a".to_string())]
    );
}

#[test]
fn a_profile_id_is_drawn_by_escouade_and_its_secret_s_name_is_the_plugin_s_own() {
    let id = profiles::new_id();
    assert!(profiles::is_id(&id));
    for bad in [
        "",
        "../x",
        "0123456789ABCDEF",
        "0123456789abcde",
        "0123456789abcdef0",
        "p:1:token",
    ] {
        assert!(!profiles::is_id(bad), "{bad}");
    }
    assert_eq!(
        profiles::secret_name(ID, "vpn", &id, "password"),
        format!("plugin:{ID}:profile:vpn:{id}:password")
    );
}

// ── With the plugin running ──────────────────────────────────────────────────────────────────────────────────────

struct Setup {
    h: Harness,
    a: String,
    b: String,
    atlas: String,
    portail: String,
}

async fn setup(name: &str) -> Setup {
    let h = harness(name);
    let dir = h.dir.join("tunnel");
    let run = crate::plugins_runtime_tests::fake_module(&dir, &[]);
    std::fs::write(dir.join(manifest::FILE), tunnel(run).to_string()).unwrap();
    h.core.plugins.load_dev(&dir, Lang::Fr).unwrap();
    crate::plugins_runtime_tests::warm(&h.core.plugin_runtime, ID);
    let mut ids = Vec::new();
    for p in ["atlas", "portail"] {
        let at = repo(&h.dir.join(p));
        let (project, _) = h
            .core
            .add_project(
                &at.to_string_lossy(),
                p,
                "#5b8def",
                false,
                None,
                false,
                None,
            )
            .await
            .unwrap();
        ids.push(project.id);
    }
    let a = h
        .core
        .plugin_profile_save(ID, "vpn", None, &map(json!({ "name": "Client A", "server": "vpn.a.example", "port": 1194, "password": "no" })))
        .unwrap();
    let b = h
        .core
        .plugin_profile_save(
            ID,
            "vpn",
            None,
            &map(json!({ "name": "Client B", "server": "gw.b.example" })),
        )
        .unwrap();
    h.core
        .plugin_profile_secret(ID, "vpn", &a, "password", PASSWORD_A, false)
        .unwrap();
    h.core
        .plugin_profile_secret(ID, "vpn", &b, "password", PASSWORD_B, false)
        .unwrap();
    let portail = ids.pop().unwrap();
    let atlas = ids.pop().unwrap();
    Setup {
        h,
        a,
        b,
        atlas,
        portail,
    }
}

async fn ask(h: &Harness, method: &str, params: Value) -> Value {
    h.core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "request", "method": method, "params": params }),
        )
        .await
        .unwrap()
}

fn journal(h: &Harness) -> String {
    h.core
        .plugin_runtime
        .journal(ID)
        .iter()
        .map(|e| e.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test(flavor = "multi_thread")]
async fn two_projects_take_two_profiles_and_the_plugin_reads_the_right_secret_for_each() {
    let s = setup("profiles-two").await;
    let h = &s.h;
    // Only the simple fields are kept with the list; a field it does not declare, or a secret given as one, is not.
    let c = h.core.plugins.choices(ID).unwrap();
    let kept = &c.profiles["vpn"][0];
    assert_eq!(kept.id, s.a);
    assert_eq!(
        Value::Object(kept.values.clone()),
        json!({ "name": "Client A", "server": "vpn.a.example", "port": 1194 })
    );
    // The names of the secrets kept, listed (never their values).
    assert!(c
        .secret_names
        .contains(&profiles::secret_name(ID, "vpn", &s.a, "password")));
    assert!(!serde_json::to_string(&c).unwrap().contains(PASSWORD_A));

    h.core
        .plugin_project_set(ID, &s.atlas, "profile", Some(json!(s.a)))
        .unwrap();
    h.core
        .plugin_project_set(ID, &s.portail, "profile", Some(json!(s.b)))
        .unwrap();
    for (project, profile, password) in
        [(&s.atlas, &s.a, PASSWORD_A), (&s.portail, &s.b, PASSWORD_B)]
    {
        let got = ask(h, "settings/get", json!({ "project": project })).await;
        assert_eq!(got["result"]["profile"], json!(profile));
        let secret = ask(
            h,
            "secrets/get",
            json!({ "key": "password", "profile": profile }),
        )
        .await;
        assert_eq!(secret, json!({ "result": password }));
    }
    let list = ask(h, "profiles/list", json!({ "kind": "vpn" })).await;
    assert_eq!(
        list["result"],
        json!([{ "id": s.a, "name": "Client A" }, { "id": s.b, "name": "Client B" }])
    );
    let one = ask(h, "profiles/get", json!({ "kind": "vpn", "profile": s.b })).await;
    assert_eq!(
        one["result"],
        json!({ "name": "Client B", "server": "gw.b.example" })
    );
    let none = ask(
        h,
        "profiles/get",
        json!({ "kind": "vpn", "profile": "0123456789abcdef" }),
    )
    .await;
    assert_eq!(none["result"], Value::Null);
    // The journal says which profile, never the value.
    let said = journal(h);
    assert!(
        said.contains("was given its secret \"password\" of profile \"Client A\""),
        "{said}"
    );
    assert!(
        !said.contains(PASSWORD_A) && !said.contains(PASSWORD_B),
        "{said}"
    );
    // A secret on several lines kept is said by its lines and size.
    h.core
        .plugin_profile_secret(
            ID,
            "vpn",
            &s.a,
            "ovpn",
            "client\nremote vpn.a.example 1194\n",
            false,
        )
        .unwrap();
    let states = h.core.plugin_profile_secrets(ID).unwrap();
    let a = states.iter().find(|p| p.profile == s.a).unwrap();
    let ovpn = a.secrets.iter().find(|x| x.key == "ovpn").unwrap();
    assert!(ovpn.kept);
    assert_eq!(ovpn.size.map(|z| z.lines), Some(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_refusal_of_section_2_3_holds() {
    let s = setup("profiles-refusals").await;
    let h = &s.h;
    let code = |v: &Value| v["error"]["code"].as_i64();
    // A secret field of a profile asked without its profile: refused, never another secret given.
    let got = ask(h, "secrets/get", json!({ "key": "password" })).await;
    assert_eq!(code(&got), Some(-32602), "{got}");
    // Its own secret of the page still answers as before.
    h.core
        .plugin_secret_set(ID, "token", "tok-123456789", false)
        .unwrap();
    assert_eq!(
        ask(h, "secrets/get", json!({ "key": "token" })).await,
        json!({ "result": "tok-123456789" })
    );
    // A profile's id it made up, or one that is no id: refused.
    for bad in ["0123456789abcdef", "../../token", "", "token"] {
        let got = ask(
            h,
            "secrets/get",
            json!({ "key": "password", "profile": bad }),
        )
        .await;
        assert_eq!(code(&got), Some(-32602), "{bad}: {got}");
    }
    // A key that is no secret field of the kind (a simple field, its own token): refused.
    for key in ["server", "token", "nope"] {
        let got = ask(h, "secrets/get", json!({ "key": key, "profile": s.a })).await;
        assert_eq!(code(&got), Some(-32602), "{key}: {got}");
    }
    // A kind it does not declare.
    let got = ask(h, "profiles/list", json!({ "kind": "other" })).await;
    assert_eq!(code(&got), Some(-32602), "{got}");
    // `profile:vpn` taken back by the user: the plugin alone may read them, and only while it holds the permission.
    h.core
        .plugins
        .choose(ID, |c| c.revoked.push("secrets:profile:vpn".into()))
        .unwrap();
    h.core.plugin_runtime.forget(ID);
    let got = ask(
        h,
        "secrets/get",
        json!({ "key": "password", "profile": s.a }),
    )
    .await;
    assert_eq!(code(&got), Some(-32001), "{got}");
    let said = journal(h);
    assert!(!said.contains(PASSWORD_A), "{said}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_project_gives_its_choice_only_once_the_plugin_is_on_there_and_a_hostile_file_chooses_nothing(
) {
    let s = setup("profiles-private").await;
    let h = &s.h;
    h.core
        .plugin_project_set(ID, &s.atlas, "profile", Some(json!(s.a)))
        .unwrap();
    // A hostile `.escouade.yml` names the other profile by its id, sets a secret and a port kept off the file.
    let atlas = h.core.project(&s.atlas).unwrap();
    std::fs::write(
        std::path::Path::new(&atlas.path).join(".escouade.yml"),
        format!("version: 1\nplugins:\n  {ID}:\n    profile: {}\n    password: stolen\n    port: 6666\n    routes: 10.20.0.0/16\n", s.b),
    )
    .unwrap();
    h.core.read_shared(&s.atlas, false);
    let got = ask(h, "settings/get", json!({ "project": s.atlas })).await["result"].clone();
    assert_eq!(got["profile"], json!(s.a), "the file chose nothing");
    assert_eq!(got["routes"], json!("10.20.0.0/16"));
    assert!(
        got.get("password").is_none() && got.get("port").is_none(),
        "{got}"
    );
    // Another project chose none: no default, null; the file's id is no choice there either.
    let got = ask(h, "settings/get", json!({ "project": s.portail })).await["result"].clone();
    assert_eq!(got["profile"], Value::Null);

    // Private: nothing reaches it while the plugin is not on there.
    let mut p = h.core.project(&s.atlas).unwrap();
    p.private = true;
    h.core.update_project(p).unwrap();
    let got = ask(h, "settings/get", json!({ "project": s.atlas })).await["result"].clone();
    assert_eq!(got["profile"], Value::Null);
    assert_eq!(got["routes"], json!("0.0.0.0/0"));
    h.core
        .plugins
        .choose(ID, |c| {
            c.projects.insert(s.atlas.clone(), true);
        })
        .unwrap();
    let got = ask(h, "settings/get", json!({ "project": s.atlas })).await["result"].clone();
    assert_eq!(got["profile"], json!(s.a));
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_profile_erases_its_secrets_and_leaves_its_projects_with_none() {
    let s = setup("profiles-delete").await;
    let h = &s.h;
    h.core
        .plugin_project_set(ID, &s.atlas, "profile", Some(json!(s.a)))
        .unwrap();
    h.core
        .plugins
        .choose(ID, |c| {
            c.settings.insert("fallback".into(), json!(s.a));
        })
        .unwrap();
    let name = profiles::secret_name(ID, "vpn", &s.a, "password");
    assert_eq!(
        h.core.secrets.get(&name).unwrap().as_deref(),
        Some(PASSWORD_A)
    );
    let gone = h.core.plugin_profile_delete(ID, "vpn", &s.a).unwrap();
    assert_eq!(gone.projects, vec![s.atlas.clone()]);
    assert_eq!(h.core.secrets.get(&name).unwrap(), None);
    let c = h.core.plugins.choices(ID).unwrap();
    assert!(!c.secret_names.contains(&name));
    assert_eq!(c.project_settings[&s.atlas]["profile"], Value::Null);
    assert!(
        c.settings.get("fallback").is_none(),
        "the default that named it is forgotten"
    );
    assert_eq!(c.profiles["vpn"].len(), 1);
    let got = ask(h, "settings/get", json!({ "project": s.atlas })).await;
    assert_eq!(got["result"]["profile"], Value::Null);
    let secret = ask(
        h,
        "secrets/get",
        json!({ "key": "password", "profile": s.a }),
    )
    .await;
    assert_eq!(secret["error"]["code"], json!(-32602));
    // The other profile is untouched.
    let b = profiles::secret_name(ID, "vpn", &s.b, "password");
    assert_eq!(h.core.secrets.get(&b).unwrap().as_deref(), Some(PASSWORD_B));
}

#[tokio::test(flavor = "multi_thread")]
async fn uninstalling_the_plugin_erases_every_name_it_kept() {
    let s = setup("profiles-uninstall").await;
    let h = &s.h;
    h.core
        .plugin_secret_set(ID, "token", "tok-123456789", false)
        .unwrap();
    let names = h.core.plugins.choices(ID).unwrap().secret_names;
    assert_eq!(names.len(), 3, "{names:?}");
    // Another plugin's name in the keychain is never touched, even listed by mistake.
    let other = "plugin:dev.other:token";
    h.core.secrets.set(other, "theirs").unwrap();
    h.core
        .plugins
        .choose(ID, |c| c.secret_names.push(other.into()))
        .unwrap();
    assert!(h.core.plugin_uninstall(ID, false).unwrap());
    for n in &names {
        assert_eq!(h.core.secrets.get(n).unwrap(), None, "{n}");
    }
    assert_eq!(
        h.core.secrets.get(other).unwrap().as_deref(),
        Some("theirs")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_setting_of_each_project_is_written_into_escouade_yml_key_by_key_and_comments_stay() {
    let s = setup("profiles-write").await;
    let h = &s.h;
    let atlas = h.core.project(&s.atlas).unwrap();
    let file = std::path::Path::new(&atlas.path).join(".escouade.yml");
    std::fs::write(
        &file,
        "# Le fichier de l'équipe\nversion: 1\nagents:\n  engine: claude # le harness\n",
    )
    .unwrap();
    h.core
        .plugin_project_set(ID, &s.atlas, "routes", Some(json!("10.0.0.0/8")))
        .unwrap();
    h.core
        .plugin_project_set(ID, &s.atlas, "port", Some(json!(1194)))
        .unwrap();
    h.core
        .plugin_project_set(ID, &s.atlas, "profile", Some(json!(s.a)))
        .unwrap();
    h.core.plugin_project_write(ID, &s.atlas, "routes").unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("# Le fichier de l'équipe") && text.contains("engine: claude # le harness"),
        "{text}"
    );
    assert!(
        text.contains(&format!("plugins:\n  {ID}:\n    routes: 10.0.0.0/8")),
        "{text}"
    );
    // Written, it follows the file again: no value of this machine for it.
    let c = h.core.plugins.choices(ID).unwrap();
    assert!(c.project_settings[&s.atlas].get("routes").is_none());
    let got = ask(h, "settings/get", json!({ "project": s.atlas })).await;
    assert_eq!(got["result"]["routes"], json!("10.0.0.0/8"));
    // Never a profile, nor what its author keeps off the file.
    assert!(h
        .core
        .plugin_project_write(ID, &s.atlas, "profile")
        .is_err());
    assert!(h.core.plugin_project_write(ID, &s.atlas, "port").is_err());
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        !text.contains(&s.a) && !text.contains("port: 1194"),
        "{text}"
    );
    // The export of the whole project still writes no `plugins.*` of its own.
    let values = crate::projectfile::export_values(&h.core.project(&s.atlas).unwrap());
    assert!(values.iter().all(|(k, _)| !k.starts_with("plugins")));
    // A setting of the app's scope is not one of a project's.
    assert!(h
        .core
        .plugin_project_set(ID, &s.atlas, "autoStart", Some(json!(true)))
        .is_err());
}
