//! Tests of the language servers (ESC-446): the catalog, where they are found, what the settings and the project's
//! file say, the confirmation of a project's own server, the bounds of the transport, and the manager on a false
//! server (`tests/fixtures/fake-lsp.mjs`): shared, stopped after a delay, restarted a bounded number of times, in a
//! Job.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::io::BufReader;

use super::catalog::{self, SERVERS};
use super::locate::{self, Found, NotFound, Origin, Places};
use super::manager::{Info, Launch, Manager, Policy, Sink, State};
use super::transport::{frame, read_frame, FrameError, MAX_MESSAGE};
use super::{config, launch_for, server_named, status, Refused, TrustReason};
use crate::model::LanguageServerSetting;

fn tmp(name: &str) -> PathBuf {
    crate::paths::test_dir(&format!("lsp-{name}"))
}

/// A program `name` in `dir`, as the system names one (`name.cmd` on Windows).
fn exe(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(if cfg!(windows) {
        format!("{name}.cmd")
    } else {
        name.to_string()
    });
    std::fs::write(&p, "").unwrap();
    p
}

fn no_tool(_: &str, _: &[&str]) -> Option<String> {
    None
}

fn places<'a>(
    path: Vec<PathBuf>,
    roots: Vec<PathBuf>,
    home: Option<PathBuf>,
    run: &'a (dyn Fn(&str, &[&str]) -> Option<String> + Sync),
) -> Places<'a> {
    Places {
        path,
        roots,
        home,
        appdata: None,
        run,
    }
}

fn server(id: &str) -> &'static catalog::Server {
    catalog::get(id).unwrap()
}

// ---------- the catalog ----------

#[test]
fn the_catalog_serves_the_languages_asked() {
    for (language, id, program) in [
        ("typescript", "typescript", "typescript-language-server"),
        (
            "typescriptreact",
            "typescript",
            "typescript-language-server",
        ),
        ("javascript", "typescript", "typescript-language-server"),
        (
            "javascriptreact",
            "typescript",
            "typescript-language-server",
        ),
        ("rust", "rust", "rust-analyzer"),
        ("python", "python", "basedpyright-langserver"),
        ("go", "go", "gopls"),
        ("json", "json", "vscode-json-language-server"),
        ("jsonc", "json", "vscode-json-language-server"),
        ("css", "css", "vscode-css-language-server"),
        ("html", "html", "vscode-html-language-server"),
    ] {
        let s = catalog::for_language(language).unwrap_or_else(|| panic!("{language}"));
        assert_eq!((s.id, s.programs[0]), (id, program), "{language}");
        assert!(!s.install.is_empty(), "{id}");
        assert_eq!(server_named(language).unwrap().id, id);
        assert_eq!(server_named(id).unwrap().id, id);
    }
    // pyright when basedpyright is not there; the stdio servers say so.
    assert_eq!(server("python").programs[1], "pyright-langserver");
    for id in ["typescript", "python", "json", "css", "html"] {
        assert_eq!(server(id).args, &["--stdio"], "{id}");
        assert!(server(id).node, "{id} is an npm package");
    }
    assert!(server("rust").args.is_empty() && !server("rust").node);
    assert_eq!(server("rust").install, "rustup component add rust-analyzer");
    assert_eq!(
        server("go").install,
        "go install golang.org/x/tools/gopls@latest"
    );
    // One id each, one server by language.
    let ids: std::collections::HashSet<_> = SERVERS.iter().map(|s| s.id).collect();
    assert_eq!(ids.len(), SERVERS.len());
    let langs: Vec<_> = SERVERS.iter().flat_map(|s| s.languages).collect();
    let unique: std::collections::HashSet<_> = langs.iter().collect();
    assert_eq!(unique.len(), langs.len());
    // A name of the page is a server's or a language's, never a program.
    assert_eq!(
        server_named("sh"),
        Err(Refused::Unknown { name: "sh".into() })
    );
}

// ---------- where they are found ----------

#[test]
fn a_server_is_found_on_the_path() {
    let dir = tmp("on-path");
    let bin = dir.join("bin");
    let tsls = exe(&bin, "typescript-language-server");
    let p = places(vec![dir.join("empty"), bin.clone()], vec![], None, &no_tool);
    assert_eq!(
        locate::find(server("typescript"), "", None, &p),
        Ok(Found {
            program: tsls,
            origin: Origin::Path
        })
    );
}

#[test]
fn the_node_modules_of_the_worktree_then_of_the_project_come_before_the_path() {
    let dir = tmp("node-modules");
    let (wt, project) = (dir.join("wt"), dir.join("project"));
    let bin = dir.join("bin");
    exe(&bin, "typescript-language-server");
    exe(&bin, "rust-analyzer");
    let in_project = exe(
        &project.join("node_modules").join(".bin"),
        "typescript-language-server",
    );
    // An npm server: the project's own.
    let p = places(
        vec![bin.clone()],
        vec![wt.clone(), project.clone()],
        None,
        &no_tool,
    );
    let found = locate::find(server("typescript"), "", None, &p).unwrap();
    assert_eq!(
        (found.program, found.origin),
        (in_project, Origin::NodeModules)
    );
    assert!(Origin::NodeModules.of_project());
    // The worktree's own goes first.
    let in_wt = exe(
        &wt.join("node_modules").join(".bin"),
        "typescript-language-server",
    );
    let found = locate::find(server("typescript"), "", None, &p).unwrap();
    assert_eq!(found.program, in_wt);
    // rust-analyzer is no npm package: a node_modules is not looked in.
    exe(&project.join("node_modules").join(".bin"), "rust-analyzer");
    assert_eq!(
        locate::find(server("rust"), "", None, &p).unwrap().origin,
        Origin::Path
    );
}

