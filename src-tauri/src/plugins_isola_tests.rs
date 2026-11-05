//! Tests of isola as a module (ESC-351, `plugins/isola`), the one plugin of Escouade's own hand that runs a program, played
//! as the app plays it on the test double `tests/fixtures/fake-isola.mjs`.
//!
//! Its parity: `tests/fixtures/plugins/parity/isola.json` holds what the native plugin answered and which commands it ran,
//! at 6dbd1223, before its port took it out of `escouade-plugins`. `ESCOUADE_PARITY_RECORD=1` and
//! `ESCOUADE_NATIVE_PLUGINS=<a program of then>` write it again. The module comes from `sh scripts/plugins-wasm.sh`;
//! without it, the parity says so and passes, and with `ESCOUADE_PARITY=1` it fails.

use crate::integrations::secrets::MemoryStore;
use crate::launchenv::Launch;
use crate::paths::DataDir;
use crate::plugins::events::Place;
use crate::plugins::files::AgentPlace;
use crate::plugins::runtime::{CallError, Limits, Runtime};
use crate::plugins::Plugins;
use crate::pty::ShellInfo;
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

const ID: &str = "dev.escouade.isola";
const VERSION: &str = "2.2.0";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn app_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

fn base(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "plugins-isola-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rig(name: &str) -> (Arc<Plugins>, Arc<Runtime>) {
    let data = DataDir::new(base(name).join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        Limits {
            initialize: Duration::from_secs(20),
            shutdown: Duration::from_secs(2),
            ..Limits::default()
        },
    );
    (plugins, rt)
}

/// A plugin as it was, or as it is.
enum Played<'a> {
    /// `escouade-plugins isola`, and the fake it was given.
    Native {
        child: Child,
        input: ChildStdin,
        output: BufReader<ChildStdout>,
        program: String,
        dir: String,
    },
    Module(&'a Arc<Runtime>),
}

impl Played<'_> {
    fn native(fake: &Path, dir: &Path) -> Self {
        let exe = std::env::current_exe().unwrap();
        let beside = exe.parent().unwrap().parent().unwrap();
        let program = std::env::var_os("ESCOUADE_NATIVE_PLUGINS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                beside.join(format!("escouade-plugins{}", std::env::consts::EXE_SUFFIX))
            });
        let mut child = Command::new(&program)
            .arg("isola")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}", program.display()));
        Self::Native {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
            program: fake.to_string_lossy().into_owned(),
            dir: dir.to_string_lossy().into_owned(),
        }
    }

    /// What a call answers: its result, or its error, its code and its message.
    async fn call(&mut self, method: &str, branch: &str, lang: &str) -> Value {
        match self {
            Self::Native {
                input,
                output,
                program,
                dir,
                ..
            } => tokio::task::block_in_place(|| {
                let params =
                    json!({ "program": program, "dir": dir, "branch": branch, "lang": lang });
                let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
                writeln!(input, "{line}").unwrap();
                input.flush().unwrap();
                let mut answer = String::new();
                output.read_line(&mut answer).unwrap();
                let v: Value = serde_json::from_str(&answer).expect("JSON");
                match v.get("error") {
                    Some(e) => json!({ "code": e["code"], "message": e["message"] }),
                    None => json!({ "result": v["result"] }),
                }
            }),
            Self::Module(rt) => {
                let params = json!({ "agent": "wt", "branch": branch, "lang": lang });
                match rt.call(ID, method, params).await {
                    Ok(result) => json!({ "result": result }),
                    Err(CallError::Remote { code, message }) => {
                        json!({ "code": code, "message": message })
                    }
                    Err(e) => panic!("{method}: {e}"),
                }
            }
        }
    }
}

