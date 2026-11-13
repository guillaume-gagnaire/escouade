//! The program of a harness, run as a program of the system or, under Windows, as one installed
//! in WSL and run through `wsl.exe`.

use anyhow::{anyhow, Result};
use escouade_base::i18n::{self, Lang};
use escouade_model::wsl;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The `claude` an account runs: a program of the system, or, under Windows, one installed in WSL
/// and run through `wsl.exe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Program {
    Native(PathBuf),
    Wsl(wsl::Program),
}

/// A process to start: its program, its arguments and the variables it gets on top of the app's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// The options whose value is a file or a folder Escouade made for Claude Code (in its data folder,
/// on Windows): a `claude` of WSL is given its Linux path. `--add-dir` is the folder of the files
/// joined to an agent's messages.
const PATH_OPTIONS: [&str; 3] = ["--mcp-config", "--settings", "--add-dir"];

/// Stands for an empty argument on `wsl.exe`'s command line, which it refuses (E_INVALIDARG):
/// `WSL_RUN` puts the empty argument back.
const WSL_EMPTY: &str = "escouade-empty-argument";

/// What `sh` runs in WSL: in the folder `$1` (or not at all: `wsl.exe` would fall back on the home
/// folder, or on `/`, without a word), the program (`$0`) with its arguments, empty ones put back,
/// and its own folder first on the PATH (an npm install under nvm finds its `node` beside it).
const WSL_RUN: &str = r#"p=$0; cd -- "$1" || exit 126; shift; n=$#; while [ "$n" -gt 0 ]; do a=$1; shift; [ "$a" = 'escouade-empty-argument' ] && a=; set -- "$@" "$a"; n=$((n - 1)); done; PATH="${p%/*}:$PATH" exec "$p" "$@""#;

impl Program {
    /// How to start it in the folder `cwd` with `args` and the variables `env` (the caller still
    /// starts the process in `cwd`). In WSL, a variable set on `wsl.exe` reaches Linux only when
    /// `WSLENV` names it, so each one is named there. The files Escouade wrote, `CLAUDE_CONFIG_DIR`
    /// and the folder are given by their Linux paths, and a folder WSL cannot reach is refused.
    /// Without a shell in between (`--exec`), nothing in the arguments is read as shell code.
    /// `WSL_UTF8` makes `wsl.exe` write its own messages in UTF-8 rather than UTF-16.
    pub fn launch(&self, cwd: &str, args: &[String], env: &[(String, String)]) -> Result<Launch> {
        let wsl = match self {
            Program::Native(path) => {
                return Ok(Launch {
                    program: path.clone(),
                    args: args.to_vec(),
                    env: env.to_vec(),
                })
            }
            Program::Wsl(wsl) => wsl,
        };
        let dir = wsl
            .linux_path(cwd)
            .ok_or_else(|| anyhow!(out_of_wsl(i18n::ui(), cwd, &wsl.distro)))?;
        let linux = |value: &str| wsl.linux_path(value).unwrap_or_else(|| value.to_string());
        let mut vars: Vec<(String, String)> =
            env.iter()
                .map(|(k, v)| match k.as_str() {
                    escouade_model::accounts::CONFIG_DIR_VAR
                    | "CLAUDE_SECURESTORAGE_CONFIG_DIR" => (k.clone(), linux(v)),
                    _ => (k.clone(), v.clone()),
                })
                .collect();
        let names: Vec<&str> = vars.iter().map(|(k, _)| k.as_str()).collect();
        if let Some(listed) = wslenv(std::env::var("WSLENV").ok().as_deref(), &names) {
            vars.push(("WSLENV".into(), listed));
        }
        vars.push(("WSL_UTF8".into(), "1".into()));
        let mut out: Vec<String> = ["-d", &wsl.distro, "--exec", "/bin/sh", "-c", WSL_RUN]
            .map(String::from)
            .to_vec();
        out.extend([wsl.path.clone(), dir]);
        let mut path_next = false;
        for arg in args {
            let arg = if path_next {
                linux(arg)
            } else {
                match arg.split_once('=') {
                    Some((option, value)) if PATH_OPTIONS.contains(&option) => {
                        format!("{option}={}", linux(value))
                    }
                    _ => arg.clone(),
                }
            };
            path_next = PATH_OPTIONS.contains(&arg.as_str());
            out.push(if arg.is_empty() {
                WSL_EMPTY.into()
            } else {
                arg
            });
        }
        Ok(Launch {
            program: wsl.exe.clone(),
            args: out,
            env: vars,
        })
    }