#[test]
fn a_path_set_by_hand_wins_and_is_never_replaced() {
    let dir = tmp("manual");
    let bin = dir.join("bin");
    let root = dir.join("project");
    exe(&bin, "rust-analyzer");
    exe(
        &root.join("node_modules").join(".bin"),
        "typescript-language-server",
    );
    let mine = exe(&dir.join("mine"), "my-ra");
    let p = places(vec![bin.clone()], vec![root], None, &no_tool);
    let manual = mine.display().to_string();
    // Over the PATH, over a node_modules, over the project's file.
    for id in ["rust", "typescript"] {
        assert_eq!(
            locate::find(server(id), &manual, Some("node_modules/.bin/x"), &p),
            Ok(Found {
                program: mine.clone(),
                origin: Origin::Manual
            }),
            "{id}"
        );
    }
    assert!(!Origin::Manual.of_project());
    // A name alone is a program of the PATH.
    let other = exe(&bin, "ra-nightly");
    assert_eq!(
        locate::find(server("rust"), " ra-nightly ", None, &p)
            .unwrap()
            .program,
        other
    );
    // Wrong, it is said, and no other is taken in its place.
    let gone = dir
        .join("nowhere")
        .join("rust-analyzer")
        .display()
        .to_string();
    assert_eq!(
        locate::find(server("rust"), &gone, None, &p),
        Err(NotFound::Manual { path: gone.clone() })
    );
    assert_eq!(
        locate::find(server("rust"), "not-there", None, &p),
        Err(NotFound::Manual {
            path: "not-there".into()
        })
    );
}

#[test]
fn the_standard_places_of_each_tool() {
    let dir = tmp("standard");
    let home = dir.join("home");
    // rustup's toolchain, when rust-analyzer is not on the PATH.
    let toolchain = exe(&dir.join("toolchain").join("bin"), "rust-analyzer");
    let asked = Mutex::new(Vec::new());
    let tc = toolchain.display().to_string();
    let gopath = dir.join("gopath");
    let gp = gopath.display().to_string();
    let run = |program: &str, args: &[&str]| -> Option<String> {
        asked.lock().push(format!("{program} {}", args.join(" ")));
        match (program, args) {
            ("rustup", ["which", "rust-analyzer"]) => Some(tc.clone()),
            ("go", ["env", "GOPATH"]) => Some(gp.clone()),
            _ => None,
        }
    };
    let p = places(vec![], vec![], Some(home.clone()), &run);
    assert_eq!(
        locate::find(server("rust"), "", None, &p),
        Ok(Found {
            program: toolchain.clone(),
            origin: Origin::Standard
        })
    );
    assert_eq!(asked.lock()[0], "rustup which rust-analyzer");
    // ~/.cargo/bin when rustup has none.
    let cargo = exe(&home.join(".cargo").join("bin"), "rust-analyzer");
    std::fs::remove_file(&toolchain).unwrap();
    assert_eq!(
        locate::find(server("rust"), "", None, &p).unwrap().program,
        cargo
    );
    // gopls in $(go env GOPATH)/bin, then in ~/go/bin when go says nothing.
    let gopls = exe(&gopath.join("bin"), "gopls");
    assert_eq!(
        locate::find(server("go"), "", None, &p).unwrap().program,
        gopls
    );
    let home_go = exe(&home.join("go").join("bin"), "gopls");
    let p2 = places(vec![], vec![], Some(home.clone()), &no_tool);
    assert_eq!(
        locate::find(server("go"), "", None, &p2).unwrap().program,
        home_go
    );
    // basedpyright of pip's user scripts goes before pyright on the PATH: each program everywhere before the next.
    let bin = dir.join("bin");
    exe(&bin, "pyright-langserver");
    let based = exe(&home.join(".local").join("bin"), "basedpyright-langserver");
    let p3 = places(vec![bin.clone()], vec![], Some(home.clone()), &no_tool);
    assert_eq!(
        locate::find(server("python"), "", None, &p3),
        Ok(Found {
            program: based.clone(),
            origin: Origin::Standard
        })
    );
    std::fs::remove_file(&based).unwrap();
    assert_eq!(
        locate::find(server("python"), "", None, &p3)
            .unwrap()
            .origin,
        Origin::Path
    );
}

#[test]
fn a_server_absent_is_said_with_the_command_that_installs_it() {
    let dir = tmp("absent");
    let p = places(vec![dir.clone()], vec![dir.clone()], Some(dir), &no_tool);
    for s in SERVERS {
        assert_eq!(
            locate::find(s, "", None, &p),
            Err(NotFound::Absent {
                install: s.install.to_string()
            }),
            "{}",
            s.id
        );
    }
    let r = launch_for(
        server("go"),
        &config(server("go"), &BTreeMap::new(), None),
        &p,
        true,
        PathBuf::new(),
    )
    .unwrap_err();
    assert!(
        r.to_string()
            .contains("go install golang.org/x/tools/gopls@latest"),
        "{r}"
    );
}

