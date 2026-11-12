//! The guards of the paths the app is handed: a path kept inside its folder (`contained`), one relative to another
//! (`relative_slash`), one that reaches no other machine (`safe_origin`).

use std::path::{Path, PathBuf};

/// A path, as Claude Code's frames tell it, that does not lead to another machine: Windows opens a
/// UNC path (`\\server\share\…`), a verbatim one (`\\?\UNC\…`) or a device (`\\.\…`) over the
/// network, and sends the credentials of the user along. False for any path that starts with two
/// separators, whatever their kind (`\\`, `//`, `\/`, `/\`) and whatever the platform this runs on
/// (the same text is on its way to another one, and a Linux path never begins so). True for
/// everything else, the empty path and a relative one included: they reach no other machine.
///
/// It is asked of the path as the frames say it, before any conversion: the path of a `claude`
/// under WSL (`/tmp/claude/…`) is turned into `\\wsl.localhost\<distribution>\…` by our own
/// `Wsl::windows_path`, which is a path to a machine we started, of a path already accepted here.
pub fn safe_origin(path: &str) -> bool {
    let mut chars = path.chars();
    !(matches!(chars.next(), Some('\\' | '/')) && matches!(chars.next(), Some('\\' | '/')))
}

/// Refused: `rel` leads out of the folder it is asked inside of.
pub fn outside(rel: &str) -> anyhow::Error {
    outside_in(crate::i18n::ui(), rel)
}

fn outside_in(lang: crate::i18n::Lang, rel: &str) -> anyhow::Error {
    anyhow::anyhow!(tr_in!(
        lang,
        "chemin hors du dossier : {rel}",
        "path outside the folder: {rel}"
    ))
}

/// `rel` inside `root`, refused when it could leave it: absolute or drive-prefixed paths, `..`,
/// or a symbolic link on the way that points outside. `rel` may not exist yet (a file to create).
pub fn contained(root: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    use std::path::Component;
    let out = || outside(rel);
    if rel.trim().is_empty() {
        return Err(out());
    }
    let rel_path = Path::new(rel);
    if !rel_path
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(out());
    }
    let full = root.join(rel_path);
    let real_root = std::fs::canonicalize(root)?;
    // The deepest part that exists, links resolved, must still be inside the root.
    let mut probe = Some(full.as_path());
    while let Some(p) = probe {
        if let Ok(real) = std::fs::canonicalize(p) {
            if !real.starts_with(&real_root) {
                return Err(out());
            }
            break;
        }
        // An entry exists (symlink or file) but can't be resolved (dangling link, permission error, etc.)
        if p.symlink_metadata().is_ok() {
            return Err(out());
        }
        probe = p.parent();
    }
    Ok(full)
}

