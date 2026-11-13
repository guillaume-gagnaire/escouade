//! Where the `claude` of a setting, or of the system, is.

#[cfg(test)]
use crate::program::Located;
use crate::program::Program;
use escouade_model::wsl;
use std::path::{Path, PathBuf};

/// Locates the `claude` executable: explicit setting, PATH, then the native installer location;
/// under Windows, then one installed in WSL (`wsl_distro`: the distribution of the settings,
/// empty for the default one). A setting may start with `~` (the home folder); under Windows it
/// may leave out `.exe` or `.cmd`, and name a program of WSL: `\\wsl.localhost\<distro>\…` (or
/// `\\wsl$\…`), a Linux path, or `~/…` that names no program of Windows.
pub fn resolve_binary(configured: &str, wsl_distro: &str) -> Option<Program> {
    let configured = configured.trim();
    if !configured.is_empty() {
        return resolve_in(configured, dirs::home_dir().as_deref(), |spec| {
            wsl::find(spec, wsl_distro)
        });
    }
    // Not from the tests: a developer's WSL would answer for a machine that has no claude.
    detect_native().map(Program::Native).or_else(|| {
        if cfg!(all(windows, not(any(test, feature = "testing")))) {
            wsl::detect(wsl_distro).map(Program::Wsl)
        } else {
            None
        }
    })
}

/// `resolve_binary`, told as `Located`.
#[cfg(test)]
fn locate(configured: &str, wsl_distro: &str) -> Located {
    Located::of(resolve_binary(configured, wsl_distro))
}

/// `resolve_binary` of a setting, `home` the home folder and `find_wsl` the look into WSL.
fn resolve_in(
    configured: &str,
    home: Option<&Path>,
    find_wsl: impl Fn(&wsl::Spec) -> Option<wsl::Program>,
) -> Option<Program> {
    let spec = if cfg!(windows) {
        wsl::parse(configured)
    } else {
        None
    };
    // The share is a program of Linux, which Windows can read but not start.
    if let Some(spec) = spec.as_ref().filter(|s| s.distro.is_some()) {
        return find_wsl(spec).map(Program::Wsl);
    }
    native(configured, home)
        .map(Program::Native)
        .or_else(|| find_wsl(spec.as_ref()?).map(Program::Wsl))
}

/// The program of the system a setting names: `~` for the home folder and, under Windows, its
/// extension left out.
fn native(configured: &str, home: Option<&Path>) -> Option<PathBuf> {
    let path = match configured.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
            // One separator throughout: the path shows under the field of the settings.
            let rest = rest.trim_start_matches(['/', '\\']);
            home?.join(rest.replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR))
        }
        _ => PathBuf::from(configured),
    };
    if path.exists() && !path.is_dir() {
        return Some(path);
    }
    if !cfg!(windows) || path.extension().is_some() {
        return None;
    }
    ["exe", "cmd"]
        .iter()
        .map(|ext| path.with_extension(ext))
        .find(|p| p.is_file())
}

/// `claude` where it usually is on this system: the PATH, then the installers' locations.
fn detect_native() -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) {
        &["exe", "cmd"]
    } else {
        &[""]
    };
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for ext in exts {
                let mut candidate = dir.join("claude");
                if !ext.is_empty() {
                    candidate.set_extension(ext);
                }
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let home = dirs::home_dir()?;
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        vec![
            home.join(".local").join("bin").join("claude.exe"),
            home.join(".claude").join("local").join("claude.exe"),
            home.join("AppData")
                .join("Roaming")
                .join("npm")
                .join("claude.cmd"),
        ]
    } else {
        // The native installer, the old local install, Homebrew (Apple Silicon, then Intel).
        vec![
            home.join(".local").join("bin").join("claude"),
            home.join(".claude").join("local").join("claude"),
            PathBuf::from("/opt/homebrew/bin/claude"),
            PathBuf::from("/usr/local/bin/claude"),
        ]
    };
    candidates.into_iter().find(|p| p.is_file())
}

