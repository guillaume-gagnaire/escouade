use std::path::{Path, PathBuf};

// The app was « Claude Code Manager » until 0.1.3: what it keeps on disk moves to its new names
// on the first launch that finds it under the old ones.

pub const DATA_DIR_NAME: &str = ".escouade";
const OLD_DATA_DIR_NAME: &str = ".claude-code-manager";
/// The app identifier (tauri.conf.json), which names its WebView and window-state folders, and its
/// entries in the system's keychain.
pub(crate) const IDENTIFIER: &str = "dev.gagnaire.escouade";
const OLD_IDENTIFIER: &str = "dev.gagnaire.claude-code-manager";
/// A project's « Préfixe des branches des agents » until it sets its own (agents made before 0.1.4
/// keep `ccm/`).
pub const AGENT_BRANCH_PREFIX: &str = "escouade/";
/// A project's « Préfixe des branches des tickets » until it sets its own.
pub const TICKET_BRANCH_PREFIX: &str = "ticket/";
/// Where a project's agents' worktrees go, from its folder, until it sets its own « Dossier des
/// worktrees »: Claude Code's own (`claude --worktree`).
pub const DEFAULT_WORKTREES: &str = ".claude/worktrees";

/// The folder of the agents' worktrees of the project at `project`: its « Dossier des worktrees »
/// `setting` (quotes around it taken away, as « Copier en tant que chemin » gives it), from its
/// folder when relative, from the user's home folder after `~`, `.` and `..` taken away as
/// written (`../app-worktrees` is a sibling of the project's folder, whatever links are on the
/// way); `.claude/worktrees` of it when empty.
pub fn worktrees_root(project: &str, setting: &str) -> PathBuf {
    use std::path::Component;
    let setting = unquoted_setting(setting);
    let setting = if setting.is_empty() {
        DEFAULT_WORKTREES
    } else {
        setting
    };
    let full = match setting.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => dirs::home_dir()
            .unwrap_or_default()
            .join(rest.trim_start_matches(['/', '\\'])),
        _ => Path::new(project).join(setting),
    };
    let mut out = PathBuf::new();
    for c in full.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// A « Dossier des worktrees » as typed, trimmed, without the quotes around it.
fn unquoted_setting(setting: &str) -> &str {
    let s = setting.trim();
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
        .trim()
}

/// Why `setting` cannot be the « Dossier des worktrees » of the project at `project`, if it cannot.
pub fn worktrees_root_problem(project: &str, setting: &str) -> Option<String> {
    worktrees_root_problem_in(crate::i18n::ui(), project, setting)
}

fn worktrees_root_problem_in(
    lang: crate::i18n::Lang,
    project: &str,
    setting: &str,
) -> Option<String> {
    use std::path::Component;
    // Under Windows, `D:wt` (from D:'s current folder) and `\wt` (from the current drive's root)
    // depend on what the app's process happens to be in: neither relative to the project nor
    // whole.
    let mut parts = Path::new(unquoted_setting(setting)).components();
    let partial = match (parts.next(), parts.next()) {
        (Some(Component::Prefix(_)), Some(Component::RootDir)) => false,
        (Some(Component::Prefix(_)), _) => true,
        (Some(Component::RootDir), _) => cfg!(windows),
        _ => false,
    };
    if partial {
        return Some(tr_in!(
            lang,
            "Écris un chemin relatif au dossier du projet, ou un chemin complet (C:\\…).",
            "Write a path relative to the project’s folder, or a full one (C:\\…)."
        ));
    }
    // Compared as the disk names them: case ignored on Windows and macOS.
    let names = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| c.as_os_str().to_string_lossy().replace('\\', "/"))
            .map(|n| {
                if cfg!(any(windows, target_os = "macos")) {
                    n.to_lowercase()
                } else {
                    n
                }
            })
            .collect()
    };
    let root = names(&worktrees_root(project, setting));
    let own = names(&worktrees_root(project, "."));
    if root == own {
        return Some(tr_in!(
            lang,
            "Le dossier des worktrees ne peut pas être celui du projet.",
            "The worktrees folder can’t be the project’s own folder."
        ));
    }
    // Its parent (`..`) or higher: a worktree would sit beside the project's folder, in a
    // repository's folder the project is part of, or would hide it from its status.
    if own.starts_with(&root) {
        return Some(tr_in!(
            lang,
            "Le dossier des worktrees ne peut pas contenir celui du projet.",
            "The worktrees folder can’t hold the project’s folder."
        ));
    }
    // Git takes `.GIT` for its own folder on any disk that ignores case: refused everywhere.
    if root.iter().any(|n| n.eq_ignore_ascii_case(".git")) {
        return Some(tr_in!(
            lang,
            "Le dossier des worktrees ne peut pas être dans un dossier .git.",
            "The worktrees folder can’t be in a .git folder."
        ));
    }
    None
}

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

