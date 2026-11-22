//! TS-12: the single list of the policies as the core's table, the keys it adds to TS-11's seven, the enrollment and
//! the refusal of another server, and the policies read again while the app runs.

use super::*;
use crate::core_tests::harness;
use crate::model::Settings;
use serde_json::json;

fn managed(pairs: &[(&str, Value)]) -> Managed {
    Managed {
        values: pairs
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_owned(),
                    (v.clone(), "the machine's policy".to_owned()),
                )
            })
            .collect(),
        warnings: Vec::new(),
    }
}

fn team(enforced: &[(&str, Value)]) -> TeamPolicies {
    TeamPolicies {
        id: "acme".into(),
        team: "Acme".into(),
        signed: None,
        recommended: BTreeMap::new(),
        enforced: enforced
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    }
}

fn locks(pairs: &[(&str, Value)]) -> Locks {
    Locks::compute(KEYS, None, &managed(pairs))
}

#[test]
fn the_tables_modes_and_efforts_are_the_cores() {
    assert!(std::ptr::eq(MODES, crate::modes::ORDER));
    assert_eq!(EFFORTS, ["low", "medium", "high", "xhigh", "max"]);
    // The app's table is the list the ADMX is generated from: one and the same.
    assert!(std::ptr::eq(KEYS, escouade_policy::list::POLICIES));
}

#[test]
fn the_new_settings_are_brought_within_the_policies_and_a_save_against_them_is_refused() {
    let locks = locks(&[
        (AUTO_UPDATE, json!(false)),
        (DENY_INSECURE_TLS, json!(true)),
        (VERIFIED_ONLY, json!(true)),
        (FORCE_PRIVATE, json!(true)),
        (PROXY_URL, json!("http://proxy.acme:3128")),
        (PROXY_EXCLUSIONS, json!(["localhost", ".acme.corp"])),
        (AUTHORITIES, json!(r"C:\ProgramData\Acme\root.pem")),
        (VOICE_ONLINE, json!(false)),
    ]);
    assert!(locks.warnings.is_empty(), "{:?}", locks.warnings);
    let mut s = Settings {
        auto_update: true,
        insecure_tls: true,
        plugins_verified_only: false,
        private_projects: false,
        proxy_url: "http://mine:8080".into(),
        no_proxy: "localhost".into(),
        ..Default::default()
    };
    s.voice.transcription = "online".into();
    s.voice.speech.engine = "online".into();
    let changed = locks.clamp_settings(&mut s);
    assert_eq!(changed.len(), 8, "{changed:?}");
    assert!(!s.auto_update && !s.insecure_tls && s.plugins_verified_only && s.private_projects);
    assert_eq!(s.proxy_url, "http://proxy.acme:3128");
    assert_eq!(s.no_proxy, "localhost,.acme.corp");
    assert_eq!(s.extra_ca_file, r"C:\ProgramData\Acme\root.pem");
    assert_eq!(
        (
            s.voice.transcription.as_str(),
            s.voice.speech.engine.as_str()
        ),
        ("local", "local")
    );
    // Brought within, nothing more to change; the same exclusions written with spaces are the same.
    s.no_proxy = "localhost, .acme.corp".into();
    assert!(locks.clamp_settings(&mut s.clone()).is_empty());
    assert!(locks.check_settings(&s).is_ok());
    // A save that turns one back is refused, and says what imposes it.
    let mut back = s.clone();
    back.auto_update = true;
    let err = locks.check_settings(&back).unwrap_err().to_string();
    assert!(err.contains("updates.automatic"), "{err}");
    let mut proxy = s.clone();
    proxy.proxy_url = "http://other:1".into();
    assert!(locks
        .check_settings(&proxy)
        .unwrap_err()
        .to_string()
        .contains("proxy.url"));
    // The rows they lock are greyed by the window.
    assert_eq!(locks.rows["autoUpdate"], [AUTO_UPDATE]);
    assert_eq!(locks.rows["noProxy"], [PROXY_EXCLUSIONS]);
    assert_eq!(locks.rows["private"], [FORCE_PRIVATE]);
}