/// What an installation of Claude Code by npm cut half-way left (ESC-133): npm takes the old
/// `claude` away, then puts the new one by a link written under a temporary name (`.claude-XXXX`
/// beside it); the old package is moved aside (`@anthropic-ai/.claude-code-XXXX`). Looked for in the
/// folders of the PATH and where npm puts its programs (Homebrew's, `/usr/local`, Windows'
/// `%APPDATA%\npm`). None when nothing is left.
pub fn interrupted_install() -> Option<PathBuf> {
    let mut bins: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if cfg!(windows) {
        bins.extend(dirs::data_dir().map(|d| d.join("npm")));
    } else {
        bins.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    leftover_in(&bins)
}

/// `interrupted_install` in the folders of programs `bins`.
pub fn leftover_in(bins: &[PathBuf]) -> Option<PathBuf> {
    let starting = |dir: &Path, prefix: &str| -> Option<PathBuf> {
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .map(|e| e.path())
    };
    bins.iter().find_map(|bin| {
        starting(bin, ".claude-").or_else(|| {
            // npm's packages: beside its programs under Windows, in ../lib on Unix.
            let modules = if cfg!(windows) {
                bin.join("node_modules")
            } else {
                bin.parent()?.join("lib").join("node_modules")
            };
            starting(&modules.join("@anthropic-ai"), ".claude-code-")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_an_npm_installation_cut_half_way_left_is_found() {
        let root = escouade_base::testing::test_dir("claude-leftover");
        let bin = if cfg!(windows) {
            root.join("npm")
        } else {
            root.join("bin")
        };
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("node"), "").unwrap();
        // A folder of programs with nothing of Claude Code's, or a whole one: nothing left.
        assert_eq!(leftover_in(std::slice::from_ref(&bin)), None);
        std::fs::write(bin.join("claude"), "").unwrap();
        assert_eq!(leftover_in(std::slice::from_ref(&bin)), None);
        // The old package moved aside, never put back.
        let scope = if cfg!(windows) {
            bin.join("node_modules")
        } else {
            root.join("lib").join("node_modules")
        }
        .join("@anthropic-ai");
        std::fs::create_dir_all(scope.join("claude-code")).unwrap();
        assert_eq!(leftover_in(std::slice::from_ref(&bin)), None);
        std::fs::create_dir_all(scope.join(".claude-code-2DTsDk1V")).unwrap();
        assert_eq!(
            leftover_in(std::slice::from_ref(&bin)),
            Some(scope.join(".claude-code-2DTsDk1V"))
        );
        // The link under its temporary name, as on the client's Mac (/opt/homebrew/bin).
        std::fs::remove_file(bin.join("claude")).unwrap();
        std::fs::write(bin.join(".claude-Xa3k9"), "").unwrap();
        // A folder of programs looked at first, apart: on Unix, its packages are in its own ../lib.
        let other = root.join("other").join("bin");
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(
            leftover_in(&[other, bin.clone()]),
            Some(bin.join(".claude-Xa3k9"))
        );
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
    fn reads_the_settings_forms_of_the_issue() {
        let dir = escouade_base::testing::test_dir("claude-resolve");
        let bin = dir.join(".local").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        };
        std::fs::write(bin.join(exe), "").unwrap();
        let in_wsl = |spec: &wsl::Spec| {
            (spec.path == "~/.local/bin/claude-linux"
                || spec.path == "/home/me/.local/bin/claude"
                || spec.distro.as_deref() == Some("Ubuntu-24.04"))
            .then(ubuntu_claude)
        };
        let resolve = |configured: &str| resolve_in(configured, Some(&dir), in_wsl);
        // `~` is the home folder (also outside Windows); a folder is no program.
        assert_eq!(resolve("~/.local/bin"), None);
        assert_eq!(
            resolve(&format!("~/.local/bin/{exe}")),
            Some(Program::Native(bin.join(exe)))
        );
        if cfg!(windows) {
            // Under Windows, `.exe` may be left out: the native installer's `claude.exe`.
            assert_eq!(
                resolve(r"~\.local\bin\claude"),
                Some(Program::Native(bin.join("claude.exe")))
            );
            assert_eq!(
                resolve(&bin.join("claude").to_string_lossy()),
                Some(Program::Native(bin.join("claude.exe")))
            );
            // Shown with one separator throughout, however it was typed.
            let Some(Program::Native(shown)) = resolve("~/.local/bin/claude.exe") else {
                panic!("not found");
            };
            assert!(!shown.to_string_lossy().contains('/'), "{shown:?}");
            // The three forms of the issue name a program of WSL.
            let wsl = Some(Program::Wsl(ubuntu_claude()));
            assert_eq!(
                resolve(r"\\wsl.localhost\Ubuntu-24.04\home\me\.local\bin\claude"),
                wsl
            );
            assert_eq!(resolve("~/.local/bin/claude-linux"), wsl);
            assert_eq!(resolve("/home/me/.local/bin/claude"), wsl);
            assert_eq!(resolve(r"\\wsl.localhost\Debian\usr\bin\claude"), None);
        } else {
            assert_eq!(resolve("~/.local/bin/claude-linux"), None);
        }
        assert_eq!(resolve(&dir.join("absent").to_string_lossy()), None);
    }

    #[test]
    fn tells_where_the_claude_of_a_setting_is() {
        let dir = escouade_base::testing::test_dir("claude-locate");
        let exe = dir.join(if cfg!(windows) {
            "claude.cmd"
        } else {
            "claude"
        });
        std::fs::write(&exe, "").unwrap();
        assert_eq!(
            locate(&exe.to_string_lossy(), ""),
            Located {
                path: Some(exe.display().to_string()),
                wsl: None,
                nat: false,
                can_wsl: cfg!(windows)
            }
        );
        assert_eq!(
            locate(&dir.join("absent").to_string_lossy(), ""),
            Located {
                can_wsl: cfg!(windows),
                ..Default::default()
            }
        );
        // One of WSL, where WSL has a distribution: any program of it will do.
        let machine = wsl::Machine::this();
        if let (Some(_), Some(distro)) = (machine.exe, machine.default) {
            assert_eq!(
                locate("/usr/bin/env", ""),
                Located {
                    path: Some("/usr/bin/env".into()),
                    wsl: Some(distro),
                    nat: !wsl::mirrored_here(),
                    can_wsl: true
                }
            );
        }
    }
}