/// The window of a sandboxed run kept out of the user's way (end-to-end tests run while they work):
/// `$ESCOUADE_HIDDEN_WINDOW` set, which only a sandboxed run hears. The window opens off every
/// screen, never takes the focus, has no button in the taskbar, and the app no tray icon.
pub fn hidden_window() -> bool {
    hidden_window_from(|k| std::env::var_os(k))
}

fn hidden_window_from(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> bool {
    sandbox_dir_from(&var).is_some()
        && var("ESCOUADE_HIDDEN_WINDOW").is_some_and(|v| !v.is_empty() && v != "0")
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

/// The name of the folder of a ticket's files that are no image, in the ticket's own. No dot in
/// it: the images of a ticket are files of that folder too, named `<id>.<extension>`.
pub const TICKET_FILES: &str = "files";

/// Layout of the app's data folder. Injected so tests run on throwaway folders.
#[derive(Debug, Clone)]
pub struct DataDir(PathBuf);

impl DataDir {
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    /// The folder itself.
    pub fn root(&self) -> &Path {
        &self.0
    }

    pub fn state_file(&self) -> PathBuf {
        self.0.join("state.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.0.join("settings.json")
    }

    /// The accounts of the external ticket systems; their secrets are in the system's keychain,
    /// unless it refused them (`integrations::secrets`).
    pub fn integrations_file(&self) -> PathBuf {
        self.0.join("integrations.json")
    }

    /// The MCP server's token for Claude outside Escouade, while the system's keychain refuses it
    /// (`integrations::secrets::mcp_token`).
    pub fn mcp_token_file(&self) -> PathBuf {
        self.0.join("mcp-token.json")
    }

    /// The configuration folder of the agent's own harness (`Engine::agent_home_var`): Cursor's
    /// rules, model and sessions for that agent alone, kept while it lives.
    pub fn agent_home(&self, agent_id: &str) -> PathBuf {
        let name = if is_agent_folder_id(agent_id) {
            agent_id
        } else {
            "_"
        };
        self.0.join("agent-homes").join(name)
    }

    /// The installed plugins' copies, one folder per plugin, by its id (`plugins`).
    pub fn plugins_dir(&self) -> PathBuf {
        self.0.join("plugins")
    }

    /// The plugins' journals, one file per plugin, by its id (`plugins::runtime`).
    pub fn plugin_logs_dir(&self) -> PathBuf {
        self.0.join("plugin-logs")
    }

    /// The list of the installed plugins: where each comes from and its fingerprint.
    pub fn plugins_file(&self) -> PathBuf {
        self.0.join("plugins.json")
    }

    /// The MCP configs the agents' processes are started with (`--mcp-config`), one per agent.
    pub fn mcp_agents(&self) -> PathBuf {
        self.0.join("mcp")
    }

    /// The MCP config of the agent's process: Escouade's server, with the agent's own token.
    pub fn mcp_agent_config(&self, agent_id: &str) -> PathBuf {
        self.mcp_agents().join(format!("{agent_id}.json"))
    }

    /// The settings of the agent's process that tell it its fence (« Projet privé »,
    /// `Engine::fence_settings`), rewritten at each of its starts: never removed while it runs,
    /// its harness may read them again.
    pub fn fence_settings(&self, agent_id: &str) -> PathBuf {
        let name = if is_plain_id(agent_id) { agent_id } else { "_" };
        self.0.join("fence").join(format!("{name}.json"))
    }

    /// The settings an agent refused Escouade's tools is started with (`--settings`): Escouade's
    /// server kept out of it, whatever config declares it (`Engine::refusal`).
    pub fn mcp_refusal_file(&self) -> PathBuf {
        self.0.join("mcp-refused.json")
    }

    /// The syncs of imported tickets not through yet (`integrations::sync::SyncQueue`).
    pub fn sync_queue_file(&self) -> PathBuf {
        self.0.join("sync-queue.json")
    }

    pub fn conversations(&self) -> PathBuf {
        self.0.join("conversations")
    }

    /// The projects closed with their history kept, one file each (`closed::ClosedProject`).
    pub fn closed_projects(&self) -> PathBuf {
        self.0.join("closed-projects")
    }

    /// The file of the closed project `id`. An id is a name of its own (`new_id`): one that is not
    /// (a hand-edited file) never names a file outside the folder.
    pub fn closed_project_file(&self, id: &str) -> PathBuf {
        let name = if is_plain_id(id) { id } else { "_" };
        self.closed_projects().join(format!("{name}.json"))
    }

    /// The files of an open or closed project of this machine (its logo, `projectlogo`), a folder
    /// each. An id that is no name of its own never names another project's folder, nor one above.
    pub fn project_files(&self, project_id: &str) -> PathBuf {
        let name = if is_plain_id(project_id) {
            project_id
        } else {
            "_"
        };
        self.0.join("projects").join(name)
    }

    /// The code index of a project (`codeindex`): recomputed from its files, of this machine,
    /// never in the project.
    pub fn code_index_db(&self, project_id: &str) -> PathBuf {
        self.project_files(project_id).join("code-index.db")
    }

    /// The embedding model's folder (`vectors`, ESC-252): downloaded once for every project, of this machine.
    #[allow(dead_code)] // no caller until the Docs tickets
    pub fn vectors_model(&self, model_id: &str) -> PathBuf {
        let name = if is_plain_id(model_id) { model_id } else { "_" };
        self.0.join("vectors").join(name)
    }

    /// The index of a project's docs and their vectors: recomputed from the documents, so never in the project.
    #[allow(dead_code)] // idem
    pub fn vectors_index(&self, project_id: &str) -> PathBuf {
        self.project_files(project_id).join("vectors.db")
    }

    /// What the last reading of a project's workflows found (`workflow`), of this machine: never in
    /// the project, never versioned.
    pub fn workflows_file(&self, project_id: &str) -> PathBuf {
        self.project_files(project_id).join("workflows.json")
    }

    /// Where each agent of a project is in its workflows, and where it has been (`workflow_track`):
    /// kept with the agents, of this machine.
    pub fn workflow_tracks_file(&self, project_id: &str) -> PathBuf {
        self.project_files(project_id).join("workflow-tracks.json")
    }

    /// The tickets' files, a folder each: the images pasted in their form.
    pub fn tickets(&self) -> PathBuf {
        self.0.join("tickets")
    }

    /// The folder of the ticket's files; there only while it has some. An id is a name of its own
    /// (`new_id`): one of a hand-edited state that is not never names the folder of all the
    /// tickets, nor one above it, which go with the ticket.
    pub fn ticket_dir(&self, ticket_id: &str) -> PathBuf {
        let plain = !ticket_id.is_empty()
            && ticket_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        self.tickets().join(if plain { ticket_id } else { "_" })
    }

    /// The folder of the ticket's files that are no image (`<uniq>/<name>` in it), in its own
    /// folder: it goes with the ticket, and its agent's Claude Code reads it without a question
    /// (`--add-dir`).
    pub fn ticket_files(&self, ticket_id: &str) -> PathBuf {
        self.ticket_dir(ticket_id).join(TICKET_FILES)
    }

    /// The files joined to messages, a folder each for what owns them: an agent, or the drafts of
    /// the tickets' forms (`drafts_files`).
    pub fn attachments(&self) -> PathBuf {
        self.0.join("attachments")
    }

    /// The folder of the files joined to the agent's messages (`<uniq>/<name>` in it), readable by
    /// its Claude Code without a question (`--add-dir`). An id is a name of its own (`new_id`): one
    /// of a hand-edited state that is not (or that starts with `_`, the drafts' folder's first
    /// letter) never names the folder of another owner, nor one above it.
    pub fn agent_files(&self, agent_id: &str) -> PathBuf {
        let own = is_agent_folder_id(agent_id);
        self.attachments().join(if own { agent_id } else { "_" })
    }

    /// The files joined in a ticket's form, before it is saved.
    pub fn drafts_files(&self) -> PathBuf {
        self.attachments().join("_drafts")
    }

    /// The configuration folders of the Claude accounts made in the settings (all but Principal's),
    /// one per account: each its `CLAUDE_CONFIG_DIR`.
    pub fn claude_accounts(&self) -> PathBuf {
        self.0.join("claude")
    }

    /// The folders of the accounts of `engine` made in the settings: `claude/<name>`,
    /// `codex/<name>` (each its `CODEX_HOME`).
    pub fn accounts_of(&self, engine: crate::model::EngineId) -> PathBuf {
        match engine {
            crate::model::EngineId::Claude => self.claude_accounts(),
            e => self.0.join(e.as_str()),
        }
    }

    /// The script the sign-in of a Claude account opens its browser with, on Unix
    /// (`pty::open_browser_apart`).
    pub fn browser_opener(&self) -> PathBuf {
        self.0.join("open-browser")
    }

    pub fn stats_db(&self) -> PathBuf {
        self.0.join("stats.db")
    }

    pub fn log_file(&self) -> PathBuf {
        self.0.join("app.log")
    }

    /// The whole output of the last setup of the agent's worktree, step after step: the window
    /// only shows the last lines of the step running.
    pub fn setup_log(&self, agent_id: &str) -> PathBuf {
        self.0.join("logs").join(format!("{agent_id}-setup.log"))
    }

    /// The install's random id, for the usage statistics: there only while the user agrees to
    /// them (`telemetry`).
    pub fn telemetry_id_file(&self) -> PathBuf {
        self.0.join("telemetry-id")
    }

    /// The update the app stopped to install, for the window of its next start.
    pub fn update_note_file(&self) -> PathBuf {
        self.0.join("update.json")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.conversations())
    }
}

/// Whether the agent `id` has a folder of its own for its joined files (`DataDir::agent_files`): a
/// plain id that does not start with `_`.
pub fn is_agent_folder_id(id: &str) -> bool {
    is_plain_id(id) && !id.starts_with('_')
}

/// Whether `id` is a name of its own, as `new_id` makes them: letters, digits, `-` and `_`.
pub fn is_plain_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// What the spellings of one folder's path share (`same_folder`): separators, a trailing one,
/// the `\\?\` form Windows gives back, the case under Windows, the two names of a WSL share.
pub fn folder_key(path: &str) -> String {
    folder_key_in(path, cfg!(windows))
}

/// `folder_key` for Windows (`windows`) or for the other systems, whichever this one is.
pub(crate) fn folder_key_in(path: &str, windows: bool) -> String {
    let path = path.trim();
    // The extended forms are the plain ones.
    let path = match path.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => path.strip_prefix(r"\\?\").unwrap_or(path).to_string(),
    };
    // Windows takes `\` for a separator, the other systems for a letter of a name.
    let slashed = if windows {
        path.replace('\\', "/")
    } else {
        path
    };
    // Repeated and trailing separators are one and none; only a UNC path's leading two stay.
    let leading = match slashed.len() - slashed.trim_start_matches('/').len() {
        0 => "",
        2 => "//",
        _ => "/",
    };
    let body: Vec<&str> = slashed.split('/').filter(|part| !part.is_empty()).collect();
    let key = format!("{leading}{}", body.join("/"));
    // Both hosts reach the same distribution, which Linux names with a case that counts: only the
    // distribution's name is read without it.
    if let Some((distro, rest)) = crate::wsl::share_path(&key) {
        let rest = rest.trim_end_matches('/');
        return format!("//wsl.localhost/{}{rest}", distro.to_lowercase());
    }
    if windows {
        key.to_lowercase()
    } else {
        key
    }
}

/// The first of `items` whose folder (`folder_of`) is the folder `path`: the same key. Failing
/// that, the same one once the disk has spelled both (short names, links), but for the local
/// folders only: asking the disk about one on a share that is down, or in a WSL distribution that
/// is not started, can block for long, and this is asked at each key typed.
pub fn find_folder<'a, T>(
    items: &'a [T],
    path: &str,
    folder_of: impl Fn(&T) -> &str,
) -> Option<&'a T> {
    let key = folder_key(path);
    if let Some(found) = items.iter().find(|i| folder_key(folder_of(i)) == key) {
        return Some(found);
    }
    // The folder typed is spelt once; one that is not there, or not local, ends the search.
    if !is_local_key(&key) {
        return None;
    }
    let spelt = |path: &str| {
        let canonical = std::fs::canonicalize(path.trim()).ok()?;
        Some(folder_key(&canonical.to_string_lossy()))
    };
    let typed = spelt(path)?;
    items
        .iter()
        .filter(|i| is_local_key(&folder_key(folder_of(i))))
        .find(|i| spelt(folder_of(i)).is_some_and(|key| key == typed))
}