/// Path relative to `base`, with forward slashes. Falls back to the input when unrelated.
/// Symbolic links are seen through (on macOS, a folder under /var is reported under
/// /private/var by the tools that resolve it).
pub fn relative_slash(base: &str, path: &str) -> String {
    if let Some(rel) = strip_base(base, path) {
        return rel;
    }
    let real = |s: &str| {
        std::fs::canonicalize(s)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    };
    if let Some(b) = real(base) {
        let p = real(path).unwrap_or_else(|| path.to_string());
        if let Some(rel) = strip_base(&b, &p).or_else(|| strip_base(&b, path)) {
            return rel;
        }
    }
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

/// `path` relative to `base` (forward slashes, case ignored) when it is inside it, without reading the disk.
/// Under Windows, `/mnt/<drive>/…` is read as `<drive>:/…`: a `claude` of WSL tells the files of
/// the agent's folder there (WSL's usual mount point, `wsl::mount_root`).
pub fn strip_base(base: &str, path: &str) -> Option<String> {
    let norm = |s: &str| {
        let s = s
            .trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_string();
        let drive = s
            .strip_prefix("/mnt/")
            .filter(|rest| {
                let b = rest.as_bytes();
                cfg!(windows)
                    && b.first().is_some_and(u8::is_ascii_alphabetic)
                    && (b.len() == 1 || b[1] == b'/')
            })
            .map(|rest| format!("{}:{}", &rest[..1], &rest[1..]));
        drive.unwrap_or(s)
    };
    let (b, p) = (norm(base), norm(path));
    // Compared lowercase character by character, and cut where the path's own characters end:
    // a letter's lowercase can be longer or shorter in UTF-8 (the Kelvin sign is `k`, `ẞ` is
    // `ß`), so the base's length in bytes is not where the rest of the path starts.
    let want: Vec<char> = b
        .chars()
        .flat_map(char::to_lowercase)
        .chain(['/'])
        .collect();
    let mut matched = 0;
    for (i, c) in p.char_indices() {
        for l in c.to_lowercase() {
            if want.get(matched) != Some(&l) {
                return None;
            }
            matched += 1;
        }
        if matched == want.len() {
            return Some(p[i + c.len_utf8()..].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{make_dir_link, test_dir};

    #[test]
    fn no_path_that_starts_with_two_separators_is_safe_on_any_platform() {
        for refused in [
            // UNC: a share of another machine.
            r"\\server\share\claude\s1\tasks\b1.output",
            "//server/share/claude/s1/tasks/b1.output",
            r"\/server/share/tasks/b1.output",
            r"/\server\share\tasks\b1.output",
            // Verbatim, with a drive or a UNC share.
            r"\\?\C:\Users\x\tasks\b1.output",
            r"\\?\UNC\server\share\tasks\b1.output",
            "//?/C:/Users/x/tasks/b1.output",
            // Devices.
            r"\\.\pipe\claude",
            r"\\.\COM1",
            r"\\",
            "//",
            r"\\wsl.localhost\Ubuntu\tmp\claude\tasks\b1.output",
        ] {
            assert!(!safe_origin(refused), "{refused:?}");
        }
        for safe in [
            r"C:\Users\x\AppData\Local\Temp\claude\s1\tasks\b1.output",
            "C:/Users/x/AppData/Local/Temp/claude/s1/tasks/b1.output",
            r"C:\Users\Jean Dupont\tasks\b1.output",
            "/tmp/claude/s1/tasks/b1.output",
            "/tasks/b1.output",
            // One separator is the root of the current drive, not another machine.
            r"\tasks\b1.output",
            // Relative, or nothing: no machine to reach (the callers drop an empty path anyway).
            "tasks/b1.output",
            r"tasks\b1.output",
            "b1.output",
            "",
            // Two separators that do not start it.
            r"C:\\Users\x\tasks\b1.output",
            "/tmp//claude/tasks/b1.output",
        ] {
            assert!(safe_origin(safe), "{safe:?}");
        }
    }

    #[test]
    fn a_path_leaving_its_folder_is_refused_in_english() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(
            outside_in(En, "../x").to_string(),
            "path outside the folder: ../x"
        );
        assert_eq!(
            outside_in(Fr, "../x").to_string(),
            "chemin hors du dossier : ../x"
        );
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_slash("C:\\code\\app", "C:\\code\\app\\src\\a.ts"),
            "src/a.ts"
        );
        assert_eq!(
            relative_slash("C:/code/app/", "c:/Code/App/README.md"),
            "README.md"
        );
        assert_eq!(relative_slash("C:/code/app", "D:/other/x"), "D:/other/x");
    }

    #[test]
    fn relative_paths_whatever_the_length_of_their_letters_in_lowercase() {
        // The Kelvin sign (3 bytes) is `k` (1 byte) in lowercase, `ẞ` (3 bytes) is `ß` (2 bytes).
        let rel = |base, path| strip_base(base, path);
        assert_eq!(
            rel("C:/code/\u{212A}elvin", "C:/code/kelvin/src/a.ts").as_deref(),
            Some("src/a.ts")
        );
        assert_eq!(
            rel("C:/code/kelvin", "C:/code/\u{212A}elvin/src/a.ts").as_deref(),
            Some("src/a.ts")
        );
        assert_eq!(
            rel("C:/Stra\u{1E9E}e", "c:/straße/x.ts").as_deref(),
            Some("x.ts")
        );
        // The base's length in bytes falls inside a character of the path.
        assert_eq!(rel("C:/\u{212A}", "C:/k/€€.ts").as_deref(), Some("€€.ts"));
        assert_eq!(rel("C:/code/kelvin", "C:/code/kelvinx/a.ts"), None);
        assert_eq!(rel("C:/code/kelvin", "C:/code/kelvin"), None);
    }

    #[cfg(windows)]
    #[test]
    fn the_paths_of_a_claude_of_wsl_are_read_under_their_drive() {
        // A `claude` of WSL tells the files of the agent's folder under /mnt/<drive>/.
        assert_eq!(
            strip_base(r"C:\code\app", "/mnt/c/code/app/src/a.ts").as_deref(),
            Some("src/a.ts")
        );
        assert_eq!(
            relative_slash(r"C:\code\App", "/mnt/c/code/app/README.md"),
            "README.md"
        );
        assert_eq!(strip_base(r"D:\work", "/mnt/d/work/x"), Some("x".into()));
        assert_eq!(strip_base(r"C:\code\app", "/mnt/d/code/app/x"), None);
        // Anything else of Linux stays as it is.
        assert_eq!(
            relative_slash(r"C:\code\app", "/home/me/.claude/plans/p.md"),
            "/home/me/.claude/plans/p.md"
        );
        assert_eq!(strip_base(r"C:\mnt\wslg", "/mnt/wslg/x"), None);
    }

    #[cfg(unix)]
    #[test]
    fn relative_paths_see_through_symbolic_links() {
        let real = test_dir("relative-real");
        std::fs::create_dir_all(real.join("src")).unwrap();
        std::fs::write(real.join("src").join("a.ts"), "").unwrap();
        let link = real.with_file_name(format!(
            "{}-link",
            real.file_name().unwrap().to_string_lossy()
        ));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let file = real.join("src").join("a.ts");
        assert_eq!(
            relative_slash(&link.to_string_lossy(), &file.to_string_lossy()),
            "src/a.ts"
        );
        let new = real.join("src").join("new.ts");
        assert_eq!(
            relative_slash(&link.to_string_lossy(), &new.to_string_lossy()),
            "src/new.ts"
        );
    }

    #[test]
    fn keeps_a_relative_path_inside_its_root() {
        let root = test_dir("contained-ok");
        std::fs::create_dir_all(root.join("src")).unwrap();
        assert_eq!(
            contained(&root, "src/app.ts").unwrap(),
            root.join("src/app.ts")
        );
        // Not there yet (a file to create): still inside.
        assert_eq!(
            contained(&root, "new/deep/b.ts").unwrap(),
            root.join("new/deep/b.ts")
        );
        assert_eq!(contained(&root, "./a.ts").unwrap(), root.join("./a.ts"));
    }

    #[test]
    fn refuses_paths_that_leave_their_root() {
        let root = test_dir("contained-out");
        for bad in ["../x", "src/../../x", "", "/etc/passwd"] {
            assert!(contained(&root, bad).is_err(), "{bad}");
        }
        if cfg!(windows) {
            for bad in [r"C:\Windows\win.ini", r"C:x", r"\\server\share\x", r"..\x"] {
                assert!(contained(&root, bad).is_err(), "{bad}");
            }
        }
    }

    #[test]
    fn refuses_a_link_that_points_outside() {
        let root = test_dir("contained-link");
        let outside = test_dir("contained-link-target");
        let made = make_dir_link(&outside, &root.join("out"));
        if !made {
            eprintln!("Could not create link/junction (skipping test)");
            return;
        }
        assert!(contained(&root, "out/secret.txt").is_err());
    }

    #[test]
    fn refuses_a_link_that_cannot_be_resolved() {
        let root = test_dir("contained-dangling");
        let target = root.join("missing-target");
        let made = make_dir_link(&target, &root.join("dangling"));
        if !made {
            eprintln!("Could not create link/junction (skipping test)");
            return;
        }
        assert!(contained(&root, "dangling/file.txt").is_err());
    }
}