#[test]
fn the_path_of_the_project_file_is_a_file_of_the_repository() {
    let dir = tmp("file-path");
    let root = dir.join("project");
    let own = exe(&root.join("tools"), "my-tsls");
    let p = places(vec![], vec![root.clone()], None, &no_tool);
    // With or without its ending.
    for path in [
        "tools/my-tsls",
        &format!("tools/{}", own.file_name().unwrap().to_str().unwrap()),
    ] {
        assert_eq!(
            locate::find(server("typescript"), "", Some(path), &p),
            Ok(Found {
                program: own.clone(),
                origin: Origin::ProjectFile
            }),
            "{path}"
        );
    }
    // Never out of it.
    let outside = exe(&dir, "evil");
    for path in [
        outside.display().to_string(),
        "../evil".to_string(),
        "tools/../../evil".to_string(),
        "tools/none".to_string(),
    ] {
        assert_eq!(
            locate::find(server("typescript"), "", Some(&path), &p),
            Err(NotFound::ProjectFile { path: path.clone() }),
            "{path}"
        );
    }
}

// ---------- the settings, the project's file, the confirmation ----------

#[test]
fn the_project_file_goes_over_the_app_settings_but_for_the_path_set_by_hand() {
    let ts = server("typescript");
    // Nothing set: the catalog's.
    let c = config(ts, &BTreeMap::new(), None);
    assert!(c.enabled && c.enabled_by == "default" && !c.file_args);
    assert_eq!(c.args, vec!["--stdio"]);
    // The app's settings.
    let mut app = BTreeMap::new();
    app.insert(
        "typescript".to_string(),
        LanguageServerSetting {
            enabled: Some(false),
            path: " /opt/tsls ".into(),
            args: Some(vec!["--stdio".into(), "--log-level=4".into()]),
        },
    );
    let c = config(ts, &app, None);
    assert_eq!((c.enabled, c.enabled_by), (false, "app"));
    assert_eq!(c.manual, "/opt/tsls");
    assert_eq!(c.args, vec!["--stdio", "--log-level=4"]);
    assert!(!c.file_args);
    // The project's file over them.
    let file = json!({ "enabled": true, "args": ["--stdio", "--strict"], "path": "bin/tsls" });
    let c = config(ts, &app, file.as_object());
    assert_eq!((c.enabled, c.enabled_by), (true, "project"));
    assert_eq!(c.args, vec!["--stdio", "--strict"]);
    assert!(c.file_args);
    assert_eq!(c.file_path.as_deref(), Some("bin/tsls"));
    // …but the path set by hand still wins.
    assert_eq!(c.manual, "/opt/tsls");
    let off = json!({ "enabled": false });
    assert_eq!(
        config(ts, &BTreeMap::new(), off.as_object()).enabled_by,
        "project"
    );
}

#[test]
fn a_server_of_the_project_runs_only_once_the_project_is_trusted() {
    // JSON's server runs no code of the project: what is asked is what the repository chose.
    let dir = tmp("trust");
    let root = dir.join("project");
    let bin = dir.join("bin");
    let name = "vscode-json-language-server";
    let local = exe(&root.join("node_modules").join(".bin"), name);
    let p = places(vec![bin.clone()], vec![root.clone()], None, &no_tool);
    let js = server("json");
    let none = BTreeMap::new();
    // From the project's node_modules: asked first.
    let refused = launch_for(js, &config(js, &none, None), &p, false, root.clone()).unwrap_err();
    assert_eq!(
        refused,
        Refused::NeedsTrust {
            server: "json",
            program: local.display().to_string(),
            args: vec!["--stdio".into()],
            reason: TrustReason::Chosen,
        }
    );
    assert!(refused.to_string().contains(&local.display().to_string()));
    let s = status(js, &config(js, &none, None), &p, false);
    assert!(s.of_project && s.needs_trust && !s.runs_project_code && !s.trusted);
    // Trusted: it runs, known as the project's.
    let l = launch_for(js, &config(js, &none, None), &p, true, root.clone()).unwrap();
    assert_eq!((l.program, l.of_project), (local.clone(), true));
    assert_eq!(l.cwd, root);
    // A program of the PATH with arguments of the repository: those are the repository's choice too.
    std::fs::remove_file(&local).unwrap();
    let global = exe(&bin, name);
    let file = json!({ "args": ["--stdio", "--evil"] });
    let c = config(js, &none, file.as_object());
    assert!(matches!(
        launch_for(js, &c, &p, false, root.clone()),
        Err(Refused::NeedsTrust {
            reason: TrustReason::Chosen,
            ..
        })
    ));
    // The path of the project's file: a program of the repository.
    let own = exe(&root.join("tools"), "jsonls");
    let file = json!({ "path": "tools/jsonls" });
    let c = config(js, &none, file.as_object());
    assert!(matches!(
        launch_for(js, &c, &p, false, root.clone()),
        Err(Refused::NeedsTrust { program, .. }) if program == own.display().to_string()
    ));
    // The machine's own, with the catalog's arguments: nothing to ask.
    let l = launch_for(js, &config(js, &none, None), &p, false, root.clone()).unwrap();
    assert_eq!((l.program, l.of_project), (global.clone(), false));
    let s = status(js, &config(js, &none, None), &p, false);
    assert!(!s.of_project && !s.needs_trust);
    // Nor one set by hand, even in the project's folder: the user chose it.
    let mut app = BTreeMap::new();
    app.insert(
        "json".to_string(),
        LanguageServerSetting {
            path: own.display().to_string(),
            ..Default::default()
        },
    );
    let l = launch_for(js, &config(js, &app, None), &p, false, root.clone()).unwrap();
    assert_eq!((l.program, l.of_project), (own, false));
    // Off: said, by whom.
    let off = json!({ "enabled": false });
    assert_eq!(
        launch_for(js, &config(js, &none, off.as_object()), &p, true, root),
        Err(Refused::Disabled { by: "project" })
    );
}