/// Whether the paths `a` and `b` name one folder (`find_folder` of one).
pub fn same_folder(a: &str, b: &str) -> bool {
    find_folder(&[b], a, |p| *p).is_some()
}

/// Whether a folder (by its `folder_key`) is on this machine's own disks: not on a share (`//…`),
/// WSL's included.
fn is_local_key(key: &str) -> bool {
    !key.starts_with("//")
}

/// Writes a file atomically (temp file + rename) so a crash never leaves a truncated file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// `write_atomic` for a file that may hold secrets, maybe their only copy: on disk before it takes
/// the old one's place, and on Unix readable by its owner only (0600) from the moment it is
/// written.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // The mode only applies to a new file: one left by a crash keeps its own, emptied.
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    f.write_all(bytes)?;
    f.sync_all()?;
    // Closed first: Windows does not move a file still open.
    drop(f);
    std::fs::rename(&tmp, path)
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
pub(crate) fn strip_base(base: &str, path: &str) -> Option<String> {
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

/// Cargo puts in the `PATH` of a test every folder a build script gives the linker, and that of
/// whisper-rs-sys gives each folder of whisper.cpp's cmake tree (46 of them): the `PATH` then
/// passes the 8191 characters cmd.exe reads, and the fake `claude.cmd` no longer finds `node`.
/// Those folders hold no program: they go, once, before a test starts anything.
#[cfg(all(test, windows))]
pub(crate) fn trim_test_path() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(path) = std::env::var_os("PATH") else {
            return;
        };
        let kept: Vec<PathBuf> = std::env::split_paths(&path)
            .filter(|p| !p.to_string_lossy().contains("whisper-rs-sys-"))
            .collect();
        if let Ok(joined) = std::env::join_paths(kept) {
            // Edition 2021: safe; done once, before the test spawns anything.
            std::env::set_var("PATH", joined);
        }
    });
}