    /// The path it reads a file Escouade wrote by: its Linux one in WSL.
    pub fn file_path(&self, file: &Path) -> String {
        let path = file.to_string_lossy();
        match self {
            Program::Wsl(wsl) => wsl.linux_path(&path),
            Program::Native(_) => None,
        }
        .unwrap_or_else(|| path.into_owned())
    }

    /// What the user knows it by, in messages: its path, and its distribution for one of WSL.
    pub fn describe(&self) -> String {
        match self {
            Program::Native(path) => path.display().to_string(),
            Program::Wsl(wsl) => format!("{} (WSL {})", wsl.path, wsl.distro),
        }
    }

    /// The program of WSL, for one that runs there.
    pub fn wsl(&self) -> Option<&wsl::Program> {
        match self {
            Program::Wsl(wsl) => Some(wsl),
            Program::Native(_) => None,
        }
    }
}

/// Where the program of a setting is, as the line under its field tells it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Located {
    /// Its path (its Linux path for one of WSL); None when it is not found.
    pub path: Option<String>,
    /// Its distribution, for one of WSL.
    pub wsl: Option<String>,
    /// For one of WSL: WSL does not share the network of Windows (`wsl::mirrored`), so Escouade's
    /// MCP server and the hooks of the Claude Code view, on 127.0.0.1 of Windows, are out of its
    /// reach.
    pub nat: bool,
    /// A program of WSL may be named (under Windows).
    pub can_wsl: bool,
}

impl Located {
    /// Where `found` is, told as the settings show it.
    pub fn of(found: Option<Program>) -> Self {
        let located = match found {
            Some(Program::Native(path)) => Located {
                path: Some(path.display().to_string()),
                ..Default::default()
            },
            Some(Program::Wsl(wsl)) => Located {
                path: Some(wsl.path),
                wsl: Some(wsl.distro),
                nat: !wsl::mirrored_here(),
                ..Default::default()
            },
            None => Located::default(),
        };
        Located {
            can_wsl: cfg!(windows),
            ..located
        }
    }
}

/// `WSLENV` naming the variables `names` for Linux after those the app's (`existing`) names; None
/// when there is none to name.
fn wslenv(existing: Option<&str>, names: &[&str]) -> Option<String> {
    let all: Vec<&str> = existing
        .into_iter()
        .flat_map(|e| e.split(':'))
        .chain(names.iter().copied())
        .filter(|n| !n.is_empty())
        .collect();
    (!all.is_empty()).then(|| all.join(":"))
}

