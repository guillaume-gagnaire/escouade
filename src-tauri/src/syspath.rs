//! The PATH of today (ESC-186): a process keeps the PATH it was started with, and a harness
//! installed while the app runs (`npm install -g @openai/codex` adds `%APPDATA%\npm` to the PATH
//! of the user, in the registry) would not be found before a restart. Each detection of the
//! harnesses (the start, « Chercher à nouveau », the window back in front, the menu « Harness »,
//! Réglages › Comptes et harness) reads it again first: under Windows the PATH of the registry (the user's,
//! then the machine's, their variables expanded), elsewhere the login shell's (`shellenv`); then
//! the folders the harnesses install into (npm's global prefix, `~/.local/bin`, Homebrew…).
//! What is missing of them is added at the end of the process's own PATH, which the agents'
//! processes inherit: they run with the PATH they were found by.
//!
//! In a debug build, `ESCOUADE_TEST_PATH_FILE` names a file that stands for all of that (the
//! end-to-end tests): its folders, one a line, and nothing of the registry, the shell or the usual
//! folders. No test reads or writes the real PATH of the user.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Reads the PATH of today and adds to the process's what it lacks of it. Cheap under Windows (two
/// values of the registry); the login shell elsewhere is asked once a minute at most.
pub fn refresh() {
    let extra = match test_file() {
        Some(file) => from_file(&file),
        None => outside(),
    };
    let current = std::env::var_os("PATH").unwrap_or_default();
    let Some(merged) = merge(&current, &extra) else {
        return;
    };
    log::info!(
        "PATH: {} folders added",
        std::env::split_paths(&merged).count() - std::env::split_paths(&current).count()
    );
    // Rust's own reads and writes of the environment are under one lock; the processes started
    // from now on inherit it.
    std::env::set_var("PATH", merged);
}

/// `current` with the folders of `extra` it lacks at its end, each once (without regard to case
/// or a final separator under Windows), in their order; None when it lacks none.
pub fn merge(current: &OsStr, extra: &[PathBuf]) -> Option<OsString> {
    let mut all: Vec<PathBuf> = std::env::split_paths(current).collect();
    let mut seen: Vec<String> = all.iter().map(|p| key(p)).collect();
    let before = all.len();
    for p in extra {
        let k = key(p);
        if p.as_os_str().is_empty() || seen.contains(&k) {
            continue;
        }
        seen.push(k);
        all.push(p.clone());
    }
    if all.len() == before {
        return None;
    }
    std::env::join_paths(all).ok()
}

/// How two folders of the PATH are told apart.
fn key(p: &Path) -> String {
    let s = p.to_string_lossy();
    let s = s.trim_end_matches(['/', '\\']);
    if cfg!(windows) {
        s.replace('/', "\\").to_lowercase()
    } else {
        s.to_string()
    }
}

/// The file of `ESCOUADE_TEST_PATH_FILE`, in a debug build (never in a release, nor the unit
/// tests').
fn test_file() -> Option<PathBuf> {
    if cfg!(test) || !cfg!(debug_assertions) {
        return None;
    }
    std::env::var_os("ESCOUADE_TEST_PATH_FILE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The folders of a test's file, one a line (it may not be there yet).
fn from_file(file: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// The PATH of the system as a new terminal would get it, then the folders the harnesses install
/// into.
fn outside() -> Vec<PathBuf> {
    let mut out = system();
    out.extend(install_folders(
        dirs::home_dir().as_deref(),
        std::env::var_os("APPDATA").as_deref().map(Path::new),
        std::env::var_os("LOCALAPPDATA").as_deref().map(Path::new),
    ));
    out
}

/// Under Windows, the PATH of the registry: the user's (`HKCU\Environment`), then the machine's
/// (`HKLM\…\Session Manager\Environment`), their `%VARIABLES%` expanded.
#[cfg(windows)]
fn system() -> Vec<PathBuf> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let user = registry::string(HKEY_CURRENT_USER, "Environment", "Path");
    let machine = registry::string(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        "Path",
    );
    [user, machine]
        .into_iter()
        .flatten()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .collect()
}

/// Elsewhere, the login shell's PATH (`shellenv`), asked once a minute at most (it may take
/// seconds).
#[cfg(unix)]
fn system() -> Vec<PathBuf> {
    use parking_lot::Mutex;
    use std::time::{Duration, Instant};
    static LAST: Mutex<Option<(Instant, Vec<PathBuf>)>> = Mutex::new(None);
    let mut last = LAST.lock();
    if let Some((at, kept)) = last.as_ref() {
        if at.elapsed() < Duration::from_secs(60) {
            return kept.clone();
        }
    }
    let read: Vec<PathBuf> = crate::shellenv::login_path()
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    *last = Some((Instant::now(), read.clone()));
    read
}

/// The folders the harnesses' installers put their programs in, which the PATH of a process
/// started before may lack: npm's global prefix (`prefix=` of `~/.npmrc`, else its default:
/// `%APPDATA%\npm` under Windows), winget's links (`%LOCALAPPDATA%\Microsoft\WinGet\Links`: Copilot
/// CLI by `winget install GitHub.Copilot`), `~/.local/bin` (Claude Code's native installer), and
/// Homebrew's elsewhere. Only those that are there.
pub fn install_folders(
    home: Option<&Path>,
    appdata: Option<&Path>,
    local_appdata: Option<&Path>,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let npmrc = home.and_then(|h| std::fs::read_to_string(h.join(".npmrc")).ok());
    let prefix = npmrc.as_deref().and_then(npm_prefix);
    if cfg!(windows) {
        // npm's programs are in its prefix itself under Windows.
        out.extend(prefix.or_else(|| appdata.map(|a| a.join("npm"))));
        out.extend(local_appdata.map(|l| l.join("Microsoft").join("WinGet").join("Links")));
    } else {
        out.extend(prefix.map(|p| p.join("bin")));
        out.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    if let Some(h) = home {
        out.push(h.join(".local").join("bin"));
    }
    out.into_iter().filter(|p| p.is_dir()).collect()
}

/// The `prefix` an `.npmrc` sets (`prefix=C:\Users\me\npm-global`, `~/` for the home folder
/// left as npm reads it: as it is).
pub fn npm_prefix(npmrc: &str) -> Option<PathBuf> {
    npmrc.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "prefix")
            .then(|| v.trim().trim_matches('"'))
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    })
}