#[cfg(test)]
pub fn test_dir(name: &str) -> PathBuf {
    #[cfg(windows)]
    trim_test_path();
    let dir = std::env::temp_dir().join(format!("ccm-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The real path (macOS: /private/var, not /var), as the processes started in it report it.
    if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap()
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
    fn the_worktrees_go_to_claude_worktrees_unless_the_project_says_where() {
        let (project, parent, abs) = if cfg!(windows) {
            (r"C:\code\app", r"C:\code", r"D:\worktrees\app")
        } else {
            ("/code/app", "/code", "/worktrees/app")
        };
        let root = |s| worktrees_root(project, s);
        let default = Path::new(project).join(".claude").join("worktrees");
        assert_eq!(root(""), default);
        assert_eq!(root("  "), default);
        assert_eq!(root(".worktrees"), Path::new(project).join(".worktrees"));
        // `..` taken away as written: a sibling of the project's folder.
        assert_eq!(
            root("../app-worktrees"),
            Path::new(parent).join("app-worktrees")
        );
        assert_eq!(root(" ./wt/../wt2/ "), Path::new(project).join("wt2"));
        assert_eq!(root(".."), PathBuf::from(parent));
        assert_eq!(root(abs), PathBuf::from(abs));
        // As « Copier en tant que chemin » gives it, and from the home folder after `~`.
        assert_eq!(
            worktrees_root(project, &format!("\"{abs}\"")),
            PathBuf::from(abs)
        );
        let home = dirs::home_dir().unwrap();
        assert_eq!(root("~/worktrees/app"), home.join("worktrees").join("app"));
        assert_eq!(root("~"), home);
        // A name that starts with `~` is a name.
        assert_eq!(root("~wt"), Path::new(project).join("~wt"));
        if cfg!(windows) {
            assert_eq!(
                root(r"..\app-worktrees"),
                Path::new(parent).join("app-worktrees")
            );
        }
    }

    #[test]
    fn the_worktrees_folder_is_neither_the_projects_nor_in_git() {
        use crate::i18n::Lang::{En, Fr};
        let project = if cfg!(windows) {
            r"C:\code\app"
        } else {
            "/code/app"
        };
        let problem = |s| worktrees_root_problem_in(En, project, s);
        for fine in [
            "",
            ".worktrees",
            "../app-worktrees",
            ".claude/worktrees",
            "~/worktrees/app",
            "\"../app-worktrees\"",
        ] {
            assert_eq!(problem(fine), None, "{fine}");
        }
        for itself in [".", "./", "sub/..", "../app", project, "\"../app\""] {
            assert_eq!(
                problem(itself).as_deref(),
                Some("The worktrees folder can’t be the project’s own folder."),
                "{itself}"
            );
        }
        // Its parent or higher: worktrees beside the project in its repository's folder, or the
        // project hidden from its own status.
        let root_of_disk = if cfg!(windows) { r"C:\" } else { "/" };
        for holding in ["..", "../..", "../../..", root_of_disk] {
            assert_eq!(
                problem(holding).as_deref(),
                Some("The worktrees folder can’t hold the project’s folder."),
                "{holding}"
            );
        }
        if cfg!(windows) {
            // From D:'s current folder, from the current drive's root: neither the project's nor whole.
            for partial in ["D:wt", r"\wt", "/wt"] {
                assert_eq!(
                    problem(partial).as_deref(),
                    Some("Write a path relative to the project’s folder, or a full one (C:\\…)."),
                    "{partial}"
                );
            }
            assert_eq!(problem(r"\\server\share\wt"), None);
        }
        for in_git in [".git", ".git/worktrees", "../app/.git/wt", "sub/.GIT/x"] {
            assert_eq!(
                problem(in_git).as_deref(),
                Some("The worktrees folder can’t be in a .git folder."),
                "{in_git}"
            );
        }
        assert_eq!(
            worktrees_root_problem_in(Fr, project, ".").as_deref(),
            Some("Le dossier des worktrees ne peut pas être celui du projet.")
        );
        assert_eq!(
            worktrees_root_problem_in(Fr, project, ".git").as_deref(),
            Some("Le dossier des worktrees ne peut pas être dans un dossier .git.")
        );
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
    fn data_dir_layout() {
        let d = DataDir::new(PathBuf::from("C:/data"));
        assert_eq!(d.state_file(), PathBuf::from("C:/data/state.json"));
        assert_eq!(d.conversations(), PathBuf::from("C:/data/conversations"));
        assert_eq!(
            d.setup_log("a1"),
            PathBuf::from("C:/data/logs/a1-setup.log")
        );
        assert_eq!(
            d.sync_queue_file(),
            PathBuf::from("C:/data/sync-queue.json")
        );
        assert_eq!(
            d.ticket_dir("3f2a9c01b7de"),
            PathBuf::from("C:/data/tickets/3f2a9c01b7de")
        );
    }

    #[test]
    fn a_tickets_folder_is_one_of_its_own_whatever_its_id() {
        // An id of a hand-edited state: its folder (removed with the ticket) is neither the one of
        // all the tickets nor one above it.
        let d = DataDir::new(PathBuf::from("C:/data"));
        for id in ["", ".", "..", "../..", "a/b", "..\\x", "C:"] {
            let dir = d.ticket_dir(id);
            assert_eq!(dir.parent(), Some(d.tickets().as_path()), "{id:?}");
            assert_ne!(dir, d.tickets(), "{id:?}");
            assert!(
                dir.file_name().is_some_and(|n| n != ".." && n != "."),
                "{id:?}"
            );
        }
    }

    #[test]
    fn the_files_of_a_ticket_are_in_a_folder_of_its_own_that_goes_with_it() {
        let d = DataDir::new(PathBuf::from("C:/data"));
        assert_eq!(
            d.ticket_files("3f2a9c01b7de"),
            PathBuf::from("C:/data/tickets/3f2a9c01b7de/files")
        );
        // A doubtful id has the shared folder's, never one above it.
        assert_eq!(
            d.ticket_files("../x").parent(),
            Some(d.ticket_dir("../x").as_path())
        );
        // Not a name an image takes (a name, a dot, an extension): the two do not meet.
        assert!(!TICKET_FILES.contains('.'));
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
    fn only_a_sandboxed_run_may_hide_its_window() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            }
        };
        const SANDBOX: (&str, &str) = ("ESCOUADE_DATA_DIR", "C:/e");
        const HIDDEN: (&str, &str) = ("ESCOUADE_HIDDEN_WINDOW", "1");
        assert!(hidden_window_from(env(&[SANDBOX, HIDDEN])));
        assert!(hidden_window_from(env(&[("CCM_DATA_DIR", "C:/c"), HIDDEN])));
        // The user's own app never opens out of sight, whatever its environment.
        assert!(!hidden_window_from(env(&[HIDDEN])));
        assert!(!hidden_window_from(env(&[SANDBOX])));
        assert!(!hidden_window_from(env(&[
            SANDBOX,
            ("ESCOUADE_HIDDEN_WINDOW", "0")
        ])));
        assert!(!hidden_window_from(env(&[
            SANDBOX,
            ("ESCOUADE_HIDDEN_WINDOW", "")
        ])));
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
    fn the_spellings_of_a_windows_folder_share_one_key() {
        let key = |p: &str| folder_key_in(p, true);
        for spelling in [
            r"C:\Code\App",
            "c:/code/app",
            r"C:\Code\App\",
            "C:/CODE/app//",
            r"  c:\code\\app  ",
            r"\\?\C:\Code\App",
        ] {
            assert_eq!(key(spelling), "c:/code/app", "{spelling}");
        }
        assert_eq!(key(r"\\?\UNC\Server\Share\App"), "//server/share/app");
        assert_eq!(key(r"\\Server\Share\App\"), "//server/share/app");
        assert_ne!(key(r"C:\code\app"), key(r"C:\code\app2"));
        assert_ne!(key(r"C:\code\app"), key(r"D:\code\app"));
    }

    #[test]
    fn elsewhere_the_case_and_the_backslashes_of_a_name_are_kept() {
        let key = |p: &str| folder_key_in(p, false);
        assert_eq!(key("/home/me/App/"), "/home/me/App");
        assert_eq!(key("/home//me/App"), "/home/me/App");
        assert_ne!(key("/home/me/app"), key("/home/me/App"));
        // A backslash is a letter of a name there, not a separator.
        assert_eq!(key(r"/home/me/a\b"), r"/home/me/a\b");
        assert_ne!(key(r"/home/me/a\b"), key("/home/me/a/b"));
        assert_eq!(key("/"), "/");
        // The extended form is still the plain one.
        assert_eq!(key(r"\\?\C:\x"), r"C:\x");
    }

    #[test]
    fn the_two_names_of_a_wsl_share_are_one_folder_and_linux_keeps_its_case() {
        let key = |p: &str| folder_key_in(p, true);
        let app = "//wsl.localhost/ubuntu/home/Me/App";
        for spelling in [
            r"\\wsl$\Ubuntu\home\Me\App",
            r"\\wsl.localhost\ubuntu\home\Me\App\",
            r"\\WSL.LOCALHOST\UBUNTU\home\Me\App",
            r"\\?\UNC\wsl.localhost\Ubuntu\home\Me\App",
            "//wsl$/Ubuntu/home/Me/App",
        ] {
            assert_eq!(key(spelling), app, "{spelling}");
        }
        // Linux tells `Me` from `me`, and one distribution from another.
        assert_ne!(key(r"\\wsl$\Ubuntu\home\me\app"), app);
        assert_ne!(key(r"\\wsl$\Debian\home\Me\App"), app);
        assert_eq!(key(r"\\wsl$\Ubuntu"), "//wsl.localhost/ubuntu");
        assert_eq!(key(r"\\wsl$\Ubuntu\"), "//wsl.localhost/ubuntu");
        // Written with slashes, it is the same on any system.
        assert_eq!(folder_key_in("//wsl$/Ubuntu/home/Me/App", false), app);
    }

    #[test]
    fn this_systems_key_is_its_own() {
        assert_eq!(
            folder_key(r"C:\Code\App"),
            folder_key_in(r"C:\Code\App", cfg!(windows))
        );
    }

    #[test]
    fn two_spellings_of_one_folder_are_one_and_two_folders_are_two() {
        let dir = test_dir("paths-same-folder");
        let app = dir.join("app");
        std::fs::create_dir_all(&app).unwrap();
        let s = app.to_string_lossy().to_string();
        assert!(same_folder(&s, &s));
        assert!(same_folder(
            &s,
            &format!("{s}{}", std::path::MAIN_SEPARATOR)
        ));
        assert!(same_folder(&s, &s.replace('\\', "/")));
        assert!(same_folder(&format!(" {s} "), &s));
        if cfg!(windows) {
            assert!(same_folder(&s, &s.to_uppercase()));
        }
        assert!(!same_folder(&s, &dir.to_string_lossy()));
        // A link leads to the same folder, as a short name (8.3) does: the disk spells both.
        let link = dir.join("link");
        if make_dir_link(&app, &link) {
            assert!(same_folder(&s, &link.to_string_lossy()));
        }
        // Folders that are not there are told apart by their keys alone.
        let (x, y) = (dir.join("x"), dir.join("y"));
        assert!(!same_folder(&x.to_string_lossy(), &y.to_string_lossy()));
        assert!(same_folder(&x.to_string_lossy(), &x.to_string_lossy()));
    }

    #[test]
    fn a_closed_projects_file_is_in_its_folder_whatever_its_id() {
        let d = DataDir::new(PathBuf::from("data"));
        assert_eq!(
            d.closed_project_file("3f2a9c01b7de"),
            d.closed_projects().join("3f2a9c01b7de.json")
        );
        for id in ["", "..", "../x", "a/b", r"a\b", "C:x", "a.b"] {
            let file = d.closed_project_file(id);
            assert_eq!(file.parent().unwrap(), d.closed_projects(), "{id}");
            assert_eq!(file.file_name().unwrap(), "_.json", "{id}");
        }
        assert!(is_plain_id("a1-B_2"));
        assert!(!is_plain_id("") && !is_plain_id("a.b") && !is_plain_id("a b"));
    }
}