impl Drop for Played<'_> {
    fn drop(&mut self) {
        if let Self::Native { child, .. } = self {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Each call of the plugin that matters: its answer, and the commands it ran.
async fn transcript(plugin: &mut Played<'_>, wt: &Path) -> Value {
    let dir = wt.to_string_lossy().to_string();
    let touch = |name: &str| std::fs::write(wt.join(name), "").unwrap();
    let rid = |name: &str| std::fs::remove_file(wt.join(name)).unwrap();
    let up = || crate::isola::run(&dir, &["up"], crate::isola::LIMIT);
    let mut said = Vec::new();
    let mut ran = 0;
    std::fs::write(wt.join(".isola-branch"), "feat").unwrap();
    macro_rules! step {
        ($method:expr, $branch:expr, $lang:expr) => {
            let answer = plugin.call($method, $branch, $lang).await;
            let calls = crate::isola::tests::calls(wt);
            said.push(json!({
                "method": $method,
                "branch": $branch,
                "lang": $lang,
                "answer": answer,
                "ran": calls[ran..],
            }));
            ran = calls.len();
        };
    }
    step!("launch/ls", "feat", "en");
    up().await.unwrap();
    ran += 1;
    step!("launch/ls", "feat", "en");
    step!("launch/ls", "another", "fr");
    touch(".isola-garbage");
    step!("launch/ls", "feat", "fr");
    step!("launch/ls", "feat", "en");
    rid(".isola-garbage");
    step!("launch/down", "", "en");
    step!("launch/ls", "feat", "en");
    up().await.unwrap();
    ran += 1;
    step!("launch/destroy", "", "fr");
    touch(".isola-fail");
    step!("launch/ls", "feat", "en");
    step!("launch/ls", "feat", "fr");
    step!("launch/down", "", "en");
    step!("launch/down", "", "fr");
    step!("launch/destroy", "", "en");
    step!("launch/destroy", "", "fr");
    rid(".isola-fail");
    step!("launch/up", "", "en");
    assert_eq!(ran, crate::isola::tests::calls(wt).len());
    Value::Array(said)
}

fn recording() -> PathBuf {
    root().join("tests/fixtures/plugins/parity/isola.json")
}

#[tokio::test(flavor = "multi_thread")]
async fn isola_runs_the_same_commands_and_says_the_same_as_its_native_plugin() {
    let built = app_dir().join("../plugins-wasm/isola/plugin.wasm");
    if !built.is_file() {
        assert!(
            std::env::var_os("ESCOUADE_PARITY").is_none(),
            "{}",
            built.display()
        );
        eprintln!("skipped: {} is not built", built.display());
        return;
    }
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let fake = crate::isola::tests::use_fake();
    let (_plugins, rt) = rig("parity");
    let wt = base("parity-worktree");
    let place = AgentPlace {
        cwd: wt.clone(),
        ..AgentPlace::default()
    };
    rt.place_agent(ID, "wt", place).unwrap();
    let played = transcript(&mut Played::Module(&rt), &wt).await;
    rt.shutdown();
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let wt = base("parity-native");
        let was = transcript(&mut Played::native(&fake, &wt), &wt).await;
        let text = serde_json::to_string_pretty(&was).unwrap();
        std::fs::write(recording(), format!("{text}\n")).unwrap();
    }
    let was: Value =
        serde_json::from_str(&std::fs::read_to_string(recording()).unwrap()).expect("a recording");
    for (a, b) in played
        .as_array()
        .unwrap()
        .iter()
        .zip(was.as_array().unwrap())
    {
        assert_eq!(a, b);
    }
    assert_eq!(played, was);
}

/// A project of the test: private or not.
struct OnePlace {
    private: bool,
}

impl Place for OnePlace {
    fn shell(&self) -> Option<ShellInfo> {
        None
    }
    fn project_path(&self, _: &str) -> Option<String> {
        None
    }
    fn project_plugin(&self, _: &str, _: &str) -> Option<(bool, Map<String, Value>)> {
        Some((self.private, Map::new()))
    }
}

/// A plugin does not reach a private project until it is turned on there, and isola is one like the others: its worktrees
/// are nobody's there, and no recipe is told to leave them to it.
#[tokio::test(flavor = "multi_thread")]
async fn a_private_project_has_isola_only_once_it_is_turned_on_there() {
    crate::isola::tests::use_fake();
    let (plugins, rt) = rig("private");
    let place: Arc<dyn Place> = Arc::new(OnePlace { private: true });
    rt.place_with(Arc::downgrade(&place));
    let launch = Launch::new(plugins.clone(), rt.clone());
    let wt = base("private-worktree");
    let dir = wt.to_string_lossy().to_string();
    std::fs::write(wt.join(".isola.toml"), "").unwrap();
    assert!(!launch.manages(&dir, "secret") && !launch.configured(&dir, "secret"));
    plugins
        .choose(ID, |c| {
            c.projects.insert("secret".into(), true);
        })
        .unwrap();
    assert!(launch.manages(&dir, "secret") && launch.configured(&dir, "secret"));
    rt.shutdown();
}

/// The worktree is lent for a call: nothing of it is left to the plugin after, and what is missing is said as it was.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktree_that_is_gone_is_said_and_a_loan_ends_with_its_call() {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    crate::isola::tests::use_fake();
    let (plugins, rt) = rig("loan");
    let launch = Launch::new(plugins, rt.clone());
    let wt = base("loan-worktree");
    let dir = wt.to_string_lossy().to_string();
    std::fs::write(wt.join(".isola.toml"), "").unwrap();
    let env = launch.for_dir(&dir, "p").unwrap();
    assert!(env.services(&dir, "main").await.unwrap().is_empty());
    assert_eq!(
        rt.lent_places(ID),
        0,
        "the plugin keeps no worktree between two calls"
    );
    std::fs::remove_dir_all(&wt).unwrap();
    let e = env.services(&dir, "main").await.unwrap_err().to_string();
    assert_eq!(e, format!("{dir} is not a folder"));
    rt.shutdown();
}
