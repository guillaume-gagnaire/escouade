//! What Escouade opens outside itself: a link in the browser, a file in its program, an item shown
//! in the file manager. Everywhere but in WSL, Tauri's opener does it (the shell of Windows and
//! macOS, `xdg-open` and D-Bus on Linux).
//!
//! **Under WSLg** (Escouade for Linux run in a distribution of WSL), `xdg-open` usually has nothing
//! to open with (no browser, no desktop in the distribution) and a click does nothing: the link,
//! the file or the folder goes to Windows instead. `wslview` (wslu) when it is installed, which
//! knows both sides; else Windows' own programs, through WSL's interop, a path of the distribution
//! given as Windows names it (`wslpath -w`: `\\wsl.localhost\Ubuntu\home\…`).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Result};

/// What is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What<'a> {
    /// A link (http, https, mailto…).
    Url(&'a str),
    /// A file in its program, or a folder in the file manager.
    Path(&'a Path),
    /// An item shown, selected, in its folder.
    Reveal(&'a Path),
}

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// Whether Escouade runs in a distribution of WSL: its interop with Windows is there.
pub fn in_wsl() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::env::var_os("WSL_INTEROP").is_some()
            || Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists())
}

/// How WSL opens `what` on Windows. `find` finds a program (the PATH, which WSL extends with
/// Windows' by default), `windows_path` names a path of the distribution as Windows does, `is_dir`
/// tells a folder. None when nothing can open it.
pub fn wsl_launch(
    what: What,
    find: impl Fn(&str) -> Option<PathBuf>,
    windows_path: impl Fn(&Path) -> Option<String>,
    is_dir: impl Fn(&Path) -> bool,
) -> Option<Launch> {
    let wslview = find("wslview");
    // Windows' programs, on the PATH or where Windows keeps them (its PATH may be left out of WSL's).
    let windows = |exe: &str, dir: &str| {
        find(exe).or_else(|| {
            let p = PathBuf::from(format!("/mnt/c/Windows/{dir}{exe}"));
            p.is_file().then_some(p)
        })
    };
    let rundll = || windows("rundll32.exe", "System32/");
    let explorer = || windows("explorer.exe", "");
    let launch = |program: PathBuf, args: Vec<String>| Some(Launch { program, args });
    match what {
        What::Url(url) => match wslview {
            Some(w) => launch(w, vec![url.to_string()]),
            None => launch(
                rundll()?,
                vec!["url.dll,FileProtocolHandler".into(), url.to_string()],
            ),
        },
        What::Path(path) if is_dir(path) => launch(explorer()?, vec![windows_path(path)?]),
        What::Path(path) => match wslview {
            Some(w) => launch(w, vec![path.to_string_lossy().into_owned()]),
            None => launch(
                rundll()?,
                vec!["url.dll,FileProtocolHandler".into(), windows_path(path)?],
            ),
        },
        What::Reveal(path) => launch(
            explorer()?,
            vec![format!("/select,{}", windows_path(path)?)],
        ),
    }
}

/// The name Windows gives a path of the distribution (`wslpath -w`).
fn wslpath_w(path: &Path) -> Option<String> {
    let out = Command::new("wslpath")
        .arg("-w")
        .arg(path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let name = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !name.is_empty()).then_some(name)
}