#[test]
fn a_server_that_runs_code_of_the_project_waits_for_its_trust_even_from_the_machine() {
    let dir = tmp("runs-code");
    let root = dir.join("project");
    let bin = dir.join("bin");
    let ra = exe(&bin, "rust-analyzer");
    exe(&bin, "typescript-language-server");
    exe(&bin, "gopls");
    let pyright = exe(&bin, "pyright-langserver");
    for n in [
        "vscode-json-language-server",
        "vscode-css-language-server",
        "vscode-html-language-server",
    ] {
        exe(&bin, n);
    }
    let p = places(vec![bin.clone()], vec![root.clone()], None, &no_tool);
    let none = BTreeMap::new();
    // The marks of the catalog: rust-analyzer, typescript-language-server and gopls; not the others.
    let marked: Vec<&str> = SERVERS
        .iter()
        .filter(|s| s.runs_project_code)
        .map(|s| s.id)
        .collect();
    assert_eq!(marked, vec!["typescript", "rust", "go"]);
    // rust-analyzer of the PATH: refused without the trust, said why.
    let rust = server("rust");
    let refused =
        launch_for(rust, &config(rust, &none, None), &p, false, root.clone()).unwrap_err();
    assert_eq!(
        refused,
        Refused::NeedsTrust {
            server: "rust",
            program: ra.display().to_string(),
            args: vec![],
            reason: TrustReason::RunsProjectCode,
        }
    );
    assert_eq!(
        refused.to_string(),
        "ce serveur exécute du code du projet (build.rs, macros) : confirme d’abord que tu fais confiance au projet"
    );
    let s = status(rust, &config(rust, &none, None), &p, false);
    assert!(s.runs_project_code && s.needs_trust && !s.of_project);
    // With it: started, and stopped when the trust goes.
    let l = launch_for(rust, &config(rust, &none, None), &p, true, root.clone()).unwrap();
    assert_eq!((l.program, l.of_project), (ra.clone(), true));
    // Set by hand too: the program is the user's, the code it runs is the project's.
    let mut app = BTreeMap::new();
    app.insert(
        "rust".to_string(),
        LanguageServerSetting {
            path: ra.display().to_string(),
            ..Default::default()
        },
    );
    assert!(matches!(
        launch_for(rust, &config(rust, &app, None), &p, false, root.clone()),
        Err(Refused::NeedsTrust {
            reason: TrustReason::RunsProjectCode,
            ..
        })
    ));
    for id in ["typescript", "go"] {
        let s = server(id);
        let r = launch_for(s, &config(s, &none, None), &p, false, root.clone()).unwrap_err();
        assert!(r.to_string().contains(&super::runs_what(id)), "{id}: {r}");
    }
    // pyright of the PATH, JSON, CSS and HTML: free.
    let py = server("python");
    let l = launch_for(py, &config(py, &none, None), &p, false, root.clone()).unwrap();
    assert_eq!((l.program, l.of_project), (pyright, false));
    for id in ["json", "css", "html"] {
        let s = server(id);
        assert!(
            launch_for(s, &config(s, &none, None), &p, false, root.clone()).is_ok(),
            "{id}"
        );
        assert!(
            !status(s, &config(s, &none, None), &p, false).needs_trust,
            "{id}"
        );
    }
}

#[test]
fn a_server_is_started_without_the_current_folder_of_cmd() {
    let l = Launch {
        program: PathBuf::from("x"),
        args: vec![],
        cwd: PathBuf::from("."),
        of_project: false,
    };
    let cmd = super::manager::command(&l);
    let set: Option<String> = cmd
        .as_std()
        .get_envs()
        .find(|(k, _)| *k == "NoDefaultCurrentDirectoryInExePath")
        .and_then(|(_, v)| v.map(|v| v.to_string_lossy().into_owned()));
    if cfg!(windows) {
        assert_eq!(set.as_deref(), Some("1"));
    } else {
        assert_eq!(set, None);
    }
}