#[test]
fn a_registry_number_of_a_new_flag_and_a_widening_value_are_read_or_said() {
    // What escouade-policy gives a REG_DWORD 1: the list says Boolean, the core takes it.
    let ok = locks(&[(VERIFIED_ONLY, json!(1)), (AUTO_UPDATE, json!(0))]);
    assert_eq!(ok.value(VERIFIED_ONLY), Some(&json!(true)));
    assert_eq!(ok.value(AUTO_UPDATE), Some(&json!(false)));
    // A value that widens is ignored, and said.
    let wide = locks(&[(VERIFIED_ONLY, json!(false)), (AUTO_UPDATE, json!(true))]);
    assert!(wide.locks.is_empty());
    assert_eq!(wide.warnings.len(), 2, "{:?}", wide.warnings);
}

#[test]
fn a_ceiling_holds_the_lower_and_a_union_holds_all() {
    let device = managed(&[
        (MAX_EFFORT, json!("high")),
        (PLUGINS_DENIED, json!(["a.b"])),
    ]);
    let acme = team(&[
        (MAX_EFFORT, json!("medium")),
        (PLUGINS_DENIED, json!(["c.d", "a.b"])),
    ]);
    let both = Locks::compute(KEYS, Some(&acme), &device);
    assert_eq!(both.text(MAX_EFFORT), Some("medium"));
    assert_eq!(
        both.get(MAX_EFFORT).unwrap().by,
        Some(LockedBy::Team {
            team: "Acme".into()
        })
    );
    assert_eq!(both.value(PLUGINS_DENIED), Some(&json!(["a.b", "c.d"])));
    // The device's lower ceiling wins over the team's.
    let lower = Locks::compute(KEYS, Some(&team(&[(MAX_EFFORT, json!("max"))])), &device);
    assert_eq!(lower.text(MAX_EFFORT), Some("high"));
}

#[test]
fn an_effort_above_the_ceiling_and_a_model_not_allowed_are_refused() {
    let locks = locks(&[
        (MAX_EFFORT, json!("high")),
        (MODELS, json!({"claude": ["claude-sonnet-5-5"]})),
    ]);
    assert!(locks.check_effort("high").is_ok() && locks.check_effort("low").is_ok());
    let err = locks.check_effort("max").unwrap_err().to_string();
    assert!(err.contains("« max »") && err.contains("« high »"), "{err}");
    // An effort a harness names that the list does not know passes, as none.
    assert!(locks.check_effort("minimal").is_ok() && locks.check_effort("").is_ok());
    assert_eq!(locks.effort_within("xhigh"), "high");
    assert_eq!(locks.effort_within("medium"), "medium");
    assert!(locks.check_model("claude", "claude-sonnet-5-5").is_ok());
    assert!(locks.check_model("claude", "").is_ok(), "its default");
    assert!(
        locks.check_model("codex", "gpt-5.5").is_ok(),
        "a harness not named"
    );
    let err = locks
        .check_model("claude", "claude-opus-5-5")
        .unwrap_err()
        .to_string();
    assert!(err.contains("claude-opus-5-5"), "{err}");
    // The repository's file may not raise it.
    assert!(locks.file_refusal("board.effort", &json!("max")).is_some());
    assert!(locks.file_refusal("board.effort", &json!("low")).is_none());
    assert!(locks.bounds_file_key("board.effort"));
}

