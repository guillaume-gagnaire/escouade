//! The launch environment of a worktree (INT-14, ESC-286; docs/PLUGINS.md §4.12): what runs a worktree's services on
//! ports and addresses of their own, so that a test launch needs no recipe on the reserved ports. isola is the first,
//! a plugin of Escouade's own hand (`dev.escouade.isola`), a module (`run.wasm`).
//!
//! A plugin declares an environment in `contributes.launchEnvironments`:
//!
//! - `detect`: the file whose presence in a worktree says its services are the environment's (`.isola.toml`);
//! - `program`: the CLI it needs, looked for on the PATH (`isola`);
//! - `up`: the command line a test launch runs, in a terminal of the project's, once the user has read `detect`'s
//!   content and approved it (the file may declare commands the agent wrote): `isola up`.
//!
//! The core does what needs no process: tell whether a worktree is the environment's (`Launch::for_dir`), read the file
//! whole for the user, run `up`. The plugin does the rest, by JSON-RPC: `launch/ls` (the services of the worktree, and
//! their addresses), `launch/down` and `launch/destroy`. A module holds no path: each call lends it the worktree, under a
//! name of the call's, and it asks the host to run what its manifest names there (`proc/*`, docs/PLUGINS.md §8.4.7). With
//! the plugin off (for the app, or for the project; a private project only has it once it is turned on there), or without
//! its program, or without its file, a worktree is nobody's and a ticket starts as it did without isola.
//!
//! Only the plugins the app gives serve this point for now: `up` is a command of the core's terminal, which a plugin
//! from elsewhere is not trusted with.

use crate::i18n::{self, Lang};
use crate::isola::Service;
use crate::plugins::adapters::PluginDown;
use crate::plugins::files::AgentPlace;
use crate::plugins::runtime::{CallError, Runtime};
use crate::plugins::Plugins;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The extension point of the manifest.
pub const POINT: &str = "launchEnvironments";

/// The largest file the user is asked to read: a bigger one is refused, never cut.
const FILE_LIMIT: u64 = 256 * 1024;

/// The launch environments of the plugins the app gives.
pub struct Launch {
    plugins: Arc<Plugins>,
    rt: Arc<Runtime>,
}

/// What a plugin declares for one environment.
#[derive(Clone)]
struct Decl {
    plugin: String,
    detect: String,
    program: String,
    up: String,
}

/// An environment that runs a worktree: its plugin and what it declares.
#[derive(Clone)]
pub struct Env {
    rt: Arc<Runtime>,
    decl: Decl,
}

/// The CLI `name`, on the PATH, as the host finds what a plugin runs (the tests' own for isola).
fn find(name: &str) -> Option<PathBuf> {
    if name == "isola" {
        crate::isola::cli()
    } else {
        crate::plugins::procs::locate(name)
    }
}

/// A name that is a file of a worktree's root or a program on the PATH, and nothing else.
fn plain(s: &str) -> bool {
    !s.is_empty() && !s.contains(['/', '\\', '\0']) && s != "." && s != ".." && !s.starts_with('-')
}

impl Launch {
    pub fn new(plugins: Arc<Plugins>, rt: Arc<Runtime>) -> Arc<Self> {
        Arc::new(Self { plugins, rt })
    }

    /// The environments on in `project`: the plugin on for the app and for it, as any plugin (a private project only has
    /// it once it is turned on there).
    fn decls(&self, project: &str) -> Vec<Decl> {
        self.plugins
            .builtin_contributions(POINT)
            .into_iter()
            .filter(|(plugin, choices, _)| self.rt.reaches(plugin, project, choices))
            .flat_map(|(plugin, _, items)| {
                items.into_iter().filter_map(move |item| {
                    let text = |k: &str| item.get(k).and_then(Value::as_str).map(str::to_string);
                    let (detect, program, up) = (text("detect")?, text("program")?, text("up")?);
                    (plain(&detect) && plain(&program) && !up.trim().is_empty()).then(|| Decl {
                        plugin: plugin.clone(),
                        detect,
                        program,
                        up,
                    })
                })
            })
            .collect()
    }

