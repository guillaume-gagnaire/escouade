use std::path::{Path, PathBuf};

// The app was « Claude Code Manager » until 0.1.3: what it keeps on disk moves to its new names
// on the first launch that finds it under the old ones.

pub const DATA_DIR_NAME: &str = ".escouade";
const OLD_DATA_DIR_NAME: &str = ".claude-code-manager";
/// The app identifier (tauri.conf.json), which names its WebView and window-state folders.
const IDENTIFIER: &str = "dev.gagnaire.escouade";
const OLD_IDENTIFIER: &str = "dev.gagnaire.claude-code-manager";
/// Prefix of the agents' worktree branches (agents made before 0.1.4 keep `ccm/`).
pub const BRANCH_PREFIX: &str = "escouade/";

/// Written into a folder moved to its new name: the migration is done.
const MIGRATED: &str = ".migrated-from-claude-code-manager";

/// What the migrations did, logged once the logger is up (they run before it).
static NOTES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub fn take_migration_notes() -> Vec<String> {
    std::mem::take(&mut *NOTES.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Moves `old` to `new` and returns the folder to use:
/// - nothing under `old`: `new`;
/// - `new` already migrated (it holds the marker): `new`, and `old` is a leftover;
/// - `new` there without the marker (made by a launch that could not move `old`, or by a
///   development or test run): it is set aside next to it, never deleted, and `old` takes its place;
/// - `old` cannot be moved (in use): `old`, and the next launch tries again.
pub fn migrate_dir(
    old: &Path,
    new: &Path,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
    notes: &mut Vec<String>,
) -> PathBuf {
    if !old.exists() || new.join(MIGRATED).exists() {
        return new.to_path_buf();
    }
    if new.exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = new.file_name().unwrap_or_default().to_string_lossy();
        let aside = new.with_file_name(format!("{name}.before-migration-{stamp}"));
        if let Err(e) = rename(new, &aside) {
            notes.push(format!(
                "{} not moved: {} is in the way ({e})",
                old.display(),
                new.display()
            ));
            return new.to_path_buf();
        }
        notes.push(format!(
            "{} set aside as {}",
            new.display(),
            aside.display()
        ));
    }
    match rename(old, new) {
        Ok(()) => {
            let _ = std::fs::write(new.join(MIGRATED), old.to_string_lossy().as_bytes());
            notes.push(format!("{} moved to {}", old.display(), new.display()));
            new.to_path_buf()
        }
        Err(e) => {
            notes.push(format!("{} not moved yet: {e}", old.display()));
            old.to_path_buf()
        }
    }
}

/// `rename`, tried `attempts` times `pause` apart: a closing WebView can still hold its files.
fn retrying(
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
    attempts: u32,
    pause: std::time::Duration,
) -> impl Fn(&Path, &Path) -> std::io::Result<()> {
    move |a, b| {
        let mut result = rename(a, b);
        for _ in 1..attempts {
            if result.is_ok() {
                break;
            }
            std::thread::sleep(pause);
            result = rename(a, b);
        }
        result
    }
}

fn rename_patiently(a: &Path, b: &Path) -> std::io::Result<()> {
    retrying(
        |a, b| std::fs::rename(a, b),
        10,
        std::time::Duration::from_millis(300),
    )(a, b)
}

/// A sandboxed data folder (end-to-end tests, demos): `$ESCOUADE_DATA_DIR`, or the former
/// `$CCM_DATA_DIR`.
pub fn sandbox_dir() -> Option<PathBuf> {
    sandbox_dir_from(|k| std::env::var_os(k))
}

fn sandbox_dir_from(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    ["ESCOUADE_DATA_DIR", "CCM_DATA_DIR"]
        .into_iter()
        .find_map(|k| var(k).filter(|d| !d.is_empty()))
        .map(PathBuf::from)
}

/// The data folder in `home`. A development build never moves the user's data (the installed app
/// may still be the old one): it uses the folder under the new name if there is one.
fn data_dir_in(home: &Path, debug: bool, notes: &mut Vec<String>) -> PathBuf {
    let (old, new) = (home.join(OLD_DATA_DIR_NAME), home.join(DATA_DIR_NAME));
    if debug {
        return if new.exists() || !old.exists() {
            new
        } else {
            old
        };
    }
    migrate_dir(&old, &new, rename_patiently, notes)
}

/// `~/.escouade` (moved from `~/.claude-code-manager`), or the sandboxed folder.
pub fn default_data_dir() -> PathBuf {
    if let Some(dir) = sandbox_dir() {
        return dir;
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut notes = NOTES.lock().unwrap_or_else(|e| e.into_inner());
    data_dir_in(&home, cfg!(debug_assertions), &mut notes)
}

/// Moves the WebView and window-state folders, named after the app identifier, to the new
/// identifier. Runs before the window is created; sandboxed runs and development builds leave
/// them alone.
pub fn migrate_app_folders() {
    if sandbox_dir().is_some() || cfg!(debug_assertions) {
        return;
    }
    let mut notes = NOTES.lock().unwrap_or_else(|e| e.into_inner());
    for base in [dirs::data_local_dir(), dirs::config_dir()]
        .into_iter()
        .flatten()
    {
        migrate_dir(
            &base.join(OLD_IDENTIFIER),
            &base.join(IDENTIFIER),
            rename_patiently,
            &mut notes,
        );
    }
}

/// Layout of the app's data folder. Injected so tests run on throwaway folders.
#[derive(Debug, Clone)]
pub struct DataDir(PathBuf);

impl DataDir {
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    pub fn state_file(&self) -> PathBuf {
        self.0.join("state.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.0.join("settings.json")
    }

    /// The accounts of the external ticket systems, with their secrets.
    pub fn integrations_file(&self) -> PathBuf {
        self.0.join("integrations.json")
    }

    pub fn conversations(&self) -> PathBuf {
        self.0.join("conversations")
    }

    pub fn stats_db(&self) -> PathBuf {
        self.0.join("stats.db")
    }

    pub fn log_file(&self) -> PathBuf {
        self.0.join("app.log")
    }

    /// The update the app stopped to install, for the window of its next start.
    pub fn update_note_file(&self) -> PathBuf {
        self.0.join("update.json")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.conversations())
    }
}

/// Writes a file atomically (temp file + rename) so a crash never leaves a truncated file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// `rel` inside `root`, refused when it could leave it: absolute or drive-prefixed paths, `..`,
/// or a symbolic link on the way that points outside. `rel` may not exist yet (a file to create).
pub fn contained(root: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    use std::path::Component;
    let out = || anyhow::anyhow!("chemin hors du dossier : {rel}");
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
pub(crate) fn strip_base(base: &str, path: &str) -> Option<String> {
    let norm = |s: &str| {
        s.trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_string()
    };
    let (b, p) = (norm(base), norm(path));
    let (bl, pl) = (b.to_lowercase(), p.to_lowercase());
    pl.starts_with(&(bl + "/"))
        .then(|| p[b.len() + 1..].to_string())
}

/// When this run made its first test folder: a fake CLI's log written before is an earlier run's.
#[cfg(test)]
static TESTS_START: std::sync::OnceLock<std::time::SystemTime> = std::sync::OnceLock::new();

#[cfg(test)]
pub fn test_dir(name: &str) -> PathBuf {
    let start = *TESTS_START.get_or_init(std::time::SystemTime::now);
    let dir = std::env::temp_dir().join(format!("ccm-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The real path (macOS: /private/var, not /var), as the processes started in it report it.
    let dir = if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap()
    };
    forget_fake_logs(&dir, start);
    dir
}

/// The fake `claude`, `gh` and `isola` keep their logs in the temporary folder, named after the folder they
/// ran in (every character but ASCII letters and digits replaced by `_`). Those of `dir` and its
/// subfolders that an earlier run left go: its process had the same id (Windows reuses them), and
/// a test would count their lines with its own. Those of this run stay: another test's folder may
/// read the same once encoded (`tk-env` is the start of `tk-env-push`).
#[cfg(test)]
fn forget_fake_logs(dir: &Path, start: std::time::SystemTime) {
    let key = |p: &Path| -> String {
        p.to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    };
    let mut keys = vec![key(dir)];
    // Its long name too (a temporary folder given by an 8.3 name), as a process may report it.
    if let Ok(real) = std::fs::canonicalize(dir) {
        let real = real.to_string_lossy().into_owned();
        let k = key(Path::new(real.trim_start_matches(r"\\?\")));
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let prefixes: Vec<String> = keys
        .iter()
        .flat_map(|k| {
            [
                format!("fake-claude-{k}"),
                format!("fake-gh-{k}"),
                format!("fake-isola-{k}"),
            ]
        })
        .collect();
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !prefixes.iter().any(|p| name.starts_with(p.as_str())) {
            continue;
        }
        let earlier = e
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| t < start);
        if earlier {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
pub fn make_dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        // Try symlink first (requires admin/developer mode)
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return true;
        }
        // Fall back to junction (no special privileges needed)
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
pub fn make_file_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn data_dir_layout() {
        let d = DataDir::new(PathBuf::from("C:/data"));
        assert_eq!(d.state_file(), PathBuf::from("C:/data/state.json"));
        assert_eq!(d.conversations(), PathBuf::from("C:/data/conversations"));
    }

    fn rename(a: &Path, b: &Path) -> std::io::Result<()> {
        std::fs::rename(a, b)
    }

    fn read(p: &Path) -> String {
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn a_folder_under_the_former_name_moves_to_the_new_one_and_is_marked() {
        let d = test_dir("migrate-move");
        let (old, new) = (d.join(".claude-code-manager"), d.join(".escouade"));
        std::fs::create_dir_all(old.join("conversations")).unwrap();
        std::fs::write(old.join("state.json"), "{}").unwrap();
        let mut notes = vec![];
        assert_eq!(migrate_dir(&old, &new, rename, &mut notes), new);
        assert!(!old.exists());
        assert_eq!(read(&new.join("state.json")), "{}");
        assert!(new.join("conversations").is_dir());
        assert!(new.join(MIGRATED).is_file());
        assert_eq!(notes.len(), 1, "{notes:?}");
    }

    #[test]
    fn a_migrated_folder_wins_over_a_leftover_under_the_former_name() {
        let d = test_dir("migrate-done");
        let (old, new) = (d.join("old"), d.join("new"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join(MIGRATED), "").unwrap();
        std::fs::write(old.join("state.json"), "old").unwrap();
        assert_eq!(migrate_dir(&old, &new, rename, &mut vec![]), new);
        assert_eq!(read(&old.join("state.json")), "old");
        assert!(!new.join("state.json").exists());
    }

    #[test]
    fn a_new_folder_made_without_migrating_is_set_aside_for_the_former_one() {
        // A launch that could not move the WebView folder, or a dev or test run, made `new`.
        let d = test_dir("migrate-aside");
        let (old, new) = (d.join("old"), d.join("new"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("state.json"), "old").unwrap();
        std::fs::write(new.join("state.json"), "fresh").unwrap();
        let mut notes = vec![];
        assert_eq!(migrate_dir(&old, &new, rename, &mut notes), new);
        assert_eq!(read(&new.join("state.json")), "old");
        assert!(!old.exists());
        let aside: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p != &new)
            .collect();
        assert_eq!(aside.len(), 1, "{aside:?}");
        assert_eq!(read(&aside[0].join("state.json")), "fresh");
        assert_eq!(notes.len(), 2, "{notes:?}");
    }

    #[test]
    fn nothing_to_move_on_a_first_run() {
        let d = test_dir("migrate-none");
        let (old, new) = (d.join("old"), d.join("new"));
        assert_eq!(migrate_dir(&old, &new, rename, &mut vec![]), new);
        assert!(!old.exists() && !new.exists());
    }

    #[test]
    fn a_folder_that_cannot_move_keeps_being_used() {
        let d = test_dir("migrate-fail");
        let (old, new) = (d.join("old"), d.join("new"));
        std::fs::create_dir_all(&old).unwrap();
        let locked = |_: &Path, _: &Path| Err(std::io::Error::other("in use"));
        let mut notes = vec![];
        assert_eq!(migrate_dir(&old, &new, locked, &mut notes), old);
        assert!(old.is_dir() && !new.exists());
        assert!(notes[0].contains("in use"), "{notes:?}");
    }

    #[test]
    fn a_move_is_tried_again_while_the_folder_is_busy() {
        let tries = std::cell::Cell::new(0);
        let busy_twice = |_: &Path, _: &Path| {
            tries.set(tries.get() + 1);
            if tries.get() < 3 {
                Err(std::io::Error::other("in use"))
            } else {
                Ok(())
            }
        };
        let rename = retrying(busy_twice, 5, std::time::Duration::ZERO);
        assert!(rename(Path::new("a"), Path::new("b")).is_ok());
        assert_eq!(tries.get(), 3);
        let never = retrying(
            |_: &Path, _: &Path| Err(std::io::Error::other("in use")),
            4,
            std::time::Duration::ZERO,
        );
        assert!(never(Path::new("a"), Path::new("b")).is_err());
    }

    #[test]
    fn a_development_build_never_moves_the_users_data() {
        let d = test_dir("migrate-debug");
        let (old, new) = (d.join(".claude-code-manager"), d.join(".escouade"));
        std::fs::create_dir_all(&old).unwrap();
        assert_eq!(data_dir_in(&d, true, &mut vec![]), old);
        assert!(old.is_dir() && !new.exists());
        std::fs::create_dir_all(&new).unwrap();
        assert_eq!(data_dir_in(&d, true, &mut vec![]), new);
        assert!(old.is_dir());
    }

    #[test]
    fn a_release_build_moves_the_users_data_once() {
        let d = test_dir("migrate-release");
        let old = d.join(".claude-code-manager");
        std::fs::create_dir_all(&old).unwrap();
        let new = data_dir_in(&d, false, &mut vec![]);
        assert_eq!(new, d.join(".escouade"));
        assert!(!old.exists());
        // The app asks twice at start-up (log, then data): the second time is a no-op.
        assert_eq!(data_dir_in(&d, false, &mut vec![]), new);
    }

    #[test]
    fn a_sandboxed_run_takes_the_new_variable_then_the_former_one() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            }
        };
        assert_eq!(
            sandbox_dir_from(env(&[
                ("ESCOUADE_DATA_DIR", "C:/e"),
                ("CCM_DATA_DIR", "C:/c")
            ])),
            Some(PathBuf::from("C:/e"))
        );
        assert_eq!(
            sandbox_dir_from(env(&[("CCM_DATA_DIR", "C:/c")])),
            Some(PathBuf::from("C:/c"))
        );
        assert_eq!(sandbox_dir_from(env(&[("ESCOUADE_DATA_DIR", "")])), None);
        assert_eq!(sandbox_dir_from(env(&[])), None);
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

    #[test]
    fn a_test_folder_starts_without_the_fake_clis_logs_an_earlier_run_left() {
        use std::time::{Duration, SystemTime};
        let dir = test_dir("paths-fake-logs");
        // As the fakes name their logs after the folder they run in.
        let key: String = dir
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        /// The files planted, gone with the test whatever its outcome.
        struct Planted(Vec<PathBuf>);
        impl Drop for Planted {
            fn drop(&mut self) {
                for p in &self.0 {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let mut planted = Planted(Vec::new());
        let tmp = std::env::temp_dir();
        let mut plant = |name: String, written: SystemTime| {
            let p = tmp.join(name);
            planted.0.push(p.clone());
            std::fs::write(&p, "{}\n").unwrap();
            std::fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_modified(written)
                .unwrap();
            p
        };
        let earlier = SystemTime::now() - Duration::from_secs(3600);
        // Left by an earlier run whose process had the same id: the folder's and its subfolders'.
        let stale = [
            plant(format!("fake-claude-{key}.stdin.jsonl"), earlier),
            plant(format!("fake-claude-{key}_sub.jsonl"), earlier),
            plant(
                format!("fake-gh-{key}_repo__claude_worktrees_dem_1.jsonl"),
                earlier,
            ),
        ];
        // Written by this run for another test, whose folder's name starts the same.
        let current = plant(format!("fake-claude-{key}_2_repo.jsonl"), SystemTime::now());
        // Not a log of this folder.
        let other = plant(
            format!(
                "fake-claude-elsewhere_{}_paths_fake_logs.jsonl",
                std::process::id()
            ),
            earlier,
        );
        assert_eq!(test_dir("paths-fake-logs"), dir);
        for p in &stale {
            assert!(!p.exists(), "{}", p.display());
        }
        assert!(current.exists() && other.exists());
    }
}