#[test]
fn a_plugin_is_refused_by_its_id_or_its_origin() {
    let locks = locks(&[
        (PLUGIN_ORIGINS_KEY, json!(["store-verified", "git"])),
        (PLUGINS_DENIED, json!(["com.bad"])),
    ]);
    assert!(locks
        .check_plugin("dev.escouade.github", "store-verified")
        .is_ok());
    assert!(locks.check_plugin("com.example.x", "git").is_ok());
    assert!(locks
        .check_plugin("com.example.x", "folder")
        .unwrap_err()
        .to_string()
        .contains("folder"));
    assert!(
        locks.check_plugin("com.example.x", "store").is_err(),
        "an unverified version"
    );
    assert!(locks
        .check_plugin("com.bad", "store-verified")
        .unwrap_err()
        .to_string()
        .contains("com.bad"));
    // A list of the ids allowed.
    let only = self::locks(&[(PLUGINS_ALLOWED, json!(["a.one"]))]);
    assert!(only.check_plugin("a.one", "folder").is_ok());
    assert!(only.check_plugin("a.two", "store-verified").is_err());
    // `store` allows a verified version too; « versions vérifiées seulement » refuses the others.
    let store = self::locks(&[
        (PLUGIN_ORIGINS_KEY, json!(["store"])),
        (VERIFIED_ONLY, json!(true)),
    ]);
    assert!(store.check_plugin("x.y", "store-verified").is_ok());
    assert!(store.check_plugin("x.y", "store").is_err());
    use crate::plugins::Origin;
    assert_eq!(
        origin_name(&Origin::Folder {
            path: String::new()
        }),
        "folder"
    );
    assert_eq!(origin_name(&Origin::Workspace), "store-verified");
}

#[test]
fn the_dictation_and_the_outside_mcp_clients_follow_the_policy() {
    let none = Locks::default();
    assert!(none.check_dictation().is_ok());
    assert_eq!(none.mcp_external(), "allowed");
    let locks = locks(&[(DICTATION, json!(false)), (MCP_EXTERNAL, json!("readOnly"))]);
    assert!(locks
        .check_dictation()
        .unwrap_err()
        .to_string()
        .contains("voice.dictation"));
    assert_eq!(locks.mcp_external(), "readOnly");
    assert_eq!(locks.rows["dictation"], [DICTATION]);
    use crate::mcp::tools::reads_only;
    assert!(reads_only("get_board") && reads_only("search_spec") && reads_only("wait_ticket"));
    assert!(!reads_only("update_ticket") && !reads_only("create_agent"));
}

#[test]
fn the_enrollment_is_kept_and_any_other_server_is_refused() {
    let locks = locks(&[
        (TEAM_SERVER, json!("https://Escouade.Acme.com/")),
        (TEAM_FINGERPRINT, json!("SHA256:abcd")),
        (ONLY_THIS_SERVER, json!(true)),
    ]);
    assert_eq!(
        locks.enrollment(),
        Some(Enrollment {
            server: "https://Escouade.Acme.com/".into(),
            fingerprint: Some("SHA256:abcd".into()),
            only_this_server: true,
        })
    );
    // Its own server, however written, with its team's fingerprint.
    assert!(locks
        .check_server("https://escouade.acme.com", Some("sha256:ABCD"))
        .is_ok());
    assert!(locks
        .check_server("HTTPS://escouade.acme.com/", None)
        .is_ok());
    // Another server, or another team on it: refused, and said.
    let err = locks
        .check_server("https://evil.example.com", None)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("https://evil.example.com") && err.contains("https://escouade.acme.com"),
        "{err}"
    );
    let err = locks
        .check_server("https://escouade.acme.com", Some("SHA256:ffff"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("empreinte"), "{err}");
    // Without the refusal, any server; with a list, only those of the list.
    let open = self::locks(&[(TEAM_SERVER, json!("https://a.com"))]);
    assert!(open.check_server("https://b.com", None).is_ok());
    assert!(!open.enrollment().unwrap().only_this_server);
    let listed = self::locks(&[(SERVERS, json!(["https://a.com", "https://b.com/"]))]);
    assert!(listed.check_server("https://b.com", None).is_ok());
    assert!(listed
        .check_server("https://c.com", None)
        .unwrap_err()
        .to_string()
        .contains("c.com"));
    // Refusing any other server and setting none refuses them all.
    let none = self::locks(&[(ONLY_THIS_SERVER, json!(true))]);
    assert!(none.check_server("https://a.com", None).is_err());
    assert_eq!(none.enrollment(), None);
    assert_eq!(server_address(" https://A.com/x/ "), "https://a.com/x");
}