    /// The environment that runs the worktree at `dir` of `project`: the file it declares is there and its program
    /// is installed.
    pub fn for_dir(&self, dir: &str, project: &str) -> Option<Env> {
        self.decls(project).into_iter().find_map(|decl| {
            Path::new(dir).join(&decl.detect).is_file().then_some(())?;
            find(&decl.program)?;
            Some(Env {
                rt: self.rt.clone(),
                decl,
            })
        })
    }

    /// An environment runs the worktree at `dir`.
    pub fn manages(&self, dir: &str, project: &str) -> bool {
        self.for_dir(dir, project).is_some()
    }

    /// The folder `dir` has the file of an environment that is on (its program installed or not): what a project's
    /// recipe is told, so that it leaves the services to it.
    pub fn configured(&self, dir: &str, project: &str) -> bool {
        self.decls(project)
            .iter()
            .any(|d| Path::new(dir).join(&d.detect).is_file())
    }

    /// `rev` of the repository at `repo` has the committed file of an environment that is on and installed: a worktree
    /// made from it will be the environment's.
    pub async fn on_rev(&self, repo: &str, rev: &str, project: &str) -> bool {
        if rev.starts_with('-') {
            return false;
        }
        for decl in self.decls(project) {
            if find(&decl.program).is_some()
                && crate::git::run(repo, &["cat-file", "-e", &format!("{rev}:{}", decl.detect)])
                    .await
                    .is_ok()
            {
                return true;
            }
        }
        false
    }
}

/// What a failed call to the plugin is: the plugin's own message (the program's last line, in the language of the
/// interface), or a plugin that is down.
fn failed(plugin: &str, e: CallError) -> anyhow::Error {
    match e {
        CallError::Remote { message, .. } => anyhow!(message),
        down => PluginDown {
            plugin: plugin.to_string(),
            why: down.to_string(),
        }
        .into(),
    }
}

impl Env {
    /// The command line a test launch runs. Its input is not a terminal: the program never waits there for a password
    /// (to trust a certificate), it says how to do it instead.
    pub fn up_command(&self) -> String {
        if cfg!(windows) {
            self.decl.up.clone()
        } else {
            format!("{} < /dev/null", self.decl.up)
        }
    }

    /// The content of the file that says a worktree is this environment's, in `dir`, whole, as `up` reads it: what the
    /// user approves, and what is compared with it when the services start.
    pub fn read_config(&self, dir: &str) -> Result<String> {
        read_file(dir, &self.decl.detect)
    }

    /// A call of the plugin, the worktree at `dir` lent to it for that time under a name of the call's: `proc/spawn` takes
    /// that name, never a path. The core judged the project in `for_dir`: no project goes with the loan, and one being
    /// closed still has its services stopped.
    async fn call(&self, method: &str, dir: &str, branch: &str) -> Result<Value> {
        let lang = match i18n::ui() {
            Lang::Fr => "fr",
            Lang::En => "en",
        };
        let plugin = &self.decl.plugin;
        let key = format!("worktree-{}", uuid::Uuid::new_v4().simple());
        let place = AgentPlace {
            cwd: PathBuf::from(dir),
            ..AgentPlace::default()
        };
        self.rt
            .place_agent(plugin, &key, place)
            .map_err(|e| failed(plugin, e))?;
        let answer = self
            .rt
            .call(
                plugin,
                method,
                json!({ "agent": key, "branch": branch, "lang": lang }),
            )
            .await;
        self.rt.detach(plugin, &key);
        answer.map_err(|e| failed(plugin, e))
    }