/// A value of the registry (Windows), a `REG_EXPAND_SZ` one expanded.
#[cfg(windows)]
mod registry {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY, RRF_RT_REG_SZ};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    pub fn string(root: HKEY, key: &str, value: &str) -> Option<String> {
        let (k, v) = (wide(key), wide(value));
        let mut size = 0u32;
        // SAFETY: the names end with a NUL; the first call only asks for the size in bytes.
        // `RRF_RT_REG_SZ` without `RRF_NOEXPAND`: a `REG_EXPAND_SZ` is expanded, as REG_SZ.
        let first = unsafe {
            RegGetValueW(
                root,
                k.as_ptr(),
                v.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if first != 0 || size == 0 {
            return None;
        }
        // Its expansion may be longer than the size the first call gave: room for it.
        let mut buf = vec![0u16; size as usize + 2048];
        let mut bytes = (buf.len() * 2) as u32;
        // SAFETY: `buf` holds `bytes` bytes.
        let read = unsafe {
            RegGetValueW(
                root,
                k.as_ptr(),
                v.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if read != 0 {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_path_lacks_is_added_at_its_end_once_whatever_its_case() {
        let dir = crate::paths::test_dir("esc186-path-merge");
        let (a, b, npm) = (dir.join("a"), dir.join("b"), dir.join("npm"));
        let current = std::env::join_paths([&a, &b]).unwrap();
        // Nothing new: nothing to change.
        assert_eq!(merge(&current, std::slice::from_ref(&b)), None);
        let upper = PathBuf::from(b.to_string_lossy().to_uppercase() + "\\");
        let merged = merge(&current, &[upper, npm.clone(), npm.clone()]);
        if cfg!(windows) {
            assert_eq!(merged, Some(std::env::join_paths([&a, &b, &npm]).unwrap()));
        } else {
            assert!(merged.is_some());
        }
        // The process's own order first: a program it found stays the one it finds.
        let merged = merged.unwrap();
        let first = std::env::split_paths(&merged).next().unwrap();
        assert_eq!(first, a);
    }

    #[test]
    fn npms_global_prefix_is_read_from_its_npmrc_else_its_default() {
        assert_eq!(
            npm_prefix("registry=https://r\nprefix = C:\\Users\\me\\npm-global\n"),
            Some(PathBuf::from("C:\\Users\\me\\npm-global"))
        );
        assert_eq!(npm_prefix("registry=https://r\n"), None);
        assert_eq!(npm_prefix("prefix=\n"), None);
        let dir = crate::paths::test_dir("esc186-install-folders");
        let (home, appdata, local) = (dir.join("home"), dir.join("appdata"), dir.join("local"));
        std::fs::create_dir_all(appdata.join("npm")).unwrap();
        std::fs::create_dir_all(home.join(".local").join("bin")).unwrap();
        let winget = local.join("Microsoft").join("WinGet").join("Links");
        std::fs::create_dir_all(&winget).unwrap();
        let found = install_folders(Some(&home), Some(&appdata), Some(&local));
        assert!(
            found.contains(&home.join(".local").join("bin")),
            "{found:?}"
        );
        if cfg!(windows) {
            assert_eq!(found[0], appdata.join("npm"));
            // winget's links (Copilot CLI by winget install, ESC-186).
            assert!(found.contains(&winget), "{found:?}");
            // A prefix of its own: that one, when it is there.
            let own = dir.join("npm-global");
            std::fs::create_dir_all(&own).unwrap();
            std::fs::create_dir_all(&home).unwrap();
            std::fs::write(home.join(".npmrc"), format!("prefix={}\n", own.display())).unwrap();
            assert_eq!(
                install_folders(Some(&home), Some(&appdata), Some(&local))[0],
                own
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_path_of_the_registry_is_read_with_its_variables_expanded() {
        // Read only: the machine's PATH has System32, expanded from %SystemRoot%.
        let read = system();
        assert!(
            read.iter()
                .any(|p| p.to_string_lossy().to_lowercase().ends_with("system32")),
            "{read:?}"
        );
        assert!(
            read.iter().all(|p| !p.to_string_lossy().contains('%')),
            "{read:?}"
        );
    }
}