/// Starts `launch` and lets it go: the Explorer answers 1 even when it has done what it was
/// asked, so its end tells nothing. A thread waits for it, so that it leaves no zombie.
fn spawn(launch: &Launch) -> Result<()> {
    let mut child = Command::new(&launch.program)
        .args(&launch.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("{}: {e}", launch.program.display()))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Opens the link `url` outside Escouade (see the module's comment).
pub fn open_url(url: &str) -> Result<()> {
    open(What::Url(url))
}

/// Opens the path `allowed` outside Escouade: only one the fence let through (`opening::allow`, audit SEC-12).
pub fn open_allowed(allowed: &crate::opening::Allowed) -> Result<()> {
    open(allowed.what())
}

/// Opens `what` outside Escouade (see the module's comment).
fn open(what: What) -> Result<()> {
    if in_wsl() {
        let Some(launch) = wsl_launch(what, crate::which::find, wslpath_w, Path::is_dir) else {
            bail!(tr!(
                "Rien pour l’ouvrir sous Windows depuis WSL : installe wslu (wslview), ou garde le PATH de Windows dans WSL.",
                "Nothing to open it with on Windows from WSL: install wslu (wslview), or keep Windows’ PATH in WSL."
            ));
        };
        return spawn(&launch);
    }
    match what {
        What::Url(url) => tauri_plugin_opener::open_url(url, None::<&str>),
        What::Path(path) => tauri_plugin_opener::open_path(path, None::<&str>),
        What::Reveal(path) => tauri_plugin_opener::reveal_item_in_dir(path),
    }
    .map_err(|e| anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(names: &'static [&'static str]) -> impl Fn(&str) -> Option<PathBuf> {
        move |n: &str| {
            names
                .contains(&n)
                .then(|| PathBuf::from(format!("/usr/bin/{n}")))
        }
    }

    fn win(p: &Path) -> Option<String> {
        Some(format!(
            r"\\wsl.localhost\Ubuntu{}",
            p.to_string_lossy().replace('/', r"\")
        ))
    }

    fn launch(program: &str, args: &[&str]) -> Option<Launch> {
        Some(Launch {
            program: PathBuf::from(program),
            args: args.iter().map(|a| a.to_string()).collect(),
        })
    }

    #[test]
    fn under_wsl_wslview_opens_links_and_files_when_wslu_is_there() {
        let file = Path::new("/home/mike/pilote.csv");
        let has = found(&["wslview", "rundll32.exe", "explorer.exe"]);
        assert_eq!(
            wsl_launch(
                What::Url("https://escouade.dev/?a=1&b=2"),
                &has,
                win,
                |_| false
            ),
            launch("/usr/bin/wslview", &["https://escouade.dev/?a=1&b=2"])
        );
        // wslview takes the distribution's path itself.
        assert_eq!(
            wsl_launch(What::Path(file), &has, win, |_| false),
            launch("/usr/bin/wslview", &["/home/mike/pilote.csv"])
        );
    }

    #[test]
    fn without_wslu_windows_own_programs_open_them_with_windows_paths() {
        let file = Path::new("/home/mike/pilote.csv");
        let has = found(&["rundll32.exe", "explorer.exe"]);
        assert_eq!(
            wsl_launch(What::Url("https://escouade.dev"), &has, win, |_| false),
            launch(
                "/usr/bin/rundll32.exe",
                &["url.dll,FileProtocolHandler", "https://escouade.dev"]
            )
        );
        assert_eq!(
            wsl_launch(What::Path(file), &has, win, |_| false),
            launch(
                "/usr/bin/rundll32.exe",
                &[
                    "url.dll,FileProtocolHandler",
                    r"\\wsl.localhost\Ubuntu\home\mike\pilote.csv"
                ]
            )
        );
    }

    #[test]
    fn under_wsl_folders_and_reveals_go_to_the_explorer() {
        let dir = Path::new("/home/mike/projet");
        let file = Path::new("/home/mike/projet/a.md");
        // Even with wslu: the Explorer is the file manager there is.
        let has = found(&["wslview", "rundll32.exe", "explorer.exe"]);
        assert_eq!(
            wsl_launch(What::Path(dir), &has, win, |_| true),
            launch(
                "/usr/bin/explorer.exe",
                &[r"\\wsl.localhost\Ubuntu\home\mike\projet"]
            )
        );
        assert_eq!(
            wsl_launch(What::Reveal(file), &has, win, |_| false),
            launch(
                "/usr/bin/explorer.exe",
                &[r"/select,\\wsl.localhost\Ubuntu\home\mike\projet\a.md"]
            )
        );
    }

    #[test]
    fn under_wsl_nothing_found_or_no_windows_name_opens_nothing() {
        let file = Path::new("/home/mike/a.md");
        let none = found(&[]);
        if !Path::new("/mnt/c/Windows/System32/rundll32.exe").exists() {
            assert_eq!(
                wsl_launch(What::Url("https://x"), &none, win, |_| false),
                None
            );
        }
        // A path Windows cannot name is not handed to it.
        let has = found(&["rundll32.exe", "explorer.exe"]);
        assert_eq!(
            wsl_launch(What::Path(file), &has, |_| None, |_| false),
            None
        );
        assert_eq!(
            wsl_launch(What::Reveal(file), &has, |_| None, |_| false),
            None
        );
    }
}