/// Refused, in `lang`: the folder `cwd` has no path in WSL (a network share, another
/// distribution's folder), where the `claude` of `distro` would run in another one.
fn out_of_wsl(lang: Lang, cwd: &str, distro: &str) -> String {
    tr_in!(
        lang,
        "Le dossier {cwd} n’est pas joignable depuis WSL ({distro}) : le Claude Code qui y est installé ne peut pas y travailler.",
        "The folder {cwd} can’t be reached from WSL ({distro}): the Claude Code installed there can’t work in it."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn ubuntu_claude() -> wsl::Program {
        wsl::Program {
            exe: PathBuf::from(r"C:\Windows\System32\wsl.exe"),
            distro: "Ubuntu-24.04".into(),
            path: "/home/me/.local/bin/claude".into(),
            home: "/home/me".into(),
            mount_root: "/mnt/".into(),
        }
    }

    #[test]
    fn a_file_escouade_wrote_is_given_to_a_claude_of_wsl_by_its_linux_path() {
        let file = Path::new(r"C:\Users\Jean Dupont\.escouade\tui\pastes\4f2a.png");
        assert_eq!(
            Program::Wsl(ubuntu_claude()).file_path(file),
            "/mnt/c/Users/Jean Dupont/.escouade/tui/pastes/4f2a.png"
        );
        assert_eq!(
            Program::Native(PathBuf::from(r"C:\bin\claude.exe")).file_path(file),
            r"C:\Users\Jean Dupont\.escouade\tui\pastes\4f2a.png"
        );
    }

    #[test]
    fn a_claude_of_wsl_runs_through_wsl_exe_its_files_and_variables_in_linux() {
        let launch = Program::Wsl(ubuntu_claude())
            .launch(
                r"C:\Users\me\proj",
                &strings(&[
                    "-p",
                    "--mcp-config",
                    r"C:\Users\me\.escouade\mcp\a1.json",
                    r"--settings=C:\Users\me\.escouade\tui\a1.json",
                    "--tools",
                    "",
                    "--system-prompt",
                    "l'été, $HOME et `ls` restent du texte\n",
                    "--settings",
                    r#"{"hooks":{}}"#,
                ]),
                &[
                    (
                        "CLAUDE_CONFIG_DIR".into(),
                        r"C:\Users\me\.escouade\claude\pro".into(),
                    ),
                    ("HTTPS_PROXY".into(), "http://proxy:3128".into()),
                ],
            )
            .unwrap();
        assert_eq!(
            launch.program,
            PathBuf::from(r"C:\Windows\System32\wsl.exe")
        );
        assert_eq!(
            launch.args,
            strings(&[
                "-d",
                "Ubuntu-24.04",
                "--exec",
                "/bin/sh",
                "-c",
                WSL_RUN,
                "/home/me/.local/bin/claude",
                // The folder it works in, which the script goes into or refuses.
                "/mnt/c/Users/me/proj",
                "-p",
                "--mcp-config",
                "/mnt/c/Users/me/.escouade/mcp/a1.json",
                "--settings=/mnt/c/Users/me/.escouade/tui/a1.json",
                "--tools",
                WSL_EMPTY,
                "--system-prompt",
                "l'été, $HOME et `ls` restent du texte\n",
                // Settings given inline are no file.
                "--settings",
                r#"{"hooks":{}}"#,
            ])
        );
        // The variables reach Linux by WSLENV, not on the command line (a token, a proxy's
        // password), and wsl.exe says what it has to say in UTF-8.
        let names = ["CLAUDE_CONFIG_DIR", "HTTPS_PROXY"];
        assert_eq!(
            launch.env,
            vec![
                (
                    "CLAUDE_CONFIG_DIR".to_string(),
                    "/mnt/c/Users/me/.escouade/claude/pro".to_string()
                ),
                ("HTTPS_PROXY".into(), "http://proxy:3128".into()),
                (
                    "WSLENV".into(),
                    wslenv(std::env::var("WSLENV").ok().as_deref(), &names).unwrap()
                ),
                ("WSL_UTF8".into(), "1".into()),
            ]
        );
        assert!(WSL_RUN.contains(&format!("'{WSL_EMPTY}'")));
    }

    #[test]
    fn the_folder_of_the_joined_files_is_given_to_a_claude_of_wsl_by_its_linux_path_in_both_forms()
    {
        let launch = Program::Wsl(ubuntu_claude())
            .launch(
                r"C:\Users\me\proj",
                &strings(&[
                    r"--add-dir=C:\Users\Jean Dupont\.escouade\attachments\a1",
                    "-p",
                    "--add-dir",
                    r"C:\Users\me\.escouade\attachments\a2",
                    "--tools",
                    "",
                ]),
                &[],
            )
            .unwrap();
        let tail = &launch.args[8..];
        assert_eq!(
            tail,
            strings(&[
                "--add-dir=/mnt/c/Users/Jean Dupont/.escouade/attachments/a1",
                "-p",
                "--add-dir",
                "/mnt/c/Users/me/.escouade/attachments/a2",
                "--tools",
                WSL_EMPTY,
            ])
        );
        // A claude of the system is given the folder as it is.
        let args = strings(&[r"--add-dir=C:\x\attachments\a1"]);
        let native = Program::Native(PathBuf::from(r"C:\tools\claude.exe"))
            .launch(r"C:\p", &args, &[])
            .unwrap();
        assert_eq!(native.args, args);
    }

    #[test]
    fn a_folder_wsl_cannot_reach_is_refused_rather_than_another_used() {
        let refused = Program::Wsl(ubuntu_claude())
            .launch(r"\\server\share\proj", &[], &[])
            .unwrap_err();
        assert!(
            refused.to_string().contains(r"\\server\share\proj"),
            "{refused}"
        );
        assert_eq!(
            out_of_wsl(Lang::En, r"\\server\share\proj", "Ubuntu"),
            r"The folder \\server\share\proj can’t be reached from WSL (Ubuntu): the Claude Code installed there can’t work in it."
        );
    }

    #[test]
    fn wslenv_names_the_variables_after_those_the_app_names() {
        assert_eq!(wslenv(None, &["A", "B"]).as_deref(), Some("A:B"));
        assert_eq!(
            wslenv(Some("USERPROFILE/p:"), &["A"]).as_deref(),
            Some("USERPROFILE/p:A")
        );
        assert_eq!(wslenv(Some(""), &[]), None);
    }

    #[test]
    fn a_program_of_the_system_starts_as_it_is() {
        let path = PathBuf::from(r"C:\tools\claude.exe");
        let args = strings(&["--tools", "", "--mcp-config", r"C:\x.json"]);
        let env = vec![("CLAUDE_CONFIG_DIR".to_string(), r"C:\cfg".to_string())];
        assert_eq!(
            Program::Native(path.clone())
                .launch(r"\\server\share", &args, &env)
                .unwrap(),
            Launch {
                program: path,
                args,
                env
            }
        );
    }
}