#[test]
fn the_e2e_source_is_watched_and_a_change_of_its_file_is_told_typed() {
    // nextest runs each test in a process of its own: the variable stays in this one.
    let dir = std::env::temp_dir().join(format!("escouade-policies-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("policies.json");
    std::fs::write(&file, r#"{"telemetry.enabled": false}"#).unwrap();
    std::env::set_var(["ESCOUADE_TEST", "_POLICIES"].concat(), &file);
    let first = Managed::read();
    assert_eq!(first.values["telemetry.enabled"].0, json!(false));
    let (tell, seen) = std::sync::mpsc::channel();
    let _watch = Managed::watch(move |m| {
        let _ = tell.send(m);
    });
    std::thread::sleep(std::time::Duration::from_millis(300));
    std::fs::write(
        &file,
        r#"{"telemetry.enabled": false, "updates.automatic": 0}"#,
    )
    .unwrap();
    let got = seen
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("told");
    // A 0 the list says Boolean.
    assert_eq!(got.values["updates.automatic"].0, json!(false));
    assert!(got.values["updates.automatic"].1.ends_with("policies.json"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_policy_changed_while_the_app_runs_applies_at_once_to_the_settings_and_the_projects() {
    let h = harness("locks-live");
    let (p, _) = h.project(false).await;
    assert!(h.core.settings.read().auto_update);
    assert!(!h.core.project(&p.id).unwrap().private);
    h.core.set_managed(managed(&[
        (AUTO_UPDATE, json!(false)),
        (FORCE_PRIVATE, json!(true)),
        (MAX_EFFORT, json!("medium")),
    ]));
    assert!(!h.core.settings.read().auto_update);
    let project = h.core.project(&p.id).unwrap();
    assert!(project.private);
    // A change of the project against them is refused: made public again, its board's effort raised.
    let mut public = project.clone();
    public.private = false;
    assert!(h
        .core
        .locks
        .read()
        .check_project(&project, &public)
        .is_err());
    let mut board = project.board.clone();
    board.effort = "max".into();
    let err = h.core.board_set(&p.id, board).unwrap_err().to_string();
    assert!(err.contains("max"), "{err}");
    // The window is told the locks.
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "locks" && e["locks"]["locks"]["updates.automatic"].is_object()));
    // A policy removed: the setting is free again, as it was left.
    h.core.set_managed(Managed::default());
    assert!(h.core.locks.read().locks.is_empty());
    assert!(!h.core.settings.read().auto_update);
}

#[tokio::test]
async fn an_agent_of_a_harness_no_longer_allowed_stops_between_two_turns_and_says_it() {
    let h = harness("locks-harness");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    assert!(
        h.core.agent(&a).unwrap().lock().proc.is_some(),
        "its process kept"
    );

    h.core
        .set_managed(managed(&[(HARNESSES, json!(["codex"]))]));
    h.wait("the agent to say it stops", |h| {
        h.items(&a).iter().any(|i| i["kind"] == "notice")
    })
    .await;
    assert!(h.core.agent(&a).unwrap().lock().proc.is_none());
    let said = h.items(&a);
    assert!(
        said.iter().any(|i| i["kind"] == "notice"
            && i["text"] == "Le harness « claude » n’est pas permis par la stratégie de ce poste. L’agent s’arrête à la fin de son tour."),
        "{said:?}"
    );
    // Its next message is refused.
    let err = h
        .core
        .send_message(&a, "encore".into(), vec![])
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("claude"), "{err}");
}

#[test]
fn a_server_reached_by_ssh_is_refused_before_any_connection_when_the_list_does_not_hold_it() {
    // ESC-537: `servers.allowed` holds hosts and domains (`*.acme.internal`), written however.
    let listed = locks(&[(
        SERVERS,
        json!(["*.acme.internal", "ssh://escouade@VPS.example.net:2222"]),
    )]);
    assert!(listed.check_ssh_server("escouade.acme.internal").is_ok());
    assert!(listed.check_ssh_server("vps.example.net").is_ok());
    let err = listed
        .check_ssh_server("autre.example.org")
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("autre.example.org") && err.contains("*.acme.internal"),
        "{err}"
    );
    // The domain itself is not one of its machines' names.
    assert!(listed.check_ssh_server("acme.internal").is_err());
    // Without the policy, any server.
    assert!(locks(&[]).check_ssh_server("x.example").is_ok());
}

#[test]
fn without_the_personal_space_only_a_server_of_the_list_is_added() {
    // ESC-539, S16: `servers.personalSpace` to false: a server must be one of `servers.allowed`.
    let listed = locks(&[
        (PERSONAL_SPACE, json!(false)),
        (SERVERS, json!(["*.acme.internal"])),
    ]);
    assert!(listed.check_ssh_server("escouade.acme.internal").is_ok());
    assert!(listed.check_ssh_server("vps.example.net").is_err());
    // Without the list, none.
    let none = locks(&[(PERSONAL_SPACE, json!(false))]);
    let err = none
        .check_ssh_server("vps.example.net")
        .unwrap_err()
        .to_string();
    assert!(err.contains("vps.example.net"), "{err}");
    // `true` imposes nothing.
    assert!(locks(&[(PERSONAL_SPACE, json!(true))])
        .check_ssh_server("vps.example.net")
        .is_ok());
}

// ---------- the reservation of an account (ESC-564) ----------

fn account(id: &str, projects: &[&str]) -> crate::model::Account {
    crate::model::Account {
        id: id.into(),
        name: id.to_uppercase(),
        projects: projects.iter().map(|p| (*p).to_owned()).collect(),
        ..Default::default()
    }
}

fn private(id: &str) -> Project {
    let mut p = crate::core::blank_project("/p");
    p.id = id.into();
    p.name = id.into();
    p.private = true;
    p
}

#[test]
fn a_reservation_denied_empties_each_accounts_list_and_a_save_that_reserves_is_refused() {
    let locks = locks(&[(ACCOUNT_RESERVATION, json!("denied"))]);
    assert!(locks.warnings.is_empty(), "{:?}", locks.warnings);
    assert!(locks.reservation_denied() && !locks.reservation_required());
    let mut s = Settings {
        accounts: vec![account("main", &[]), account("pro", &["client"])],
        ..Default::default()
    };
    assert_eq!(locks.clamp_settings(&mut s), [ACCOUNT_RESERVATION]);
    assert!(s.accounts.iter().all(|a| a.projects.is_empty()));
    assert!(locks.check_settings(&s).is_ok());
    // « Réserver … à ce projet », a box of « Réservé aux projets »: refused, said with who imposes it.
    let mut reserve = s.clone();
    reserve.accounts[1].projects = vec!["client".into()];
    let err = locks.check_settings(&reserve).unwrap_err().to_string();
    assert_eq!(
        err,
        "Réserver un compte à un projet est interdit par la stratégie de ce poste (accounts.reservation) : un projet privé part sur les comptes partagés, sa clôture protège ses sessions."
    );
    // A private project goes on the shared accounts, as in 2.2.5: nothing refuses them.
    let client = private("client");
    assert!(locks
        .reservation_refusal(&s, &client, crate::model::EngineId::CLAUDE, Some("main"))
        .is_none());
    // A value outside its choices locks nothing, and is said.
    let odd = super::policies_tests::locks(&[(ACCOUNT_RESERVATION, json!("sometimes"))]);
    assert!(odd.locks.is_empty());
    assert_eq!(odd.warnings.len(), 1, "{:?}", odd.warnings);
}

#[test]
fn a_reservation_required_keeps_a_private_project_on_its_own_accounts_and_its_last_one() {
    let locks = locks(&[(ACCOUNT_RESERVATION, json!("required"))]);
    let claude = crate::model::EngineId::CLAUDE;
    let mut client = private("client");
    client.shared_on_limit = true;
    // « Comptes partagés à la limite », off and locked.
    assert_eq!(locks.clamp_project(&mut client), [ACCOUNT_RESERVATION]);
    assert!(!client.shared_on_limit);
    let mut on = client.clone();
    on.shared_on_limit = true;
    let err = locks.check_project(&client, &on).unwrap_err().to_string();
    assert!(err.contains("accounts.reservation"), "{err}");
    // No account of its own: the automatic choice is refused, and says what to do.
    let shared = Settings {
        accounts: vec![account("main", &[]), account("pro", &["client", "site"])],
        ..Default::default()
    };
    let err = locks
        .reservation_refusal(&shared, &client, claude, None)
        .unwrap();
    assert_eq!(
        err,
        "Le projet privé « client » ne part que sur un compte réservé à lui seul, imposé par la stratégie de ce poste (accounts.reservation) : aucun compte Claude Code actif ne lui est réservé. Réserve-lui un compte dans Réglages › Projets › Confidentialité."
    );
    // One reserved to it alone: the automatic choice may, a shared account named is refused.
    let own = Settings {
        accounts: vec![account("main", &[]), account("pro", &["client"])],
        ..Default::default()
    };
    assert!(locks
        .reservation_refusal(&own, &client, claude, None)
        .is_none());
    assert!(locks
        .reservation_refusal(&own, &client, claude, Some("pro"))
        .is_none());
    let err = locks
        .reservation_refusal(&own, &client, claude, Some("main"))
        .unwrap();
    assert!(
        err.ends_with(": « MAIN » est partagé avec d’autres projets."),
        "{err}"
    );
    // A project that is not private goes where it goes.
    let mut site = private("site");
    site.private = false;
    assert!(locks
        .reservation_refusal(&shared, &site, claude, Some("main"))
        .is_none());
    // « Ne plus le réserver » on its last account, « Partager à nouveau » or « Inactif »: refused; with another left, free.
    let projects = [client.clone()];
    let unreserved = [account("main", &[]), account("pro", &[])];
    let err = locks
        .check_reservations(&projects, &own.accounts, &unreserved)
        .unwrap_err()
        .to_string();
    assert!(err.contains("« PRO » est le dernier"), "{err}");
    let shared_again = [account("main", &[]), account("pro", &["client", "site"])];
    assert!(locks
        .check_reservations(&projects, &own.accounts, &shared_again)
        .is_err());
    let mut off = own.accounts.clone();
    off[1].active = false;
    assert!(locks
        .check_reservations(&projects, &own.accounts, &off)
        .is_err());
    let two = [account("main", &["client"]), account("pro", &["client"])];
    assert!(locks
        .check_reservations(
            &projects,
            &two,
            &[account("main", &[]), account("pro", &["client"])]
        )
        .is_ok());
    // Without the policy, nothing of it.
    let free = super::policies_tests::locks(&[]);
    assert!(free
        .check_reservations(&projects, &own.accounts, &unreserved)
        .is_ok());
    assert!(free
        .reservation_refusal(&shared, &client, claude, None)
        .is_none());
}

#[test]
fn a_team_imposes_the_reservation_as_the_device_does_and_the_device_wins() {
    let acme = team(&[(ACCOUNT_RESERVATION, json!("required"))]);
    let by_team = Locks::compute(KEYS, Some(&acme), &Managed::default());
    assert!(by_team.warnings.is_empty(), "{:?}", by_team.warnings);
    assert!(by_team.reservation_required());
    let s = Settings {
        accounts: vec![account("main", &[])],
        ..Default::default()
    };
    let err = by_team
        .reservation_refusal(&s, &private("client"), crate::model::EngineId::CLAUDE, None)
        .unwrap();
    assert!(
        err.contains("imposé par l’équipe Acme (accounts.reservation)"),
        "{err}"
    );
    // The key is the app's, not a project's: it holds for the app while the device is in the team.
    assert!(!project_scoped(ACCOUNT_RESERVATION));
    // The device's value wins over the team's.
    let both = Locks::compute(
        KEYS,
        Some(&acme),
        &managed(&[(ACCOUNT_RESERVATION, json!("denied"))]),
    );
    assert!(both.reservation_denied());
}

#[tokio::test]
async fn a_private_project_goes_only_on_its_own_account_while_the_policy_requires_it() {
    let h = harness("locks-reserve");
    let (p, _) = h.project(false).await;
    let mut project = h.core.project(&p.id).unwrap();
    project.private = true;
    project.shared_on_limit = true;
    h.core.update_project(project).unwrap();
    // Claude Code turned on for it by the user (ESC-404).
    crate::harness_scope_tests::turn_on_every_harness(&h, &p.id);
    let pro = crate::core_tests::second_account(&h);
    h.core
        .set_managed(managed(&[(ACCOUNT_RESERVATION, json!("required"))]));
    // Brought within at once: « Comptes partagés à la limite » off.
    assert!(!h.core.project(&p.id).unwrap().shared_on_limit);
    // No account of its own yet: no agent starts, said.
    let err = h
        .core
        .create_agent(&p.id, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("aucun compte Claude Code actif ne lui est réservé"),
        "{err}"
    );
    // « Réserver Pro à ce projet »: its agents go there, never on Principal.
    let mut reserved = pro.clone();
    reserved.projects = vec![p.id.clone()];
    h.core.update_claude_account(reserved.clone()).unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert_eq!(a.account, "pro");
    let principal = crate::accounts::principal().id;
    let err = h
        .core
        .create_agent_with(
            &p.id,
            crate::core::AgentOptions {
                account: Some(principal.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("est partagé avec d’autres projets"), "{err}");
    // « Ne plus le réserver » on its last account: refused, and the account stays its own.
    let mut back = reserved.clone();
    back.projects.clear();
    let err = h.core.update_claude_account(back).unwrap_err().to_string();
    assert!(err.contains("est le dernier, il le reste"), "{err}");
    assert_eq!(
        h.core
            .settings
            .read()
            .accounts
            .iter()
            .find(|a| a.id == "pro")
            .unwrap()
            .projects,
        std::slice::from_ref(&p.id)
    );
    // Turned to `denied`: the reservations go, a private project goes on the shared accounts again.
    h.core
        .set_managed(managed(&[(ACCOUNT_RESERVATION, json!("denied"))]));
    assert!(h
        .core
        .settings
        .read()
        .accounts
        .iter()
        .all(|a| a.projects.is_empty()));
    let err = h
        .core
        .update_claude_account(reserved)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("interdit par la stratégie de ce poste"),
        "{err}"
    );
    assert!(h.core.create_agent(&p.id, None).await.is_ok());
}

#[tokio::test]
async fn an_agent_of_a_private_project_on_a_shared_account_stops_between_two_turns_when_the_policy_comes(
) {
    let h = harness("locks-reserve-live");
    let (p, _) = h.project(false).await;
    let mut project = h.core.project(&p.id).unwrap();
    project.private = true;
    h.core.update_project(project).unwrap();
    crate::harness_scope_tests::turn_on_every_harness(&h, &p.id);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    assert!(
        h.core.agent(&a).unwrap().lock().proc.is_some(),
        "its process kept"
    );
    h.core
        .set_managed(managed(&[(ACCOUNT_RESERVATION, json!("required"))]));
    h.wait("the agent to say it stops", |h| {
        h.items(&a).iter().any(|i| i["kind"] == "notice")
    })
    .await;
    assert!(h.core.agent(&a).unwrap().lock().proc.is_none());
    let said = h.items(&a);
    assert!(
        said.iter().any(|i| i["kind"] == "notice"
            && i["text"].as_str().unwrap_or_default().contains(
                "est partagé avec d’autres projets. L’agent s’arrête à la fin de son tour."
            )),
        "{said:?}"
    );
    // Its next message is refused, with the same reason.
    let err = h
        .core
        .send_message(&a, "encore".into(), vec![])
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("accounts.reservation"), "{err}");
}