    /// The services of the worktree at `dir`, on `branch`.
    pub async fn services(&self, dir: &str, branch: &str) -> Result<Vec<Service>> {
        if !Path::new(dir).is_dir() {
            bail!("{dir} is not a folder");
        }
        let v = self.call("launch/ls", dir, branch).await?;
        serde_json::from_value(v).map_err(|e| {
            anyhow!(tr!(
                "réponse du plugin {p} illisible : {e}",
                "the plugin {p}’s answer can’t be read: {e}",
                p = self.decl.plugin
            ))
        })
    }

    /// Stops the services of the worktree at `dir`.
    pub async fn down(&self, dir: &str) -> Result<()> {
        self.call("launch/down", dir, "").await.map(|_| ())
    }

    /// Tears the services (and what they keep) of the worktree at `dir` down.
    pub async fn destroy(&self, dir: &str) -> Result<()> {
        self.call("launch/destroy", dir, "").await.map(|_| ())
    }

    /// `down`, in the background; a failure is only logged. Not in a job of the app's: the services outlive it on
    /// purpose.
    pub fn down_later(self, dir: String) {
        tauri::async_runtime::spawn(async move {
            if let Err(e) = self.down(&dir).await {
                log::warn!("{dir}: {e:#}");
            }
        });
    }
}

/// The content of `file` in `dir`, whole, or why it can't be read.
pub fn read_file(dir: &str, file: &str) -> Result<String> {
    let path = Path::new(dir).join(file);
    let unreadable =
        |e: std::io::Error| anyhow!(tr!("{file} illisible : {e}", "{file} can’t be read: {e}"));
    let len = std::fs::metadata(&path).map_err(unreadable)?.len();
    if len > FILE_LIMIT {
        bail!(too_large(i18n::ui(), file, len));
    }
    let bytes = std::fs::read(&path).map_err(unreadable)?;
    String::from_utf8(bytes)
        .map_err(|_| anyhow!(tr!("{file} n'est pas en UTF-8.", "{file} isn’t in UTF-8.")))
}

/// Refused: the file is `len` bytes, too many for the user to read it first.
fn too_large(lang: Lang, file: &str, len: u64) -> String {
    tr_in!(
        lang,
        "{file} est trop gros pour être relu avant le lancement ({len} octets).",
        "{file} is too large to be read before the launch ({len} bytes)."
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::paths::test_dir;

    #[test]
    fn the_file_is_read_whole_or_refused_never_cut() {
        let d = test_dir("launchenv-read");
        let dir = d.to_string_lossy().to_string();
        assert!(read_file(&dir, ".isola.toml").is_err());
        let text = "[services.web]\r\ncommand = \"npm run dev\"\r\n";
        std::fs::write(d.join(".isola.toml"), text).unwrap();
        // As it is on disk, line endings included: this is what is compared when the services start.
        assert_eq!(read_file(&dir, ".isola.toml").unwrap(), text);
        std::fs::write(d.join(".isola.toml"), b"setup = \"\xff\"").unwrap();
        assert!(read_file(&dir, ".isola.toml").is_err());
        std::fs::write(d.join(".isola.toml"), "#".repeat(FILE_LIMIT as usize + 1)).unwrap();
        let e = read_file(&dir, ".isola.toml").unwrap_err().to_string();
        assert!(e.contains("trop gros"), "{e}");
        std::fs::write(d.join(".isola.toml"), "#".repeat(FILE_LIMIT as usize)).unwrap();
        assert_eq!(
            read_file(&dir, ".isola.toml").unwrap().len(),
            FILE_LIMIT as usize
        );
    }

    #[test]
    fn what_the_environment_refuses_reads_in_english() {
        assert_eq!(
            too_large(Lang::En, ".isola.toml", 300_000),
            ".isola.toml is too large to be read before the launch (300000 bytes)."
        );
    }

    #[test]
    fn a_name_in_a_manifest_is_a_file_of_the_root_or_a_program_never_a_path_or_an_option() {
        assert!(plain(".isola.toml") && plain("isola"));
        for bad in ["", ".", "..", "a/b", "a\\b", "--output=x", "-x"] {
            assert!(!plain(bad), "{bad}");
        }
    }
}