/// Under Windows, a server found on the PATH as an npm shim calls `node` by its name: a `node.cmd` at the root of the
/// worktree (its current folder) does not run in its place.
#[cfg(windows)]
#[tokio::test]
async fn a_shim_of_the_path_does_not_run_the_node_of_the_worktree() {
    // A machine may have the variable already (inherited by the tests): gone, only the core's own protects.
    // Edition 2021: safe; nextest runs this test alone in its process, before it starts anything.
    std::env::remove_var("NoDefaultCurrentDirectoryInExePath");
    let dir = tmp("shim");
    let root = dir.join("repo");
    std::fs::create_dir_all(&root).unwrap();
    let marker = dir.join("planted-ran.txt");
    std::fs::write(
        root.join("node.cmd"),
        format!("@echo planted> \"{}\"\r\n@exit /b 9\r\n", marker.display()),
    )
    .unwrap();
    // The shim npm writes: `node` by its name, found on the PATH (where the tests' node is).
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("fake-lsp.mjs");
    let shim = dir.join("bin").join("fake-ls.cmd");
    std::fs::create_dir_all(shim.parent().unwrap()).unwrap();
    std::fs::write(
        &shim,
        format!("@echo off\r\nnode \"{}\" %*\r\n", script.display()),
    )
    .unwrap();
    node();
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let i = m.acquire(
        "v",
        "p",
        "typescript",
        &root.display().to_string(),
        Launch {
            program: shim,
            args: vec![],
            cwd: root.clone(),
            of_project: false,
        },
    );
    until("running", 20, || {
        m.info(i.id).unwrap().state == State::Running
    })
    .await;
    m.send(i.id, &req(1, "initialize", json!({})))
        .await
        .unwrap();
    until("initialize, or the planted node", 20, || {
        rec.answer(i.id, 1).is_some() || marker.exists()
    })
    .await;
    assert!(!marker.exists(), "the node.cmd of the worktree ran");
    m.shutdown();
}

#[test]
fn the_project_file_holds_the_language_servers() {
    use crate::projectfile as pf;
    let r = pf::read(
        "version: 1\n\
         language_servers:\n  \
           typescript:\n    \
             enabled: false\n    \
             path: node_modules/.bin/typescript-language-server\n    \
             args: [--stdio, --log-level, \"4\"]\n  \
           rust:\n    \
             args: --no-log\n  \
           python:\n    \
             path: /usr/bin/pyright\n    \
             colour: red\n  \
           cobol:\n    \
             enabled: true\n",
    )
    .unwrap();
    assert_eq!(
        r.values["language_servers.typescript"],
        json!({ "enabled": false, "path": "node_modules/.bin/typescript-language-server", "args": ["--stdio", "--log-level", "4"] })
    );
    // A single text is a list of one.
    assert_eq!(
        r.values["language_servers.rust"],
        json!({ "args": ["--no-log"] })
    );
    // An absolute path, an unknown field, an unknown server: told, left out.
    assert!(!r.values.contains_key("language_servers.python"));
    assert!(!r.values.contains_key("language_servers.cobol"));
    let told: Vec<&str> = r.warnings.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(told.len(), 3, "{told:?}");
    assert!(
        told.iter().any(|w| w.contains("/usr/bin/pyright")),
        "{told:?}"
    );
    assert!(told.iter().any(|w| w.contains("colour")), "{told:?}");
    assert!(told.iter().any(|w| w.contains("cobol")), "{told:?}");
    // The file's alone: no field of the project holds it, and the servers read it there.
    assert!(pf::file_only("language_servers.typescript"));
    assert_eq!(
        super::file_settings(&r.values, "rust").unwrap()["args"],
        json!(["--no-log"])
    );
    let mut p: crate::model::Project =
        serde_json::from_value(json!({ "id": "p", "name": "p", "path": "/p", "color": "" }))
            .unwrap();
    let applied = pf::apply(&mut p, &r);
    assert!(applied.changed.is_empty());
    let file = p.shared.file.unwrap();
    assert!(file.pending.is_none());
    assert_eq!(
        file.values["language_servers.typescript"]["enabled"],
        json!(false)
    );
    // The schema knows each server.
    let schema = pf::schema();
    let group = &schema["properties"]["language_servers"];
    for s in SERVERS {
        assert!(
            group["properties"][s.id]["properties"]["args"].is_object(),
            "{}",
            s.id
        );
    }
}

// ---------- the transport ----------

#[tokio::test]
async fn the_transport_reads_framed_messages_one_after_the_other() {
    let mut bytes = frame(r#"{"a":1}"#).unwrap();
    bytes.extend(b"\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: 4\r\n\r\n\"\xc3\xa9\"");
    let mut r = BufReader::new(&bytes[..]);
    assert_eq!(
        read_frame(&mut r).await.unwrap().as_deref(),
        Some(r#"{"a":1}"#)
    );
    assert_eq!(read_frame(&mut r).await.unwrap().as_deref(), Some("\"é\""));
    assert_eq!(read_frame(&mut r).await.unwrap(), None);
    // By bytes, not characters.
    assert_eq!(
        frame("é").unwrap(),
        b"Content-Length: 2\r\n\r\n\xc3\xa9".to_vec()
    );
}

#[tokio::test]
async fn the_transport_refuses_what_is_too_big_or_malformed() {
    async fn read(bytes: &[u8]) -> Result<Option<String>, FrameError> {
        read_frame(&mut BufReader::new(bytes)).await
    }
    // A length announced too big: refused before anything is read or allocated.
    let huge = format!("Content-Length: {}\r\n\r\n{{", MAX_MESSAGE + 1);
    assert_eq!(
        read(huge.as_bytes()).await,
        Err(FrameError::TooBig(MAX_MESSAGE + 1))
    );
    assert!(matches!(
        read(b"Content-Length: 99999999999999999999999\r\n\r\n").await,
        Err(FrameError::BadHeader(_))
    ));
    // A header line without end, too many of them, none that gives the length.
    let long = format!("X-{}: 1\r\n", "a".repeat(4096));
    assert!(matches!(
        read(long.as_bytes()).await,
        Err(FrameError::BadHeader(_))
    ));
    let many = "X: 1\r\n".repeat(17) + "Content-Length: 2\r\n\r\n{}";
    assert!(matches!(
        read(many.as_bytes()).await,
        Err(FrameError::BadHeader(_))
    ));
    assert!(matches!(
        read(b"X: 1\r\n\r\n{}").await,
        Err(FrameError::BadHeader(_))
    ));
    assert!(matches!(
        read(b"nonsense\r\n\r\n").await,
        Err(FrameError::BadHeader(_))
    ));
    // Cut in a message, not UTF-8.
    assert_eq!(
        read(b"Content-Length: 10\r\n\r\n{}").await,
        Err(FrameError::Cut)
    );
    assert_eq!(read(b"Content-Length: 10\r\n").await, Err(FrameError::Cut));
    assert_eq!(
        read(b"Content-Length: 1\r\n\r\n\xff").await,
        Err(FrameError::NotText)
    );
    // What the page sends is bounded too.
    let big = "x".repeat(MAX_MESSAGE + 1);
    assert_eq!(frame(&big), Err(FrameError::TooBig(MAX_MESSAGE + 1)));
}

// ---------- the manager, on a false server ----------

/// What the manager told.
#[derive(Default)]
struct Rec {
    messages: Mutex<Vec<(u32, Value)>>,
    states: Mutex<Vec<Info>>,
}

impl Sink for Rec {
    fn message(&self, id: u32, msg: &str) {
        self.messages
            .lock()
            .push((id, serde_json::from_str(msg).unwrap()));
    }
    fn state(&self, info: &Info) {
        self.states.lock().push(info.clone());
    }
}

impl Rec {
    /// The answer of the server `id` to the request `req`.
    fn answer(&self, id: u32, req: u64) -> Option<Value> {
        self.messages
            .lock()
            .iter()
            .find(|(i, m)| *i == id && m["id"] == json!(req))
            .map(|(_, m)| m.clone())
    }
    fn states_of(&self, id: u32) -> Vec<State> {
        self.states
            .lock()
            .iter()
            .filter(|i| i.id == id)
            .map(|i| i.state)
            .collect()
    }
}

async fn until(what: &str, secs: u64, ok: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    while !ok() {
        assert!(Instant::now() < end, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn node() -> PathBuf {
    crate::paths::test_dir("lsp-path-trim");
    let path: Vec<PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect();
    path.iter()
        .find_map(|d| locate::program_in(d, "node"))
        .expect("node on the PATH")
}

fn fake(cwd: &Path, args: &[&str]) -> Launch {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("fake-lsp.mjs");
    let mut all = vec![script.display().to_string()];
    all.extend(args.iter().map(|a| a.to_string()));
    Launch {
        program: node(),
        args: all,
        cwd: cwd.to_path_buf(),
        of_project: false,
    }
}

fn quick() -> Policy {
    Policy {
        max_restarts: 2,
        first_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(2),
        stable_after: Duration::from_secs(60),
        idle_stop: Duration::from_millis(400),
    }
}

fn req(id: u64, method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

fn starts(count: &Path) -> usize {
    std::fs::read_to_string(count)
        .map(|t| t.lines().count())
        .unwrap_or(0)
}

#[test]
fn the_delay_before_a_restart_doubles_up_to_its_bound() {
    let p = quick();
    let delays: Vec<u128> = (1..=6).map(|n| p.delay(n).as_millis()).collect();
    assert_eq!(delays, vec![100, 200, 400, 800, 1600, 2000]);
    assert_eq!(p.delay(200), Duration::from_secs(2));
    let d = Policy::default();
    assert_eq!(
        (d.max_restarts, d.delay(1), d.idle_stop),
        (5, Duration::from_secs(1), Duration::from_secs(60))
    );
}

#[tokio::test]
async fn one_server_by_root_shared_by_the_views_and_stopped_after_the_last_one_a_while_later() {
    let dir = tmp("shared");
    let count = dir.join("starts.txt");
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let launch = fake(&dir, &["--count", &count.display().to_string()]);
    let a = m.acquire("view-a", "p", "typescript", "/root", launch.clone());
    until("running", 20, || {
        m.info(a.id).unwrap().state == State::Running
    })
    .await;
    m.send(a.id, &req(1, "initialize", json!({})))
        .await
        .unwrap();
    until("initialize", 20, || rec.answer(a.id, 1).is_some()).await;
    assert_eq!(
        rec.answer(a.id, 1).unwrap()["result"]["serverInfo"]["name"],
        "fake-lsp"
    );
    // A second view of the same root and server: the same instance, no other process.
    let b = m.acquire("view-b", "p", "typescript", "/root", launch.clone());
    assert_eq!(b.id, a.id);
    // Another root: another one.
    let other = m.acquire("view-b", "p", "typescript", "/other", launch.clone());
    assert_ne!(other.id, a.id);
    m.send(a.id, &req(2, "$/echo", json!({ "text": "é" })))
        .await
        .unwrap();
    until("echo", 20, || rec.answer(a.id, 2).is_some()).await;
    assert_eq!(rec.answer(a.id, 2).unwrap()["result"]["text"], "é");
    until("two processes", 20, || starts(&count) == 2).await;
    // The first view lets go: the second still holds it.
    m.release("view-a", a.id);
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(m.info(a.id).unwrap().state, State::Running);
    // The last one lets go, then comes back before the delay: it stays.
    m.release("view-b", a.id);
    tokio::time::sleep(Duration::from_millis(100)).await;
    m.acquire("view-c", "p", "typescript", "/root", launch.clone());
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(m.info(a.id).unwrap().state, State::Running);
    // A view gone (its page reloaded) lets go of all it held: stopped after the delay, not before.
    m.release_view("view-c");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(m.info(a.id).is_ok());
    until("stopped", 20, || m.info(a.id).is_err()).await;
    assert_eq!(rec.states_of(a.id).last(), Some(&State::Stopped));
    assert!(m.send(a.id, "{}").await.is_err());
    let log = m.log(other.id).unwrap();
    assert!(
        log.iter()
            .any(|l| l.source == "stderr" && l.text.contains("fake-lsp started")),
        "{log:?}"
    );
    m.shutdown();
    assert!(m.list().is_empty());
    assert_eq!(starts(&count), 2, "never started again");
}

#[tokio::test]
async fn a_crashed_server_is_restarted_with_a_growing_delay_a_bounded_number_of_times() {
    let dir = tmp("restart");
    let count = dir.join("starts.txt");
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let i = m.acquire(
        "v",
        "p",
        "rust",
        "/r",
        fake(&dir, &["--count", &count.display().to_string()]),
    );
    until("running", 20, || {
        m.info(i.id).unwrap().state == State::Running
    })
    .await;
    // It crashes: started again, its run counted, said.
    m.send(i.id, &req(1, "$/crash", json!({}))).await.unwrap();
    until("second run", 20, || {
        let info = m.info(i.id).unwrap();
        info.state == State::Running && info.run == 2
    })
    .await;
    let restarting: Vec<Info> = rec
        .states
        .lock()
        .iter()
        .filter(|s| s.state == State::Restarting)
        .cloned()
        .collect();
    assert_eq!(restarting.len(), 1);
    assert_eq!(restarting[0].retry_in_ms, Some(100));
    assert_eq!(restarting[0].error.as_deref(), Some("exit code 3"));
    // Again: the delay doubled.
    m.send(i.id, &req(2, "$/crash", json!({}))).await.unwrap();
    until("third run", 20, || m.info(i.id).unwrap().run == 3).await;
    let delays: Vec<Option<u64>> = rec
        .states
        .lock()
        .iter()
        .filter(|s| s.state == State::Restarting)
        .map(|s| s.retry_in_ms)
        .collect();
    assert_eq!(delays, vec![Some(100), Some(200)]);
    // Once more than allowed: it fails, said, and is no longer started.
    m.send(i.id, &req(3, "$/crash", json!({}))).await.unwrap();
    until("failed", 20, || {
        m.info(i.id).unwrap().state == State::Failed
    })
    .await;
    let failed = m.info(i.id).unwrap();
    assert_eq!(
        (failed.restarts, failed.error.as_deref()),
        (2, Some("exit code 3"))
    );
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(starts(&count), 3);
    assert!(m.send(i.id, "{}").await.is_err());
    let log = m.log(i.id).unwrap();
    let core: Vec<&str> = log
        .iter()
        .filter(|l| l.source == "core")
        .map(|l| l.text.as_str())
        .collect();
    assert!(
        core.iter().any(|l| l.starts_with("restart 1 in 100 ms")),
        "{core:?}"
    );
    assert!(core.iter().any(|l| l.starts_with("failed")), "{core:?}");
    // Asked again: it starts, its crashes counted from zero.
    m.restart(i.id).unwrap();
    until("restarted", 20, || {
        m.info(i.id).unwrap().state == State::Running
    })
    .await;
    assert_eq!(m.info(i.id).unwrap().restarts, 0);
    until("fourth start", 20, || starts(&count) == 4).await;
    // A restart asked of one running is no crash.
    m.restart(i.id).unwrap();
    until("fifth run", 20, || m.info(i.id).unwrap().run == 5).await;
    assert_eq!(m.info(i.id).unwrap().restarts, 0);
    m.shutdown();
}

#[tokio::test]
async fn a_server_that_never_starts_fails_after_its_restarts() {
    let dir = tmp("crash-at-start");
    let count = dir.join("starts.txt");
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let i = m.acquire(
        "v",
        "p",
        "go",
        "/g",
        fake(
            &dir,
            &["--crash-at-start", "--count", &count.display().to_string()],
        ),
    );
    until("failed", 20, || {
        m.info(i.id).unwrap().state == State::Failed
    })
    .await;
    assert_eq!(starts(&count), 3, "the first start and its two restarts");
    let log = m.log(i.id).unwrap();
    assert!(
        log.iter()
            .any(|l| l.source == "stderr" && l.text == "fake-lsp: crashing at start"),
        "{log:?}"
    );
    // A program that does not start is a crash too.
    let gone = m.acquire(
        "v",
        "p",
        "rust",
        "/g",
        Launch {
            program: dir.join("nothing-here"),
            args: vec![],
            cwd: dir.clone(),
            of_project: false,
        },
    );
    until("failed too", 20, || {
        m.info(gone.id).unwrap().state == State::Failed
    })
    .await;
    assert!(m
        .info(gone.id)
        .unwrap()
        .error
        .unwrap()
        .contains("could not start"));
    m.shutdown();
}

#[tokio::test]
async fn a_stream_broken_by_the_server_ends_it() {
    let dir = tmp("huge");
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let i = m.acquire("v", "p", "json", "/j", fake(&dir, &[]));
    until("running", 20, || {
        m.info(i.id).unwrap().state == State::Running
    })
    .await;
    m.send(i.id, &req(1, "$/huge", json!({}))).await.unwrap();
    until("restarting", 20, || {
        rec.states_of(i.id).contains(&State::Restarting)
    })
    .await;
    let log = m.log(i.id).unwrap();
    assert!(
        log.iter().any(|l| l.source == "core"
            && l.text.contains("stream refused")
            && l.text.contains("1000000000000 bytes announced")),
        "{log:?}"
    );
    // And it runs again, read from the start of a new stream.
    until("second run", 20, || {
        m.info(i.id).unwrap().run == 2 && m.info(i.id).unwrap().state == State::Running
    })
    .await;
    m.send(i.id, &req(2, "$/echo", json!({ "ok": true })))
        .await
        .unwrap();
    until("echo", 20, || rec.answer(i.id, 2).is_some()).await;
    m.shutdown();
}

/// Whether the process `pid` is running.
fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
    }
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

#[tokio::test]
async fn a_server_runs_in_a_job_that_ends_what_it_started_with_it() {
    let dir = tmp("job");
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let i = m.acquire("v", "p", "css", "/c", fake(&dir, &[]));
    until("running", 20, || {
        m.info(i.id).unwrap().state == State::Running
    })
    .await;
    m.send(i.id, &req(1, "$/child", json!({}))).await.unwrap();
    until("child", 20, || rec.answer(i.id, 1).is_some()).await;
    let child = rec.answer(i.id, 1).unwrap()["result"]["pid"]
        .as_u64()
        .unwrap() as u32;
    assert!(alive(child));
    until("in the job", 20, || m.pids(i.id).contains(&child)).await;
    m.stop(i.id);
    until("child ended", 20, || !alive(child)).await;
    assert_eq!(rec.states_of(i.id).last(), Some(&State::Stopped));
}

#[tokio::test]
async fn the_servers_of_a_project_no_longer_trusted_stop() {
    let dir = tmp("untrust");
    let rec = Arc::new(Rec::default());
    let m = Manager::new(rec.clone(), quick());
    let mut own = fake(&dir, &[]);
    own.of_project = true;
    let theirs = m.acquire("v", "p", "typescript", "/p", own);
    let machine = m.acquire("v", "p", "rust", "/p", fake(&dir, &[]));
    let elsewhere = {
        let mut l = fake(&dir, &[]);
        l.of_project = true;
        m.acquire("v", "q", "typescript", "/q", l)
    };
    m.stop_of_project("p");
    assert!(m.info(theirs.id).is_err());
    assert!(m.info(machine.id).is_ok() && m.info(elsewhere.id).is_ok());
    m.shutdown();
}

// ---------- in the core ----------

#[tokio::test]
async fn the_core_reads_the_project_file_and_keeps_the_trust_its_own() {
    let h = crate::core_tests::harness("lsp-core");
    let (p, repo) = h.project(false).await;
    std::fs::write(
        repo.join(".escouade.yml"),
        "version: 1\nlanguage_servers:\n  rust:\n    enabled: false\n",
    )
    .unwrap();
    h.core.read_shared(&p.id, false);
    let inputs = h.core.lsp_inputs(&p.id, None).await.unwrap();
    assert_eq!(inputs.root, repo);
    assert!(!inputs.trusted);
    let rust = inputs
        .statuses()
        .into_iter()
        .find(|s| s.id == "rust")
        .unwrap();
    assert_eq!((rust.enabled, rust.enabled_by), (false, "project"));
    assert_eq!(
        inputs.launch(server("rust")),
        Err(Refused::Disabled { by: "project" })
    );
    // Trusted by the core, kept in the settings' file.
    h.core.lsp_trust(&p.id, true).unwrap();
    assert!(h.core.lsp_inputs(&p.id, None).await.unwrap().trusted);
    let saved: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(saved["languageServersTrusted"], json!([p.id]));
    // A window's save neither trusts nor distrusts: the backend's copy stays.
    let mut from_window = h.core.settings.read().clone();
    from_window.language_servers_trusted = vec!["other".into()];
    from_window.language_servers.insert(
        "rust".into(),
        LanguageServerSetting {
            path: "ra".into(),
            ..Default::default()
        },
    );
    h.core.save_settings(from_window).unwrap();
    let s = h.core.settings.read().clone();
    assert_eq!(s.language_servers_trusted, vec![p.id.clone()]);
    assert_eq!(s.language_servers["rust"].path, "ra");
    h.core.lsp_trust(&p.id, false).unwrap();
    assert!(!h.core.lsp_inputs(&p.id, None).await.unwrap().trusted);
    // No such project: said.
    assert!(h.core.lsp_trust("nope", true).is_err());
}
