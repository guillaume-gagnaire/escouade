//! Tauri commands invoked by the frontend.

use crate::accounts::{AccountStatus, MachineAccount, ShareMode};
use crate::attachfiles::AttachedFile;
use crate::closed::ClosedProjectInfo;
use crate::core::{AgentOptions, Attachment, Core, NotOnBase, SyncOp};
use crate::fsedit;
use crate::i18n::{self, Lang};
use crate::integrations::{
    Account, AccountView, Container, ExternalIssue, IssuePage, Query, StatesView,
};
use crate::model::Account as ClaudeAccount;
use crate::model::*;
use crate::pty::{self, ShellInfo, TermInfo, TermReport};
use crate::search;
use crate::stats::StatsView;
use crate::telemetry::{self, Event};
use crate::tickets::TicketDraft;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;

type Res<T> = Result<T, String>;

/// A command's result, counted in the usage statistics when it went well (`telemetry`).
fn counted<T, R: tauri::Runtime>(core: &Core<R>, event: Event, r: Res<T>) -> Res<T> {
    if r.is_ok() {
        core.telemetry.track(event);
    }
    r
}

/// The project a folder was added as, counted as created unless a closed project's history came
/// back with it (`Core::add_project`): that one is not new.
fn project_added<R: tauri::Runtime>(core: &Core<R>, r: Res<(Project, bool)>) -> Res<Project> {
    match r {
        Ok((project, false)) => counted(core, Event::ProjectCreated, Ok(project)),
        other => other.map(|(project, _)| project),
    }
}
type CoreState<'a> = State<'a, Arc<Core>>;

/// What the window is told of a refusal. One for want of an account of a harness (ESC-128) comes
/// as `no-account:<harness>|<text>`: the window shows the text with « Ajouter un compte… » on
/// that harness (`errorText`).
fn err(e: anyhow::Error) -> String {
    let text = format!("{e:#}");
    crate::accounts::marked(&e, text)
}

/// Refused: the shell `id` asked for is not one this system has.
fn shell_not_found(lang: Lang, id: &str) -> String {
    tr_in!(lang, "shell « {id} » introuvable", "shell “{id}” not found")
}

fn no_shell() -> String {
    tr!("aucun shell détecté", "no shell found")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    projects: Vec<Project>,
    tickets: Vec<Ticket>,
    agents: Vec<AgentView>,
    ui: UiState,
    settings: Settings,
    usage: UsageSnapshot,
    git: HashMap<String, GitInfo>,
    shells: Vec<ShellInfo>,
    terminals: Vec<TermInfo>,
    /// Claude Code is found for the account new agents go to.
    claude_found: bool,
    /// The settings' own path to Claude Code leads to it (what the field's hint says).
    claude_path_found: bool,
    version: String,
    models: Vec<ModelInfo>,
    /// The models the harnesses that list them told (Codex: `model/list`).
    engine_models: BTreeMap<crate::model::EngineId, Vec<crate::engine::registry::ToldModel>>,
    /// What the `claude` of each account told of its models (which take the mode `auto`).
    account_models: BTreeMap<String, crate::model::AccountModels>,
    /// What each harness declares: its capabilities, which the window reads to show a function.
    engines: Vec<crate::engine::registry::EngineDecl>,
    /// The harnesses whose program is found: the only ones the interface offers.
    engines_found: Vec<crate::model::EngineId>,
    /// Why no ticket of a project's board starts, by project.
    board_issues: HashMap<String, String>,
    /// Why no ticket of a project that goes to any account starts for now (a quota, a usage
    /// limit), if none does.
    autopilot_pause: Option<AutopilotPause>,
    /// The same for the projects that prefer an account, by project.
    project_pauses: BTreeMap<String, AutopilotPause>,
    /// The external ticket systems' accounts, as the window knows them.
    accounts: Vec<AccountView>,
    /// The update installed since the app's last start, told once (« Voir les nouveautés »).
    installed: Option<crate::updates::Installed>,
    /// The automatic restart planned for an update: when it comes.
    restart_at: Option<i64>,
    /// A version that did not install at the last try (the window offers to try again).
    failed_update: Option<String>,
    /// What the step running of each worktree setup under way wrote, its last lines, by agent.
    setup_output: HashMap<String, SetupOutput>,
    /// The languages of the interface, of the system and of the texts Claude writes.
    lang: crate::i18n::LangInfo,
}

// Off the main thread: finding a `claude` of WSL may start its distribution (seconds).
#[tauri::command(async)]
pub fn subscribe(
    core: CoreState,
    updates: State<crate::updates::Updates>,
    voice: VoiceState,
    channel: Channel<UiEvent>,
) -> InitialState {
    core.hub.set_channel(channel);
    // A page that loads again has no dictation under way: the one of the page before stops.
    voice.cancel();
    core.reset_unsaved();
    // One lock at a time, each guard ending with its statement: in a struct literal the guards
    // would live together (the lock order is projects before tickets, see `Core`).
    let settings = core.settings.read().clone();
    let projects = core.projects.read().clone();
    let tickets = core.tickets.read().clone();
    let agents = core.agent_views();
    let setup_output = core.setup_outputs();
    let ui = core.ui.read().clone();
    let usage = core.usage.lock().clone();
    let git = core.git_cache.read().clone();
    let terminals = core.pty.list();
    let models = core.models.read().clone();
    let engine_models = core.engine_models.read().clone();
    let account_models = core.account_models.read().clone();
    let board_issues = core.board_issues.lock().clone();
    // As it is now: the timer tells this window when it ends (and the board goes on then).
    let autopilot_pause = core.autopilot_pause();
    let project_pauses = core.project_pauses();
    let accounts = core.integration_accounts();
    let version = core.app.package_info().version.to_string();
    let lang = core.lang();
    let claude_path_found = crate::engine::default()
        .locate(&settings.claude_path, &settings.wsl_distro)
        .is_some();
    InitialState {
        lang,
        projects,
        tickets,
        agents,
        ui,
        shells: pty::detect_shells(&settings),
        claude_found: core.claude_found(),
        claude_path_found,
        settings,
        usage,
        git,
        terminals,
        installed: updates.take_installed(),
        restart_at: updates.restart_at(),
        failed_update: updates.failed(),
        version,
        models,
        engine_models,
        account_models,
        engines: crate::engine::registry::declared(),
        engines_found: core.engines_found(),
        board_issues,
        autopilot_pause,
        project_pauses,
        accounts,
        setup_output,
    }
}

#[tauri::command]
pub fn set_ui(core: CoreState, ui: UiState) {
    *core.ui.write() = ui;
    core.request_save();
}

/// The settings as the window saves them; the Claude accounts are left as they are (their tab
/// changes them with the `account_*` commands).
#[tauri::command(async)]
pub fn save_settings(
    core: CoreState,
    voice: VoiceState,
    settings: Settings,
) -> Res<Vec<ShellInfo>> {
    let remote_was_on = core.settings.read().remote_control_all;
    core.save_window_settings(settings.clone()).map_err(err)?;
    // A conversation under way takes the voice, its language, pace and volume (ESC-115).
    voice.conversation_tune(&conv_choice(&core.settings.read(), &*core.secrets, &voice.catalog).0);
    if settings.remote_control_all && !remote_was_on {
        core.telemetry
            .track(Event::RemoteControlEnabled { from: "global" });
    }
    Ok(pty::detect_shells(&settings))
}

// ---------- Harnesses ----------

/// Every harness with what is found of it here: its program, its version, its state (`registry`).
/// Asked by the user: versions are read again, and a harness installed since gets Escouade's
/// server (« Les agents peuvent piloter Escouade », `declare_in_harnesses`).
#[tauri::command]
pub async fn engines(core: CoreState<'_>) -> Res<Vec<crate::engine::registry::EngineInfo>> {
    crate::engine::registry::forget();
    let engines = core.engines().await;
    core.declare_in_harnesses();
    Ok(engines)
}

/// What stopped the sign-in of an account before it was signed in, when it is known (ESC-133).
#[tauri::command(async)]
pub fn account_sign_in_ended(core: CoreState, id: String) -> Option<String> {
    core.sign_in_ended(&id)
}

/// Where Escouade's update of Claude Code stands (ESC-133), for « Réglages › Harness ».
#[tauri::command(async)]
pub fn claude_update_status(core: CoreState) -> crate::selfupdate::Status {
    crate::selfupdate::status(&core.settings.read())
}

/// Where the `claude` of a path of the settings is (empty: the one found by itself), the
/// distribution of the settings (`wsl_distro`) for one of WSL: the line under the field.
#[tauri::command(async)]
pub fn claude_locate(path: String, wsl_distro: String) -> crate::engine::Located {
    // Asked by the user: what was not found is looked for again (maybe just installed).
    crate::wsl::forget(false);
    crate::engine::Located::of(crate::engine::default().locate(&path, &wsl_distro))
}

/// Whether Claude Code is found, as the start tells it (`claude_found`, `claude_path_found` of
/// `InitialState`): asked again once its path is saved.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeFound {
    claude_found: bool,
    claude_path_found: bool,
    engines_found: Vec<crate::model::EngineId>,
}

#[tauri::command(async)]
pub fn claude_found(core: CoreState) -> ClaudeFound {
    // Asked once a path is saved: every `claude` of WSL is looked for again (installed, removed).
    crate::wsl::forget(true);
    let settings = core.settings.read().clone();
    ClaudeFound {
        claude_found: core.claude_found(),
        engines_found: core.engines_found(),
        claude_path_found: crate::engine::default()
            .locate(&settings.claude_path, &settings.wsl_distro)
            .is_some(),
    }
}

// ---------- Claude accounts ----------

/// The items of the folder of the Principal of `engine` (Claude Code's by default) a new account
/// of it may share (those it has).
#[tauri::command(async)]
pub fn account_shareable(core: CoreState, engine: Option<EngineId>) -> Vec<String> {
    core.shareable(engine.unwrap_or(EngineId::Claude))
}

/// « Créer et se connecter »: a new account of `engine` (Claude Code by default), its folder made
/// and the items `share` of its Principal's linked or copied into it (`mode`), saved last in the
/// settings.
#[tauri::command(async)]
pub fn account_create(
    core: CoreState,
    name: String,
    share: Vec<String>,
    mode: ShareMode,
    engine: Option<EngineId>,
) -> Res<ClaudeAccount> {
    let engine = engine.unwrap_or(EngineId::Claude);
    let r = core
        .create_account(engine, &name, &share, mode)
        .map_err(err);
    counted(&core, Event::AccountCreated, r)
}

/// « Le compte de la machine » of `engine` added (its Principal, ESC-128).
#[tauri::command(async)]
pub fn account_add_machine(core: CoreState, engine: EngineId) -> Res<ClaudeAccount> {
    let r = core.add_machine_account(engine).map_err(err);
    counted(&core, Event::AccountCreated, r)
}

/// What « Ajouter un compte… » says of the machine's account of `engine`.
#[tauri::command]
pub async fn account_machine(core: CoreState<'_>, engine: EngineId) -> Res<MachineAccount> {
    Ok(core.machine_account(engine).await)
}

/// The account's name, `claude` and « Actif »; the accounts as they then are.
#[tauri::command(async)]
pub fn account_update(core: CoreState, account: ClaudeAccount) -> Res<Vec<ClaudeAccount>> {
    core.update_claude_account(account).map_err(err)
}

/// The accounts in the order of `ids`, as they then are.
#[tauri::command(async)]
pub fn account_reorder(core: CoreState, ids: Vec<String>) -> Res<Vec<ClaudeAccount>> {
    core.reorder_claude_accounts(&ids).map_err(err)
}

/// « Ordre des harness » set to `order`; the order as it then is.
#[tauri::command(async)]
pub fn engine_order_set(core: CoreState, order: Vec<EngineId>) -> Res<Vec<EngineId>> {
    core.set_engine_order(&order).map_err(err)
}

/// `model` of `engine` marked as a favorite (or no longer); the favorites as they then are.
#[tauri::command(async)]
pub fn favorite_model_set(
    core: CoreState,
    engine: EngineId,
    model: String,
    favorite: bool,
) -> Res<std::collections::BTreeMap<EngineId, Vec<String>>> {
    core.set_favorite_model(engine, &model, favorite)
        .map_err(err)
}

/// The account removed (its folder stays on the disk); the accounts as they then are.
#[tauri::command(async)]
pub fn account_remove(core: CoreState, id: String) -> Res<Vec<ClaudeAccount>> {
    core.remove_claude_account(&id).map_err(err)
}

/// Whether the account is signed in, with which email, and its folder.
#[tauri::command]
pub async fn account_status(core: CoreState<'_>, id: String) -> Res<AccountStatus> {
    core.claude_account_status(&id).await.map_err(err)
}

/// « Se connecter… »: the account's `claude` in an interactive terminal (written to, resized and
/// killed as the others: `term_write`, `term_resize`, `term_kill`).
#[tauri::command(async)]
pub fn account_login(
    core: CoreState,
    id: String,
    cols: u16,
    rows: u16,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    core.claude_login(&id, (cols, rows), move |bytes| {
        let _ = output.send(InvokeResponseBody::Raw(bytes));
    })
    .map_err(err)
}

/// « Vue Claude Code »: the agent runs in Claude Code's own interface, or in Escouade's view.
#[tauri::command]
pub async fn set_agent_view(core: CoreState<'_>, id: String, terminal: bool) -> Res<()> {
    core.set_agent_view(&id, terminal).await.map_err(err)?;
    if terminal {
        core.telemetry.track(Event::ClaudeCodeView);
    }
    Ok(())
}

/// The agent's Claude Code interface, in an interactive terminal (written to, resized and killed
/// as the others: `term_write`, `term_resize`, `term_kill`). `manual`: the user asked for it
/// (« Relancer »), as opposed to the window opening the view: an automatic start is refused once
/// the interface stopped as soon as it started twice in a row.
#[tauri::command]
pub async fn agent_terminal(
    core: CoreState<'_>,
    id: String,
    cols: u16,
    rows: u16,
    manual: Option<bool>,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    core.open_agent_terminal(&id, (cols, rows), manual.unwrap_or(false), move |bytes| {
        let _ = output.send(InvokeResponseBody::Raw(bytes));
    })
    .await
    .map_err(err)
}

/// An image pasted in the agent's Claude Code interface, written to a file: the path to type there.
#[tauri::command(async)]
pub fn paste_image(core: CoreState, id: String, media_type: String, data: String) -> Res<String> {
    core.paste_image(&id, &media_type, &data).map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInfo {
    exists: bool,
    is_repo: bool,
    branch: String,
    dirty: u32,
    name: String,
    /// The project closed in this folder, whose history adding a project there gives back.
    history: Option<ClosedProjectInfo>,
}

/// Always `Ok`: an async command that borrows the state must return a `Result`. `history_id`: the
/// closed project of this folder the window means to bring back, if it has several.
#[tauri::command]
pub async fn inspect_folder(
    core: CoreState<'_>,
    path: String,
    history_id: Option<String>,
) -> Res<FolderInfo> {
    let p = std::path::Path::new(path.trim());
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !p.is_dir() {
        return Ok(FolderInfo {
            exists: false,
            is_repo: false,
            branch: String::new(),
            dirty: 0,
            name,
            history: None,
        });
    }
    let path = path.trim().to_string();
    let history = core
        .closed_project_info_for(&path, history_id.as_deref())
        .await;
    Ok(match crate::git::toplevel(&path).await {
        Some(_) => {
            let st = crate::git::status(&path).await.unwrap_or_default();
            FolderInfo {
                exists: true,
                is_repo: true,
                branch: st.branch,
                dirty: st.entries.len() as u32,
                name,
                history,
            }
        }
        None => FolderInfo {
            exists: true,
            is_repo: false,
            branch: String::new(),
            dirty: 0,
            name,
            history,
        },
    })
}

/// What the new project's assistant checks in a folder before its settings are taken: whether its
/// tickets can start there (a commit to start from, the target branch), the folders its commands
/// run in, where its worktrees would go.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInfo {
    /// The folder is a repository with a commit: worktrees, so tickets, can start from it.
    has_commit: bool,
    /// Its local branches, the checked-out one first.
    branches: Vec<String>,
    /// Those of `dirs` (relative to the folder) that are no folder there.
    missing_dirs: Vec<String>,
    /// The worktrees folder `worktree_dir` would be one git tracks files in: none can go there.
    worktrees_tracked: bool,
}

/// Always `Ok`: an async command that borrows the state must return a `Result`.
#[tauri::command]
pub async fn inspect_setup(
    _core: CoreState<'_>,
    path: String,
    worktree_dir: String,
    dirs: Vec<String>,
) -> Res<SetupInfo> {
    let path = path.trim().to_string();
    let repo = crate::git::toplevel(&path).await.is_some();
    let missing_dirs = dirs
        .into_iter()
        .filter(|d| !std::path::Path::new(&path).join(d.trim()).is_dir())
        .collect();
    Ok(SetupInfo {
        has_commit: repo && crate::git::has_commit(&path).await,
        branches: if repo {
            crate::git::branches(&path).await.unwrap_or_default()
        } else {
            Vec::new()
        },
        missing_dirs,
        worktrees_tracked: repo
            && crate::git::worktrees_in_tracked_folder(
                &path,
                &crate::paths::worktrees_root(&path, &worktree_dir),
            )
            .await,
    })
}

/// « Faire un premier commit »: the folder made a repository if need be, with an empty first commit.
#[tauri::command]
pub async fn first_commit(_core: CoreState<'_>, path: String) -> Res<()> {
    crate::git::first_commit(path.trim()).await.map_err(err)
}

/// `restore`: the history of a project closed in this folder comes back with it (the default);
/// false starts a project of its own, the history staying among the closed projects.
/// `history_id`: which of the folder's histories comes back, when it has several (else the latest
/// closed).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_project(
    core: CoreState<'_>,
    path: String,
    name: String,
    color: String,
    worktree_per_agent: bool,
    first_agent: Option<String>,
    restore: Option<bool>,
    history_id: Option<String>,
) -> Res<Project> {
    let r = core
        .add_project(
            &path,
            &name,
            &color,
            worktree_per_agent,
            first_agent,
            restore.unwrap_or(true),
            history_id,
        )
        .await
        .map_err(err);
    project_added(&core, r)
}

#[tauri::command(async)]
pub fn update_project(core: CoreState, project: Project) -> Res<()> {
    let remote_was_on = core.project(&project.id).is_ok_and(|p| p.remote_control);
    let remote_on = project.remote_control;
    core.update_project(project).map_err(err)?;
    if remote_on && !remote_was_on {
        core.telemetry
            .track(Event::RemoteControlEnabled { from: "project" });
    }
    Ok(())
}

/// « Faire confiance à ces commandes »: those of the project's `.escouade.yml` the user read.
#[tauri::command(async)]
pub fn shared_trust(core: CoreState, project_id: String, hash: String) -> Res<Project> {
    core.trust_shared(&project_id, &hash).map_err(err)
}

/// « Exporter vers .escouade.yml » (`keys` none), or « Écrire dans .escouade.yml » of the keys
/// kept on this machine.
#[tauri::command(async)]
pub fn shared_write(
    core: CoreState,
    project_id: String,
    keys: Option<Vec<String>>,
) -> Res<Project> {
    core.write_shared(&project_id, keys).map_err(err)
}

/// « Reprendre la valeur du fichier »: a key kept on this machine follows the file again.
#[tauri::command(async)]
pub fn shared_unkeep(core: CoreState, project_id: String, key: String) -> Res<Project> {
    core.unkeep_shared(&project_id, &key).map_err(err)
}

/// The settings the `.escouade.yml` of a folder gives a new project (the assistant's start).
#[tauri::command(async)]
pub fn shared_preview(path: String) -> Option<Project> {
    crate::projectfile::preview(&path)
}

#[tauri::command(async)]
pub fn reorder_projects(core: CoreState, ids: Vec<String>) {
    core.reorder_projects(&ids);
}

/// `erase`: its history goes too; by default it is kept (`closed_projects`).
#[tauri::command]
pub async fn remove_project(core: CoreState<'_>, id: String, erase: Option<bool>) -> Res<()> {
    core.remove_project(&id, erase.unwrap_or(false))
        .await
        .map_err(err)
}

/// The projects closed with their history kept, the latest closed first.
#[tauri::command(async)]
pub fn closed_projects(core: CoreState) -> Vec<ClosedProjectInfo> {
    core.closed_projects()
}

/// « Effacer l'historique » of a closed project (the file of an unreadable one alone), away from
/// the async workers: it reads the history and removes its logs.
#[tauri::command]
pub async fn erase_closed_project(core: CoreState<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    tokio::task::spawn_blocking(move || core.erase_closed_project(&id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[tauri::command]
/// `isolated`: in a worktree of its own or not, whatever « Un worktree par agent » says (« Nouvel
/// agent dans un worktree », « sans worktree »).
pub async fn create_agent(
    core: CoreState<'_>,
    project_id: String,
    model: Option<String>,
    engine: Option<crate::model::EngineId>,
    isolated: Option<bool>,
    // « Nouvel agent depuis une autre branche… » (ESC-160): its worktree starts from it.
    base: Option<String>,
) -> Res<AgentView> {
    let agent = core
        .create_agent_with(
            &project_id,
            AgentOptions {
                model,
                engine,
                isolated,
                base: base.filter(|b| !b.trim().is_empty()),
                select: true,
                ..Default::default()
            },
        )
        .await
        .map_err(err)?;
    core.telemetry.track(Event::AgentCreated {
        // Made at its first message (ESC-156), it is one all the same.
        worktree: agent.meta.worktree.is_some() || agent.meta.worktree_pending,
    });
    Ok(agent)
}

#[tauri::command]
pub fn warm_agent(core: CoreState, id: String) {
    core.warm(&id);
}

/// « Relancer » (« Outils d'Escouade au prochain démarrage »): the agent's process, at rest,
/// started again on the same session.
#[tauri::command]
pub async fn restart_agent_process(core: CoreState<'_>, id: String) -> Res<()> {
    core.restart_process(&id).await.map_err(err)
}

#[tauri::command(async)]
pub fn get_conversation(core: CoreState, id: String) -> Res<Vec<Value>> {
    // Buffered deltas are already in the snapshot: send them before it, not after.
    core.flush_conv();
    let h = core.agent(&id).map_err(err)?;
    let items = h.lock().conv.items();
    Ok(items)
}

/// Searches the conversations (Ctrl+K) away from the window's threads: it reads every log.
#[tauri::command]
pub async fn search_conversations(
    core: CoreState<'_>,
    query: String,
    project_id: Option<String>,
    archived: bool,
) -> Res<crate::convsearch::Found> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.search_conversations(&query, project_id.as_deref(), archived)
    })
    .await
    .map_err(|e| e.to_string())
}

/// A message to the agent. `files`: the files joined by their path (`attachfiles`); a window that
/// does not send any (an older one) sends none.
#[tauri::command]
pub async fn send_message(
    core: CoreState<'_>,
    id: String,
    text: String,
    attachments: Vec<Attachment>,
    files: Option<Vec<AttachedFile>>,
) -> Res<()> {
    let r = core
        .send_message_files(&id, text, attachments, files.unwrap_or_default())
        .await
        .map_err(err);
    counted(&core, Event::MessageSent, r)
}

/// A file to copy for `owner` (an agent's id, or `"draft"`): its handle, for its chunks.
#[tauri::command(async)]
pub fn attach_open(core: CoreState, owner: String, name: String, size: u64) -> Res<String> {
    core.attach_open(&owner, &name, size).map_err(err)
}

/// The next bytes of a copy, as they are (`invoke('attach_chunk', bytes, { headers: { 'x-handle':
/// handle } })`: no base64 round trip through JSON).
#[tauri::command(async)]
pub fn attach_chunk(core: CoreState, request: tauri::ipc::Request) -> Res<()> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(tr!(
            "Les octets d’une copie viennent bruts, pas en JSON.",
            "The bytes of a copy come raw, not as JSON."
        ));
    };
    let handle = request
        .headers()
        .get("x-handle")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    core.attach_chunk(handle, bytes).map_err(err)
}

/// The end of a copy: the file, once all its bytes are there.
#[tauri::command(async)]
pub fn attach_finish(core: CoreState, handle: String) -> Res<AttachedFile> {
    core.attach_finish(&handle).map_err(err)
}

/// A copy given up: what was written goes.
#[tauri::command(async)]
pub fn attach_cancel(core: CoreState, handle: String) -> Res<()> {
    core.attach_cancel(&handle);
    Ok(())
}

/// A finished copy removed (its chip removed); never a path out of its owner's folder.
#[tauri::command(async)]
pub fn attach_discard(core: CoreState, owner: String, path: String) -> Res<()> {
    core.attach_discard(&owner, &path).map_err(err)
}

/// A file of the user's own, joined as it is: checked, nothing copied. `id` is the one the window
/// asked `file_origin` the path of, never a path: only a path the host itself read is joined.
#[tauri::command(async)]
pub fn attach_origin(core: CoreState, owner: String, id: String) -> Res<AttachedFile> {
    core.attach_origin(&owner, &id).map_err(err)
}

/// The image on the system's clipboard, as PNG (empty without one), for a paste WebKitGTK leaves
/// without it.
#[tauri::command]
pub async fn clipboard_image(app: tauri::AppHandle) -> Res<tauri::ipc::Response> {
    crate::clipboard::image_png(&app)
        .await
        .map(tauri::ipc::Response::new)
        .map_err(err)
}

#[tauri::command]
pub async fn interrupt(core: CoreState<'_>, id: String) -> Res<()> {
    core.interrupt(&id).await.map_err(err)
}

/// A message to a subagent of the agent `id`, by the agent (`call`: the call that launched it).
#[tauri::command]
pub async fn message_subagent(
    core: CoreState<'_>,
    id: String,
    call: String,
    text: String,
) -> Res<()> {
    core.message_subagent(&id, &call, &text).await.map_err(err)
}

/// « Arrêter » on a subagent of the agent `id`, `call` the call that launched it.
#[tauri::command]
pub async fn stop_subagent(core: CoreState<'_>, id: String, call: String) -> Res<()> {
    core.stop_subagent(&id, &call).await.map_err(err)
}

/// « Arrêter » on a task Claude Code runs in the background for the agent `id` (a command, a
/// monitor, a remote agent, a teammate: a `BgTask` of its plan; or a workflow): `stop_task`.
#[tauri::command]
pub async fn stop_bg_task(core: CoreState<'_>, id: String, task: String) -> Res<()> {
    core.stop_bg_task(&id, &task).await.map_err(err)
}

/// The end of the output of a background task of the agent `id`: 8 KiB at most while it runs (its
/// process's, else its output file's), up to 256 KiB of its output file once it is over.
#[tauri::command]
pub async fn task_output(
    core: CoreState<'_>,
    id: String,
    task: String,
) -> Res<crate::taskout::TaskOutput> {
    core.bg_task_output(&id, &task).await.map_err(err)
}

/// The conversation of a subagent of the agent `id` in the Claude Code view (`call` launched it),
/// read from its transcript: a file read, off the async workers. `cursor` is the one the read
/// before gave: null back when its files did not change since.
#[tauri::command]
pub async fn subagent_transcript(
    core: CoreState<'_>,
    id: String,
    call: String,
    cursor: Option<crate::agent::sidechain::Cursor>,
) -> Res<Option<crate::agent::sidechain::SidechainRead>> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.subagent_transcript(&id, &call, cursor))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[tauri::command(async)]
pub fn answer_question(core: CoreState, id: String, request_id: String, answers: Value) -> Res<()> {
    core.answer_question(&id, &request_id, answers).map_err(err)
}

#[tauri::command(async)]
pub fn answer_permission(
    core: CoreState,
    id: String,
    request_id: String,
    decision: String,
    message: Option<String>,
) -> Res<()> {
    core.answer_permission(&id, &request_id, &decision, message)
        .map_err(err)
}

#[tauri::command]
pub async fn set_agent_options(
    core: CoreState<'_>,
    id: String,
    model: Option<String>,
    effort: Option<String>,
    mode: Option<String>,
) -> Res<()> {
    core.set_agent_options(&id, model, effort, mode)
        .await
        .map_err(err)
}

/// The Claude account of an agent that has not started (the Composer's chip); empty is
/// « Automatique ».
#[tauri::command]
pub async fn set_agent_account(core: CoreState<'_>, id: String, account: String) -> Res<()> {
    core.set_agent_account(&id, &account).await.map_err(err)
}

/// « Continuer sur <harness> » and « Revenir sur <harness> »: the agent goes on on another harness,
/// by a handover (`handover`).
#[tauri::command]
pub async fn continue_on_engine(
    core: CoreState<'_>,
    agent_id: String,
    engine: EngineId,
) -> Res<()> {
    core.continue_on_engine(&agent_id, engine)
        .await
        .map_err(err)
}

/// « Reprendre sur <compte> »: the agent stopped by the usage limit goes on, with its session, on
/// another account.
#[tauri::command]
pub async fn resume_on_account(core: CoreState<'_>, agent_id: String, account: String) -> Res<()> {
    core.resume_on_account(&agent_id, &account)
        .await
        .map_err(err)
}

/// « Revenir sur <compte> »: a resume on another account failed; the agent goes back.
#[tauri::command]
pub async fn back_to_previous_account(core: CoreState<'_>, agent_id: String) -> Res<()> {
    core.back_to_previous_account(&agent_id).await.map_err(err)
}

/// Renames the agent, and its own branch after it: why the branch kept its name, when it did.
#[tauri::command]
pub async fn rename_agent(core: CoreState<'_>, id: String, name: String) -> Res<Option<String>> {
    core.rename_agent(&id, &name).await.map_err(err)
}

#[tauri::command]
pub async fn duplicate_agent(core: CoreState<'_>, id: String) -> Res<AgentView> {
    core.duplicate_agent(&id).await.map_err(err)
}

#[tauri::command]
pub async fn archive_agent(core: CoreState<'_>, id: String, archived: bool) -> Res<()> {
    core.archive_agent(&id, archived).await.map_err(err)
}

#[tauri::command]
pub async fn delete_agent(
    core: CoreState<'_>,
    id: String,
    remove_worktree: bool,
) -> Res<Option<String>> {
    core.delete_agent(&id, remove_worktree).await.map_err(err)
}

#[tauri::command]
pub async fn merge_agent(
    core: CoreState<'_>,
    id: String,
    squash: bool,
    switch_to_base: bool,
) -> Res<String> {
    let r = core
        .merge_agent(&id, squash, switch_to_base)
        .await
        .map_err(|e| match e.downcast_ref::<NotOnBase>() {
            // The one refusal the frontend turns into a question rather than an error.
            Some(refused) => refused.wire(),
            None => err(e),
        });
    counted(&core, Event::Merged, r)
}

#[tauri::command]
pub async fn get_commands(core: CoreState<'_>, id: String) -> Res<Vec<Value>> {
    let h = core.agent(&id).map_err(err)?;
    let cached = h.lock().commands.clone();
    if !cached.is_empty() {
        return Ok(cached);
    }
    core.ensure_process(&id).await.map_err(err)?;
    let cmds = h.lock().commands.clone();
    Ok(cmds)
}

#[tauri::command]
pub async fn file_suggestions(core: CoreState<'_>, id: String, query: String) -> Res<Vec<String>> {
    core.file_suggestions(&id, &query).await.map_err(err)
}

#[tauri::command]
pub async fn git_files(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
) -> Res<Vec<FileChange>> {
    core.git_files(&project_id, agent_id).await.map_err(err)
}

#[tauri::command]
pub async fn git_diff(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    paths: Vec<String>,
) -> Res<String> {
    core.git_diff(&project_id, agent_id, paths)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn git_project_diff(core: CoreState<'_>, project_id: String) -> Res<Vec<OwnedDiff>> {
    core.git_project_diff(&project_id).await.map_err(err)
}

#[tauri::command]
pub async fn git_log(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
) -> Res<GitLog> {
    core.git_log(&project_id, agent_id).await.map_err(err)
}

#[tauri::command]
pub async fn git_show(core: CoreState<'_>, project_id: String, hash: String) -> Res<String> {
    core.git_show(&project_id, &hash).await.map_err(err)
}

#[tauri::command]
pub async fn git_fetch(core: CoreState<'_>, project_id: String) -> Res<String> {
    let r = core.git_sync(&project_id, SyncOp::Fetch).await.map_err(err);
    counted(&core, Event::GitSync { op: "fetch" }, r)
}

#[tauri::command]
pub async fn git_pull(core: CoreState<'_>, project_id: String) -> Res<String> {
    let r = core.git_sync(&project_id, SyncOp::Pull).await.map_err(err);
    counted(&core, Event::GitSync { op: "pull" }, r)
}

#[tauri::command]
pub async fn git_push(core: CoreState<'_>, project_id: String) -> Res<String> {
    let r = core.git_sync(&project_id, SyncOp::Push).await.map_err(err);
    counted(&core, Event::GitSync { op: "push" }, r)
}

/// A branch operation's error: the refusals the window turns into a question or a message of its
/// own by their code (`BranchRefusal::wire`: `DIRTY`, `IN_WORKTREE:…`, `AGENT_WORKING:…`,
/// `UNMERGED:…`), any other in words.
fn branch_err(e: anyhow::Error) -> String {
    match e.downcast_ref::<crate::core::BranchRefusal>() {
        Some(refused) => refused.wire(),
        None => err(e),
    }
}

/// The project's branches: local ones (the folder's first), then remote ones.
#[tauri::command]
pub async fn branch_list(
    core: CoreState<'_>,
    project_id: String,
) -> Res<Vec<crate::git::BranchInfo>> {
    core.branches(&project_id).await.map_err(err)
}

/// Checks a name for a new branch: as git does, and not taken already.
#[tauri::command]
pub async fn branch_check(core: CoreState<'_>, project_id: String, name: String) -> Res<()> {
    core.branch_check(&project_id, &name).await.map_err(err)
}

/// Switches the project's folder to a branch; the stash's name when its changes were put aside.
#[tauri::command]
pub async fn branch_switch(
    core: CoreState<'_>,
    project_id: String,
    name: String,
    stash: bool,
) -> Res<Option<String>> {
    core.branch_switch(&project_id, &name, stash)
        .await
        .map_err(branch_err)
}

/// Creates a branch at `start` (HEAD when empty), and switches the folder to it with `switch`.
#[tauri::command]
pub async fn branch_create(
    core: CoreState<'_>,
    project_id: String,
    name: String,
    start: String,
    switch: bool,
    stash: bool,
) -> Res<Option<String>> {
    let r = core
        .branch_create(&project_id, &name, &start, switch, stash)
        .await
        .map_err(branch_err);
    counted(&core, Event::BranchCreated, r)
}

/// Deletes a branch (its remote one too with `remote`); unmerged, only with `force`. Returns what
/// is left to tell the user: why a remote branch was kept.
#[tauri::command]
pub async fn branch_delete(
    core: CoreState<'_>,
    project_id: String,
    name: String,
    remote: bool,
    force: bool,
) -> Res<Option<String>> {
    core.branch_delete(&project_id, &name, remote, force)
        .await
        .map_err(branch_err)
}

/// The local branches « Branches mergées » offers to delete.
#[tauri::command]
pub async fn branches_merged(core: CoreState<'_>, project_id: String) -> Res<Vec<String>> {
    core.branches_merged(&project_id).await.map_err(err)
}

/// The diff from `a` to `b`, two branches or commits of the project's repository.
#[tauri::command]
pub async fn git_diff_refs(
    core: CoreState<'_>,
    project_id: String,
    a: String,
    b: String,
) -> Res<String> {
    core.diff_refs(&project_id, &a, &b).await.map_err(err)
}

/// A new agent in a worktree on an existing branch (a local one, or `origin/feat`). Refused for
/// the folder's branch and for one a worktree has: `IN_WORKTREE:…` when an agent's. With
/// `create`, on a new branch of that name, made from where a new agent starts (ESC-155).
#[tauri::command]
pub async fn create_agent_on_branch(
    core: CoreState<'_>,
    project_id: String,
    branch: String,
    model: Option<String>,
    engine: Option<crate::model::EngineId>,
    create: Option<bool>,
    // With `create`: the branch the new one starts from (ESC-160); None: the project's.
    base: Option<String>,
) -> Res<AgentView> {
    let create = create.unwrap_or(false);
    let agent = core
        .create_agent_with(
            &project_id,
            AgentOptions {
                model,
                engine,
                select: true,
                branch: Some(branch),
                new_branch: create,
                base: base.filter(|b| create && !b.trim().is_empty()),
                ..Default::default()
            },
        )
        .await
        .map_err(branch_err)?;
    core.telemetry.track(Event::AgentCreated {
        worktree: agent.meta.worktree.is_some(),
    });
    Ok(agent)
}

/// « Intégrer <base> » on an agent with a worktree: its base branch goes into its branch.
#[tauri::command]
pub async fn integrate_base(core: CoreState<'_>, id: String) -> Res<crate::core::Integration> {
    core.integrate_base(&id).await.map_err(err)
}

/// « Ouvrir une PR » on an ordinary agent's branch: the title and the description proposed.
#[tauri::command]
pub async fn agent_pr_propose(core: CoreState<'_>, id: String) -> Res<crate::agentprs::PrProposal> {
    core.agent_pr_propose(&id).await.map_err(err)
}

/// Pushes the agent's branch and opens its pull request as the user validated it.
#[tauri::command]
pub async fn agent_pr_open(
    core: CoreState<'_>,
    id: String,
    form: crate::agentprs::PrForm,
) -> Res<crate::forge::PullRequest> {
    core.agent_pr_open(&id, form).await.map_err(err)
}

/// Reads the agent's pull requests again (at most every 20 s unless `now`).
#[tauri::command]
pub async fn agent_prs_refresh(core: CoreState<'_>, id: String, now: bool) -> Res<()> {
    core.agent_prs_refresh(&id, now).await.map_err(err)
}

/// « L'agent corrige » / « L'agent corrige les retours » on pull request `number`.
#[tauri::command]
pub async fn agent_pr_fix(
    core: CoreState<'_>,
    id: String,
    number: u64,
    fix: crate::agentprs::PrFix,
) -> Res<()> {
    core.agent_pr_fix(&id, number, fix).await.map_err(err)
}

/// The statistics of a period, of every account or (`account`) of one.
#[tauri::command(async)]
pub fn stats(
    core: CoreState,
    range: String,
    account: Option<String>,
    engine: Option<String>,
) -> StatsView {
    let some = |v: &Option<String>| v.as_deref().filter(|a| !a.is_empty()).map(str::to_string);
    let (account, engine) = (some(&account), some(&engine));
    core.stats_view_by(&range, account.as_deref(), engine.as_deref())
}

#[tauri::command]
pub async fn refresh_usage(core: CoreState<'_>) -> Res<()> {
    core.inner().refresh_usage().await;
    Ok(())
}

#[tauri::command]
pub async fn git_discard(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<()> {
    core.git_discard(&project_id, agent_id, &path)
        .await
        .map_err(err)
}

/// What a direct commit of the agent's changes (the project's own checkout without one) takes.
#[tauri::command]
pub async fn commit_preview(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
) -> Res<CommitScope> {
    core.commit_preview(&project_id, agent_id)
        .await
        .map_err(err)
}

/// Haiku's message for a direct commit of `paths`: only proposed.
#[tauri::command]
pub async fn commit_propose(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    paths: Vec<String>,
) -> Res<String> {
    core.commit_propose(&project_id, agent_id, paths)
        .await
        .map_err(err)
}

/// Commits `paths` with the user's message; the commit's short hash.
#[tauri::command]
pub async fn commit_direct(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    paths: Vec<String>,
    message: String,
) -> Res<String> {
    let r = core
        .commit_direct(&project_id, agent_id, paths, message)
        .await
        .map_err(err);
    counted(&core, Event::Committed, r)
}

// ---------- embedded editor ----------

#[tauri::command]
pub async fn fs_tree(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
) -> Res<fsedit::Tree> {
    core.fs_tree(&project_id, agent_id).await.map_err(err)
}

#[tauri::command]
pub async fn fs_read(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<fsedit::FileText> {
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    fsedit::read(std::path::Path::new(&root), &path).map_err(|e| {
        match e.downcast_ref::<fsedit::NotFound>() {
            // The editor shows a missing file rather than an error: a code it tells apart.
            Some(missing) => missing.wire(),
            None => err(e),
        }
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn fs_write(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
    text: String,
    eol: String,
    bom: bool,
    expected_hash: Option<String>,
) -> Res<String> {
    let (root, worktrees) = core
        .edit_worktrees(&project_id, agent_id)
        .await
        .map_err(err)?;
    let hash = fsedit::write(
        std::path::Path::new(&root),
        &path,
        &text,
        &eol,
        bom,
        expected_hash.as_deref(),
        &worktrees,
    )
    .map_err(err)?;
    core.git.refresh(&project_id);
    core.telemetry.track(Event::FileSaved);
    Ok(hash)
}

#[tauri::command]
pub async fn fs_create(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<()> {
    let (root, worktrees) = core
        .edit_worktrees(&project_id, agent_id)
        .await
        .map_err(err)?;
    fsedit::create(std::path::Path::new(&root), &path, &worktrees).map_err(err)?;
    core.git.refresh(&project_id);
    Ok(())
}

/// Renames or moves a file or a folder of the source; refused when something else is at `to`, or
/// when only the case of what git tracks changes.
#[tauri::command]
pub async fn fs_rename(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    from: String,
    to: String,
) -> Res<()> {
    let (root, kept) = core.edit_kept(&project_id, agent_id).await.map_err(err)?;
    // An update's restart waits: a folder on a slow disk can take a moment.
    let _working = core.working();
    fsedit::rename_entry(root, from, to, kept)
        .await
        .map_err(err)?;
    core.git.refresh(&project_id);
    Ok(())
}

/// Sends a file or a folder of the source to the system's trash.
#[tauri::command]
pub async fn fs_delete(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<()> {
    let (root, kept) = core.edit_kept(&project_id, agent_id).await.map_err(err)?;
    let _working = core.working();
    // A big folder takes a while to go: off the async workers.
    tokio::task::spawn_blocking(move || {
        fsedit::delete(std::path::Path::new(&root), &path, &kept, fsedit::to_trash)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)?;
    core.git.refresh(&project_id);
    Ok(())
}

/// Shows a file or a folder of the source in the system's file manager, selected; the source's
/// folder opened when no path is given.
#[tauri::command]
pub async fn fs_reveal(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: Option<String>,
) -> Res<()> {
    let what = core
        .reveal_target(&project_id, agent_id, path.as_deref())
        .await
        .map_err(err)?;
    // The file manager can take a moment to answer (WSL's share starts its distribution, D-Bus
    // waits for the file manager): off the async workers.
    tokio::task::spawn_blocking(move || fsedit::reveal(what))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

/// Creates an empty folder, its parents with it; refused when something is already there.
#[tauri::command]
pub async fn fs_mkdir(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<()> {
    let (root, worktrees) = core
        .edit_worktrees(&project_id, agent_id)
        .await
        .map_err(err)?;
    fsedit::mkdir(std::path::Path::new(&root), &path, &worktrees).map_err(err)
}

#[tauri::command]
pub async fn fs_base(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<Option<fsedit::Base>> {
    let (root, base) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    fsedit::base(&root, base.as_deref(), &path)
        .await
        .map_err(err)
}

/// The lines of the source's files matching `query` (the agent's worktree, else the project).
#[tauri::command]
pub async fn code_search(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    query: search::SearchQuery,
) -> Res<search::SearchResult> {
    let (root, worktrees) = core
        .edit_worktrees(&project_id, agent_id)
        .await
        .map_err(err)?;
    let r = search::search(&root, &query, &worktrees).await.map_err(err);
    counted(&core, Event::CodeSearched, r)
}

#[tauri::command(async)]
pub fn cancel_resume(core: CoreState, id: String) -> Res<()> {
    core.cancel_resume(&id).map_err(err)
}

// ---------- terminals ----------

/// An interactive terminal in the project's folder; in the agent's worktree when `agent_id` has
/// one, and in the folder `subdir` of that (a path of the editor's tree) when it is given.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn term_spawn(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    subdir: Option<String>,
    shell: String,
    name: String,
    cols: u16,
    rows: u16,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    let cwd = core
        .term_cwd(&project_id, agent_id, subdir.as_deref())
        .await
        .map_err(err)?;
    let settings = core.settings.read().clone();
    let shells = pty::detect_shells(&settings);
    let sh = shells
        .iter()
        .find(|s| s.id == shell)
        .ok_or_else(|| shell_not_found(i18n::ui(), &shell))?;
    let info = TermInfo {
        id: new_id(),
        project_id,
        name,
        shell: sh.id.clone(),
    };
    let env = if settings.proxy_terminals {
        settings.proxy_env()
    } else {
        Vec::new()
    };
    let hub_core = Arc::downgrade(core.inner());
    let id = info.id.clone();
    core.pty
        .spawn(
            info.clone(),
            sh,
            &settings.wsl_distro,
            &cwd,
            (cols, rows),
            env,
            move |bytes| {
                let _ = output.send(InvokeResponseBody::Raw(bytes));
            },
            move |code| {
                if let Some(c) = hub_core.upgrade() {
                    c.hub.emit(UiEvent::TerminalExit { id, code });
                }
            },
        )
        .map_err(err)?;
    core.telemetry.track(Event::TerminalOpened);
    Ok(info)
}

/// Runs one of the project's launch commands in its own terminal; it ends with the command
/// (TerminalExit carries its exit code). Stopped with term_kill, like a terminal.
/// `cursor_row` is the row of its log where it starts. With `agent_id`, it runs in that agent's
/// worktree (`Core::worktree_run`), named after it and with its ports, and stops with its test
/// launches (archive, deletion, validation, worktree removed).
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn run_start(
    core: CoreState,
    project_id: String,
    command_id: String,
    agent_id: Option<String>,
    cols: u16,
    rows: u16,
    cursor_row: Option<u16>,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    let project = core.project(&project_id).map_err(err)?;
    let run = project
        .run_commands
        .iter()
        .find(|c| c.id == command_id)
        .ok_or_else(|| {
            tr!(
                "commande de lancement introuvable",
                "launch command not found"
            )
        })?;
    let settings = core.settings.read().clone();
    let shells = pty::detect_shells(&settings);
    // A command set up on another system (PowerShell on Windows, zsh on macOS) runs in the
    // default shell of this one.
    let sh = shells
        .iter()
        .find(|s| s.id == run.shell)
        .or_else(|| shells.first())
        .ok_or_else(|| shell_not_found(i18n::ui(), &run.shell))?;
    let spec = match &agent_id {
        Some(a) => core
            .worktree_run(&project_id, a, &command_id)
            .map_err(err)?,
        None => crate::testlaunch::RunSpec {
            name: run.name.clone(),
            command: run.command.clone(),
            cwd: pty::run_cwd(&project.path, &run.cwd).map_err(err)?,
            env: Vec::new(),
        },
    };
    let info = TermInfo {
        id: new_id(),
        project_id,
        name: spec.name,
        shell: sh.id.clone(),
    };
    let launch = Launch {
        info,
        shell: sh,
        cwd: &spec.cwd,
        env: spec.env,
        command: &spec.command,
    };
    let r = spawn_launch(
        core.inner(),
        &settings,
        launch,
        (cols, rows),
        cursor_row,
        output,
    );
    if let (Some(a), Ok(info)) = (&agent_id, &r) {
        core.track_test_run(a, &info.id).map_err(err)?;
    }
    counted(&core, Event::LaunchStarted, r)
}

/// What a launch terminal runs: one of the project's launch commands, or a step of an agent's
/// recipe.
struct Launch<'a> {
    info: TermInfo,
    shell: &'a ShellInfo,
    cwd: &'a str,
    /// Its own variables, after the proxy's when terminals go through it.
    env: Vec<(String, String)>,
    command: &'a str,
}

/// Starts `launch` in a read-only terminal of its own, which ends with its command
/// (TerminalExit carries its exit code). `cursor_row` is the row of its log where it starts.
fn spawn_launch(
    core: &Arc<Core>,
    settings: &Settings,
    launch: Launch,
    size: (u16, u16),
    cursor_row: Option<u16>,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    let mut env = if settings.proxy_terminals {
        settings.proxy_env()
    } else {
        Vec::new()
    };
    env.extend(launch.env);
    let hub_core = Arc::downgrade(core);
    let id = launch.info.id.clone();
    core.pty
        .spawn_command(
            launch.info.clone(),
            launch.shell,
            &settings.wsl_distro,
            launch.cwd,
            size,
            env,
            cursor_row.unwrap_or(1),
            launch.command,
            move |bytes| {
                let _ = output.send(InvokeResponseBody::Raw(bytes));
            },
            move |code| {
                if let Some(c) = hub_core.upgrade() {
                    c.hub.emit(UiEvent::TerminalExit { id, code });
                }
            },
        )
        .map_err(err)?;
    Ok(launch.info)
}

/// Synchronous on purpose: Tauri handles these on the window's main thread, in the order the
/// requests arrive, which keeps the bytes of a terminal in the order they were typed. It costs
/// nothing there: `write` only queues the bytes for the terminal's input thread. Returns the
/// write's number in the terminal's log of writes, which the window sends back with the report of
/// a slow echo (`term_report`).
#[tauri::command]
pub fn term_write(core: CoreState, id: String, data: String) -> Res<u64> {
    core.pty
        .write_from_window(&id, data.as_bytes())
        .map_err(err)
}

/// Synchronous, so that resizes keep their order among themselves (the last one wins); it waits
/// on nothing heavy. It is not in order with the writes queued before it: the terminal's input
/// thread writes those whenever the program reads.
#[tauri::command]
pub fn term_resize(core: CoreState, id: String, cols: u16, rows: u16) -> Res<()> {
    core.pty.resize(&id, cols, rows).map_err(err)
}

/// Off the main thread, and off the async runtime's workers: ending a process tree (listing the
/// machine's processes, its windows…) and closing the pseudo-console take their time.
#[tauri::command]
pub async fn term_kill(core: CoreState<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.kill_terminal(&id))
        .await
        .map_err(|e| err(e.into()))
}

/// What the window measured of a terminal (the renderer it draws with, an echo that was slow to
/// show), for the log.
#[tauri::command(async)]
pub fn term_report(core: CoreState, id: String, report: TermReport) {
    core.pty.report(&id, report);
}

#[tauri::command]
pub fn play_chime() {
    crate::notify::play_chime();
}

/// « Quitter »: with an update downloaded, it installs silently and the app stays closed.
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    crate::updates::quit(&app);
}

/// How many files the editor holds unsaved: "Quitter" asks the window first when there are some.
#[tauri::command]
pub fn set_unsaved(core: CoreState, count: usize) {
    core.unsaved
        .store(count, std::sync::atomic::Ordering::Release);
}

/// Turns Remote Control on or off for the agent. True when, in the Claude Code view, the change
/// waits for the interface's next start: it runs, and was started with the other state.
#[tauri::command]
pub async fn set_remote_control(core: CoreState<'_>, id: String, enabled: bool) -> Res<bool> {
    let at_next_start = core.set_remote_control(&id, enabled).await.map_err(err)?;
    if enabled {
        core.telemetry
            .track(Event::RemoteControlEnabled { from: "agent" });
    }
    Ok(at_next_start)
}

// ---------- board ----------

#[tauri::command]
pub async fn ticket_create(
    core: CoreState<'_>,
    project_id: String,
    draft: TicketDraft,
) -> Res<Ticket> {
    let r = core.ticket_create(&project_id, draft).await.map_err(err);
    counted(&core, Event::TicketCreated, r)
}

/// Its pasted images are written: off the async workers.
#[tauri::command]
pub async fn ticket_update(core: CoreState<'_>, id: String, draft: TicketDraft) -> Res<Ticket> {
    let core = core.inner().clone();
    tokio::task::spawn_blocking(move || core.ticket_update(&id, draft))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

/// The images of a ticket, with their content, for its form to show: read off the async workers.
#[tauri::command]
pub async fn ticket_images(
    core: CoreState<'_>,
    id: String,
) -> Res<Vec<crate::ticketimages::ImageData>> {
    let core = core.inner().clone();
    tokio::task::spawn_blocking(move || core.ticket_images(&id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[tauri::command]
pub async fn ticket_delete(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_delete(&id).await.map_err(err)
}

/// « Nettoyer » in the project's settings: the worktrees its archived agents left (ESC-143).
#[tauri::command]
pub async fn clean_worktrees(
    core: CoreState<'_>,
    project_id: String,
) -> Res<crate::leftovers::LeftoverReport> {
    core.clean_leftover_worktrees(&project_id)
        .await
        .map_err(err)
}

#[tauri::command(async)]
pub fn ticket_prioritize(core: CoreState, id: String) -> Res<()> {
    core.ticket_prioritize(&id).map_err(err)
}

#[tauri::command(async)]
pub fn ticket_start(core: CoreState, id: String) -> Res<()> {
    core.ticket_start(&id).map_err(err)
}

#[tauri::command]
pub async fn ticket_resume(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_resume(&id).await.map_err(err)
}

#[tauri::command]
pub async fn ticket_approve(core: CoreState<'_>, id: String) -> Res<()> {
    let r = core.ticket_approve(&id).await.map_err(err);
    counted(&core, Event::TicketApproved, r)
}

#[tauri::command]
pub async fn ticket_reject(core: CoreState<'_>, id: String, comment: String) -> Res<()> {
    core.ticket_reject(&id, &comment).await.map_err(err)
}

#[tauri::command]
pub async fn ticket_resolve_conflict(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_resolve_conflict(&id).await.map_err(err)
}

#[tauri::command(async)]
pub fn ticket_dismiss(core: CoreState, id: String) -> Res<()> {
    core.ticket_dismiss(&id).map_err(err)
}

#[tauri::command(async)]
pub fn board_set(core: CoreState, project_id: String, settings: BoardSettings) -> Res<Project> {
    core.board_set(&project_id, settings).map_err(err)
}

/// « Reprendre maintenant » in the board's header: the autopilot's pause is lifted.
#[tauri::command(async)]
pub fn autopilot_resume(core: CoreState) {
    core.autopilot_resume();
}

#[tauri::command]
pub async fn git_branches(core: CoreState<'_>, project_id: String) -> Res<Vec<String>> {
    core.git_branches(&project_id).await.map_err(err)
}

#[tauri::command]
pub async fn agent_prepare_launch(core: CoreState<'_>, id: String) -> Res<()> {
    core.agent_prepare_launch(&id).await.map_err(err)
}

/// « Lancer » in the test modal: the user read `recipe`, the agent's recipe as the modal showed it,
/// and lets it run. Refused when the agent's recipe is no longer that one.
#[tauri::command(async)]
pub fn test_recipe_approve(core: CoreState, agent_id: String, recipe: TestRecipe) -> Res<()> {
    core.approve_recipe(&agent_id, recipe).map_err(err)
}

/// Runs a step of an agent's recipe ("prep" or "run", its index) in its own read-only terminal,
/// like a launch command, with the system's default shell and the reserved ports exported. One
/// whose agent was archived, deleted or put to validation meanwhile is stopped at once, and the
/// error tells why. A recipe the user did not approve (`test_recipe_approve`) runs nothing.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn test_run_start(
    core: CoreState,
    agent_id: String,
    kind: String,
    index: usize,
    cols: u16,
    rows: u16,
    cursor_row: Option<u16>,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    let spec = core.test_run_spec(&agent_id, &kind, index).map_err(err)?;
    let project_id = core
        .agent(&agent_id)
        .map_err(err)?
        .lock()
        .meta
        .project_id
        .clone();
    let settings = core.settings.read().clone();
    let shells = pty::detect_shells(&settings);
    let sh = shells.first().ok_or_else(no_shell)?;
    let info = TermInfo {
        id: new_id(),
        project_id,
        name: spec.name,
        shell: sh.id.clone(),
    };
    let launch = Launch {
        info,
        shell: sh,
        cwd: &spec.cwd,
        env: spec.env,
        command: &spec.command,
    };
    let info = spawn_launch(
        core.inner(),
        &settings,
        launch,
        (cols, rows),
        cursor_row,
        output,
    )?;
    core.track_test_run(&agent_id, &info.id).map_err(err)?;
    core.telemetry.track(Event::TestRunStarted);
    Ok(info)
}

/// The `.isola.toml` of the agent's worktree, whole, as `isola up` will read it: what the test modal
/// shows before the services start.
#[tauri::command(async)]
pub fn isola_config(core: CoreState, agent_id: String) -> Res<String> {
    core.isola_config(&agent_id).map_err(err)
}

/// « Lancer » in the test modal of an agent whose services isola runs: the user read `config` and the
/// address to open `open`. Refused when the file or the address is no longer what the modal showed.
#[tauri::command(async)]
pub fn isola_approve(core: CoreState, agent_id: String, config: String, open: String) -> Res<()> {
    core.approve_isola(&agent_id, config, open).map_err(err)
}

/// The services isola runs for the agent's worktree, with their addresses.
#[tauri::command]
pub async fn isola_services(
    core: CoreState<'_>,
    agent_id: String,
) -> Res<Vec<crate::isola::Service>> {
    core.isola_services(&agent_id).await.map_err(err)
}

/// Stops the services isola runs for the agent's worktree.
#[tauri::command]
pub async fn isola_down(core: CoreState<'_>, agent_id: String) -> Res<()> {
    core.isola_down(&agent_id).await.map_err(err)
}

/// The setup and teardown Claude suggests for the project's worktrees, from what it reads of it.
#[tauri::command]
pub async fn suggest_worktree_steps(
    core: CoreState<'_>,
    project_id: String,
) -> Res<crate::worktrees::WorktreeSuggestion> {
    core.suggest_worktree_steps(&project_id).await.map_err(err)
}

/// The launch commands Claude suggests for the project, from what it reads of it (nothing is saved), with how many it gave that were refused.
#[tauri::command]
pub async fn suggest_run_commands(
    core: CoreState<'_>,
    project_id: String,
) -> Res<crate::worktrees::RunSuggestion> {
    core.suggest_run_commands(&project_id).await.map_err(err)
}

/// The same, for the folder of a project not made yet (the new project's assistant).
#[tauri::command]
pub async fn suggest_worktree_steps_at(
    core: CoreState<'_>,
    path: String,
) -> Res<crate::worktrees::WorktreeSuggestion> {
    core.suggest_worktree_steps_at(&path).await.map_err(err)
}

/// The same, for the folder of a project not made yet (the new project's assistant).
#[tauri::command]
pub async fn suggest_run_commands_at(
    core: CoreState<'_>,
    path: String,
) -> Res<crate::worktrees::RunSuggestion> {
    core.suggest_run_commands_at(&path).await.map_err(err)
}

/// Looks for a newer release, through the network settings as they are now. `manual`: the user
/// asked, a failure is shown whatever the checks before it did; else it is kept quiet until it goes
/// on. A failure is an answer (`failed`, with its cause), not an error of the command.
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    core: CoreState<'_>,
    updates: State<'_, crate::updates::Updates>,
    manual: bool,
) -> Res<crate::updates::Checked> {
    let settings = core.settings.read().clone();
    Ok(crate::updates::check(&app, &settings, &updates, manual).await)
}

/// Downloads the update found as `id`, its signature checked, and keeps it until it installs. A
/// download that fails reads latest.json again and is tried once more before it is told (`failed`,
/// with its cause and the release's page, for the browser), under the rule of `update_check`
/// for `manual`.
#[tauri::command]
pub async fn update_download(
    updates: State<'_, crate::updates::Updates>,
    id: u32,
    manual: bool,
) -> Res<crate::updates::Downloaded> {
    Ok(updates.downloaded(id, manual).await)
}

/// « Redémarrer maintenant »: the update downloaded installs, the app stopped cleanly first, and
/// the app starts again on it.
#[tauri::command]
pub async fn update_restart(app: tauri::AppHandle) -> Res<()> {
    crate::updates::restart_now(app).await.map_err(err)
}

/// « Plus tard »: the automatic restart planned is called off.
#[tauri::command]
pub fn update_postpone(app: tauri::AppHandle) {
    crate::updates::postpone(&app);
}

/// What the window says of itself for an automatic restart: a modal open, a test launch being
/// prepared, its last key or mouse input.
#[tauri::command]
pub fn update_presence(
    updates: State<crate::updates::Updates>,
    presence: crate::updates::Presence,
) {
    updates.set_presence(presence);
}

/// The update found as `id` is no longer offered.
#[tauri::command]
pub fn update_close(updates: State<crate::updates::Updates>, id: u32) {
    updates.free(id);
}

/// One HTTP request to `url` (http or https only) without proxy, 2 s at most: true for any
/// answer.
#[tauri::command]
pub async fn http_ready(url: String) -> bool {
    crate::testlaunch::http_ready(&url).await
}

// ---------- code hosts ----------

#[tauri::command]
pub async fn forges(core: CoreState<'_>) -> Res<crate::forge::connect::ForgesView> {
    Ok(core.forges_view().await)
}

#[tauri::command]
pub async fn forge_check(core: CoreState<'_>) -> Res<crate::forge::connect::ForgesView> {
    Ok(core.forge_check().await)
}

#[tauri::command]
pub async fn forge_connect(
    core: CoreState<'_>,
    form: crate::forge::connect::ForgeForm,
) -> Res<crate::forge::connect::ForgesView> {
    core.forge_connect(form).await.map_err(err)
}

#[tauri::command]
pub async fn forge_disconnect(
    core: CoreState<'_>,
    host: String,
) -> Res<crate::forge::connect::ForgesView> {
    core.forge_disconnect(&host).await.map_err(err)
}

#[tauri::command]
pub async fn forge_guess(core: CoreState<'_>, url: String) -> Res<Option<String>> {
    core.forge_guess(&url).await.map_err(err)
}

#[tauri::command]
pub fn forge_token_page(core: CoreState, kind: String, url: String) -> Res<String> {
    core.forge_token_page(&kind, &url).map_err(err)
}

// ---------- external ticket systems ----------

#[tauri::command]
pub async fn integration_connect(
    core: CoreState<'_>,
    service: Service,
    account: Account,
) -> Res<AccountView> {
    let r = core
        .integration_connect(service, account)
        .await
        .map_err(err);
    let service = telemetry::service(service);
    counted(&core, Event::IntegrationConnected { service }, r)
}

#[tauri::command]
pub fn integration_disconnect(core: CoreState, service: Service) -> Res<Vec<AccountView>> {
    core.integration_disconnect(service).map_err(err)
}

#[tauri::command]
pub async fn integration_containers(
    core: CoreState<'_>,
    service: Service,
    project_id: Option<String>,
) -> Res<Vec<Container>> {
    core.integration_containers(service, project_id.as_deref())
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn integration_states(
    core: CoreState<'_>,
    service: Service,
    container: String,
) -> Res<StatesView> {
    core.integration_states(service, &container)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn integration_issues(
    core: CoreState<'_>,
    project_id: String,
    service: Service,
    text: String,
    filters: Vec<String>,
    page: Option<String>,
) -> Res<IssuePage> {
    let q = Query {
        text,
        filters,
        label: None,
        page,
    };
    core.integration_issues(&project_id, service, q)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn integration_import(
    core: CoreState<'_>,
    project_id: String,
    issues: Vec<ExternalIssue>,
    max_loops: u32,
) -> Res<Vec<Ticket>> {
    let service = issues.first().map(|i| telemetry::service(i.service));
    let r = core
        .integration_import(&project_id, issues, max_loops)
        .await
        .map_err(err);
    match service {
        Some(service) => counted(&core, Event::TicketsImported { service }, r),
        None => r,
    }
}

/// « Resynchroniser » (the ⚠ of an imported ticket's card): its failed syncs go again at once.
#[tauri::command]
pub async fn integration_resync(core: CoreState<'_>, ticket_id: String) -> Res<()> {
    core.integration_resync(&ticket_id).await.map_err(err)
}

/// The MCP server (« Claude et les agents peuvent piloter Escouade »): running or not, its port, why it failed.
// Off the main thread: a start waiting for its port is no reason for the window to wait.
#[tauri::command(async)]
pub fn mcp_status(core: CoreState) -> crate::mcp::McpStatus {
    core.mcp.status()
}

/// The MCP server's activity log, oldest first (the last 200 entries).
#[tauri::command]
pub fn mcp_activity(core: CoreState) -> Vec<crate::mcp::activity::ActivityEntry> {
    core.mcp.activity.entries()
}

/// « Effacer » of the MCP activity log.
#[tauri::command]
pub fn mcp_clear_activity(core: CoreState) {
    core.mcp.activity.clear();
}

/// « Les agents peuvent piloter Escouade » turned on or off: saved at once, not through the
/// settings modal's draft (it writes into the config of each account of every harness installed,
/// which « Annuler » could not take back). The declaration follows by itself (`mcp_declare_status`, `mcpDeclared`).
#[tauri::command(async)]
pub fn mcp_set_enabled(core: CoreState, enabled: bool) -> Res<()> {
    core.set_mcp_enabled(enabled).map_err(err)?;
    if enabled {
        core.telemetry.track(Event::McpEnabled);
    }
    Ok(())
}

/// The answer to « Statistiques d’usage », at the first start.
#[tauri::command(async)]
pub fn telemetry_set(core: CoreState, consent: bool) -> Res<()> {
    core.set_telemetry(consent).map_err(err)
}

/// Where the server stands in each active account of each harness installed: declared, or why not
/// and the command to run (or the entry to paste) by hand, its token hidden. Asked when the window
/// shows it: a harness installed since, or a file changed by hand, is seen to by a new run, which
/// the window hears of (`mcpDeclared`).
#[tauri::command]
pub fn mcp_declare_status(core: CoreState) -> Vec<crate::mcp::install::Declaration> {
    core.declare_in_harnesses();
    core.mcp.declared()
}

/// The command that declares the server by hand in the account, with the real token: asked for
/// when the user copies it, never sent with the state.
#[tauri::command(async)]
pub fn mcp_manual_command(core: CoreState, account: String) -> Res<String> {
    core.mcp_manual_command(&account).map_err(err)
}

// ---------------------------------------------------------------------------------------------
// Dictation (`voice`).

type VoiceState<'a> = State<'a, Arc<crate::voice::Voice>>;

/// What Réglages › Voix lists: the microphones and the models (which are downloaded).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInfo {
    devices: Vec<crate::voice::source::Device>,
    models: Vec<crate::voice::models::ModelView>,
}

#[tauri::command(async)]
pub fn voice_info(voice: VoiceState) -> VoiceInfo {
    VoiceInfo {
        devices: voice.devices(),
        models: voice.models(),
    }
}

/// The settings' choice for a dictation: microphone, model, and the language Whisper is told;
/// on line, the service and its key, from the keychain, and the client of the proxy of the settings.
fn voice_choice(
    settings: &Settings,
    secrets: &dyn crate::integrations::secrets::SecretStore,
    catalog: &crate::voice::online_catalog::CatalogStore,
) -> Result<crate::voice::Choice, crate::voice::VoiceError> {
    let online = crate::voice::online::spec_of(settings, secrets, catalog)?;
    Ok(crate::voice::Choice {
        device: settings.voice.device.clone(),
        model: settings.voice.model.clone(),
        language: crate::voice::whisper_language(&settings.voice.language, i18n::ui()),
        online,
    })
}

fn voice_err(e: crate::voice::VoiceError) -> String {
    e.message(i18n::ui(), std::env::consts::OS)
}

/// Starts listening, on the microphone and with the model of the settings; or says the model is
/// to be downloaded first. Off the main thread: macOS asks the user for the microphone the first
/// time, and the answer waits for them.
#[tauri::command(async)]
pub fn voice_start(core: CoreState, voice: VoiceState) -> Res<crate::voice::Started> {
    let choice =
        voice_choice(&core.settings.read(), &*core.secrets, &voice.catalog).map_err(voice_err)?;
    voice.start(&choice).map_err(voice_err)
}

/// Stops listening and gives the text heard (empty when nothing was said), transcribed on a
/// thread of its own. `target`, where it goes ("composer", "terminal", "ticket"), only for the
/// usage statistics, which never hold the text.
#[tauri::command]
pub async fn voice_stop(
    core: CoreState<'_>,
    voice: VoiceState<'_>,
    session: u64,
    target: String,
) -> Res<String> {
    let v = voice.inner().clone();
    let text = tauri::async_runtime::spawn_blocking(move || v.stop(session))
        .await
        .map_err(|e| e.to_string())?
        .map_err(voice_err)?;
    if !text.is_empty() {
        let target = match target.as_str() {
            "terminal" => "terminal",
            "ticket" => "ticket",
            _ => "composer",
        };
        let settings = core.settings.read().voice.clone();
        // On line, the service stands for the model: never the key nor the text.
        let (model, online_model) = match crate::voice::online_provider(&settings) {
            Some(p) => {
                let chosen = settings
                    .online_models
                    .get(p.id())
                    .map(String::as_str)
                    .unwrap_or("");
                let (m, _) = voice.catalog.checked_model(
                    p,
                    crate::voice::online_catalog::Purpose::Transcribe,
                    chosen,
                );
                (p.id(), Some(m))
            }
            None => (
                crate::voice::models::find(&settings.model)
                    .map(|m| m.id)
                    .unwrap_or(crate::voice::models::DEFAULT_MODEL),
                None,
            ),
        };
        core.telemetry.track(Event::DictationUsed {
            target,
            model,
            online_model,
        });
    }
    Ok(text)
}

/// Réglages › Voix: the services of « En ligne » whose key is in the keychain (never the keys).
#[tauri::command(async)]
pub fn voice_online_keys(core: CoreState) -> Vec<String> {
    crate::voice::online::keys(&*core.secrets)
}

/// Keeps the key of `service` in the system's keychain, read back to check; never in a file: a
/// keychain that refuses it (or none, on Linux) leaves it unsaved, and says so.
#[tauri::command(async)]
pub fn voice_online_key_set(core: CoreState, service: String, key: String) -> Res<()> {
    crate::voice::online::save_key(&*core.secrets, &service, &key)
}

/// Forgets the key of `service`.
#[tauri::command(async)]
pub fn voice_online_key_delete(core: CoreState, service: String) -> Res<()> {
    crate::voice::online::delete_key(&*core.secrets, &service)
}

/// Réglages › Voix: the models and the voices of `service` (ESC-132). What was listed is kept; `refresh`, or nothing
/// listed yet with a key kept, reads its API again with the key (only the key goes).
#[tauri::command]
pub async fn voice_online_catalog(
    core: CoreState<'_>,
    voice: VoiceState<'_>,
    service: String,
    refresh: bool,
) -> Res<crate::voice::online_catalog::ServiceCatalog> {
    use crate::voice::online_catalog::{catalog, fetch};
    let p = crate::voice::online::Provider::find(&service)
        .ok_or_else(|| format!("unknown service {service}"))?;
    let kept = voice.catalog.get(p);
    if !refresh && kept.is_some() {
        return Ok(catalog(p, kept.as_ref(), None));
    }
    let settings = core.settings.read().clone();
    let spec = match crate::voice::online::keyed(p, &settings, &*core.secrets) {
        Ok(s) => s,
        // No key: the documentation's, said only when asked.
        Err(e) => {
            let error = refresh.then(|| voice_err(e));
            return Ok(catalog(p, kept.as_ref(), error));
        }
    };
    match fetch(&spec).await {
        Ok(listed) => {
            if let Err(e) = voice.catalog.put(p, &listed) {
                log::warn!("voice: the catalog of {} not kept: {e:#}", p.label());
            }
            Ok(catalog(p, Some(&listed), None))
        }
        Err(e) => Ok(catalog(p, kept.as_ref(), Some(e.message(i18n::ui())))),
    }
}

/// Escape, or the field dictated into gone: the listening and the transcription stop, nothing
/// given.
#[tauri::command]
pub fn voice_cancel(voice: VoiceState) {
    voice.cancel();
}

/// Downloads a model (its progress in `UiEvent::Voice`), through the proxy of the settings.
#[tauri::command]
pub async fn voice_download(core: CoreState<'_>, voice: VoiceState<'_>, model: String) -> Res<()> {
    let client = crate::voice::models::client(&core.settings.read()).map_err(err)?;
    voice.download(&client, &model).await.map_err(err)
}

#[tauri::command]
pub fn voice_download_cancel(voice: VoiceState, model: String) {
    voice.cancel_download(&model);
}

/// « Effacer le modèle »: its file goes (it is downloaded again at the next dictation).
#[tauri::command(async)]
pub fn voice_delete_model(voice: VoiceState, model: String) -> Res<()> {
    voice.delete_model(&model).map_err(err)
}

// ---------------------------------------------------------------------------------------------
// The voice conversation (`voice::conversation`, `voice::speech`, ESC-97).

/// The language a conversation is held in: Whisper's, when it is one the voices speak, else the interface's.
fn speech_lang(settings: &Settings) -> &'static str {
    match crate::voice::whisper_language(&settings.voice.language, i18n::ui()).as_str() {
        "fr" => "fr",
        "en" => "en",
        _ => match i18n::ui() {
            i18n::Lang::Fr => "fr",
            i18n::Lang::En => "en",
        },
    }
}

fn speak_options(settings: &Settings, voice: &str) -> crate::voice::speech::SpeakOptions {
    let s = &settings.voice.speech;
    crate::voice::speech::SpeakOptions {
        speaker: s.speakers.get(voice).copied().unwrap_or(0),
        rate: if s.rate.is_finite() {
            s.rate.clamp(0.5, 2.0)
        } else {
            1.0
        },
    }
}

/// What a conversation is held with, from `settings`, and its language.
fn conv_choice(
    settings: &Settings,
    secrets: &dyn crate::integrations::secrets::SecretStore,
    catalog: &crate::voice::online_catalog::CatalogStore,
) -> (crate::voice::conversation::ConvChoice, &'static str) {
    let lang = speech_lang(settings);
    let id = settings.voice.speech.voice(lang);
    // « Voix générée par » a service on line (ESC-132), with the key of its dictation.
    let online = crate::voice::online_speech::resolve(settings, secrets, catalog, lang);
    // « Transcription : En ligne » (ESC-134): each sentence goes to the service of the dictation, told its language.
    let (transcription, transcription_error) =
        match crate::voice::online::spec_of(settings, secrets, catalog) {
            Ok(spec) => (spec, None),
            Err(e) => (None, Some(e)),
        };
    let language = if transcription.is_some() || transcription_error.is_some() {
        crate::voice::whisper_language(&settings.voice.language, i18n::ui())
    } else {
        lang.to_string()
    };
    let choice = crate::voice::conversation::ConvChoice {
        device: settings.voice.device.clone(),
        model: settings.voice.model.clone(),
        language,
        transcription,
        transcription_error,
        speak: speak_options(settings, &id),
        volume: settings.voice.speech.volume.clamp(0.0, 1.0),
        silence_ms: settings.voice.speech.silence_ms,
        voice: id,
        online: online.online,
        online_error: online.error,
        online_notice: online.notice,
    };
    (choice, lang)
}

/// Réglages › Voix: the conversation's voices (which are downloaded), and whether the app can speak here.
#[tauri::command(async)]
pub fn speech_info(voice: VoiceState) -> crate::voice::SpeechInfo {
    voice.speech_info()
}

/// Starts a conversation with the settings' microphone, model and voice; or says what is to be downloaded first.
#[tauri::command(async)]
pub fn conversation_start(
    core: CoreState,
    voice: VoiceState,
) -> Res<crate::voice::conversation::ConvStarted> {
    let (choice, lang) = conv_choice(&core.settings.read(), &*core.secrets, &voice.catalog);
    // The engine and the model that read, never the text: Piper and its voice, or the service and its model.
    let (engine, model, id) = match &choice.online {
        Some(o) => (
            o.spec.provider.id(),
            o.choice.model.clone(),
            o.choice.voice.clone(),
        ),
        None => ("piper", "piper".to_string(), choice.voice.clone()),
    };
    let started = voice.conversation_start(choice).map_err(voice_err)?;
    if matches!(
        started,
        crate::voice::conversation::ConvStarted::Listening { .. }
    ) {
        core.telemetry.track(Event::VoiceConversation {
            lang,
            voice: id,
            engine,
            model,
        });
    }
    Ok(started)
}

#[tauri::command]
pub fn conversation_listen(voice: VoiceState, session: u64) {
    voice.conversation_listen(session);
}

#[tauri::command]
pub fn conversation_hold(voice: VoiceState, session: u64) {
    voice.conversation_hold(session);
}

/// Reads `text` aloud; the number of the reading, which its end and its barge-in carry.
#[tauri::command]
pub fn conversation_say(voice: VoiceState, session: u64, text: String) -> Res<u64> {
    voice.conversation_say(session, &text).map_err(voice_err)
}

#[tauri::command]
pub fn conversation_stop(voice: VoiceState) {
    voice.conversation_stop();
}

/// A sentence said in the conversation, sent to the agent with the instruction to end with a passage to be read.
#[tauri::command]
pub async fn conversation_send(core: CoreState<'_>, id: String, text: String) -> Res<()> {
    let r = core.send_spoken(&id, text).await.map_err(err);
    counted(&core, Event::MessageSent, r)
}

/// Downloads a voice (its progress in `UiEvent::Voice`), through the proxy of the settings.
#[tauri::command]
pub async fn speech_download(core: CoreState<'_>, voice: VoiceState<'_>, id: String) -> Res<()> {
    let client = crate::voice::models::client(&core.settings.read()).map_err(err)?;
    voice.download_voice(&client, &id).await.map_err(err)
}

#[tauri::command]
pub fn speech_download_cancel(voice: VoiceState, id: String) {
    voice.cancel_voice_download(&id);
}

#[tauri::command(async)]
pub fn speech_delete(voice: VoiceState, id: String) -> Res<()> {
    voice.delete_voice(&id).map_err(err)
}

/// « Écouter »: a sentence read in the voice `id`, with the settings' pace and volume (and `speaker`, for a voice that
/// has several). Returns once it is heard.
#[tauri::command(async)]
pub fn speech_preview(
    core: CoreState,
    voice: VoiceState,
    id: String,
    speaker: Option<u32>,
) -> Res<()> {
    let settings = core.settings.read().clone();
    let mut opts = speak_options(&settings, &id);
    if let Some(s) = speaker {
        opts.speaker = s;
    }
    let lang = crate::voice::speech::catalog::find(&id)
        .map(|v| v.lang.as_str())
        .unwrap_or("fr");
    voice
        .preview(
            &id,
            preview_text(lang),
            &opts,
            settings.voice.speech.volume.clamp(0.0, 1.0),
        )
        .map_err(err)
}

/// « ▶ Écouter » on line (ESC-132): a sentence read by `service` with `model` and `voice_id` in `lang`, at the
/// settings' pace and volume. Returns once it is heard; a voice the service refuses is said at once, with its answer.
#[tauri::command(async)]
pub fn speech_preview_online(
    core: CoreState,
    voice: VoiceState,
    service: String,
    model: String,
    voice_id: String,
    lang: String,
) -> Res<()> {
    let settings = core.settings.read().clone();
    let p = crate::voice::online::Provider::find(&service)
        .ok_or_else(|| format!("unknown service {service}"))?;
    let spec = crate::voice::online::keyed(p, &settings, &*core.secrets)
        .map_err(voice_err)?
        .with_model(&model);
    let lang = if lang == "en" { "en" } else { "fr" };
    let choice = crate::voice::online_speech::SpeakChoice {
        model: spec.model.clone(),
        voice: voice_id.trim().to_string(),
        lang: lang.into(),
        rate: if settings.voice.speech.rate.is_finite() {
            settings.voice.speech.rate.clamp(0.5, 2.0)
        } else {
            1.0
        },
    };
    voice
        .preview_online(
            &spec,
            &choice,
            preview_text(lang),
            settings.voice.speech.volume.clamp(0.0, 1.0),
        )
        .map_err(|e| e.message(i18n::ui()))
}

fn preview_text(lang: &str) -> &'static str {
    if lang == "fr" {
        "Bonjour ! C’est la voix qui te lira les réponses de tes agents dans Escouade."
    } else {
        "Hello! This is the voice that will read your agents’ answers to you in Escouade."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusals_of_a_terminal_read_in_english() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(shell_not_found(En, "zsh"), "shell “zsh” not found");
        assert_eq!(shell_not_found(Fr, "zsh"), "shell « zsh » introuvable");
    }

    #[test]
    fn the_commands_of_the_background_tasks_are_in_the_handler_the_window_calls() {
        // A command that is written and not listed there is never reachable from the window.
        let lib = include_str!("lib.rs");
        let handler = lib
            .split_once("generate_handler![")
            .and_then(|(_, rest)| rest.split_once("])"))
            .map(|(list, _)| list)
            .expect("the handler list");
        for name in ["stop_bg_task", "task_output"] {
            assert_eq!(
                handler.matches(&format!("commands::{name},")).count(),
                1,
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn adding_a_folder_is_counted_as_a_project_created_unless_a_history_came_back() {
        let h = crate::core_tests::harness("commands-project-counted");
        h.core.set_telemetry(true).unwrap();
        let created = || {
            h.core
                .telemetry
                .pending()
                .iter()
                .filter(|e| e["eventName"] == "project_created")
                .count()
        };
        let (p, r) = h.project(false).await;
        assert_eq!(created(), 0, "the core itself counts nothing");

        // A new project is counted; so is the next one.
        assert!(project_added(&h.core, Ok((p.clone(), false))).is_ok());
        assert_eq!(created(), 1);
        assert!(project_added(&h.core, Ok((p.clone(), false))).is_ok());
        assert_eq!(created(), 2);
        // One that was refused is not, and neither is a history that came back.
        assert!(project_added(&h.core, Err("non".into())).is_err());
        assert!(project_added(&h.core, Ok((p.clone(), true))).is_ok());
        assert_eq!(created(), 2);

        // What the core says of a real adding: a blank project, then the same folder closed and
        // added again.
        let (_, back) = h
            .core
            .add_project(
                &r.to_string_lossy(),
                "",
                "oklch(0.7 0.1 20)",
                false,
                None,
                false,
                None,
            )
            .await
            .unwrap();
        assert!(!back);
        h.core.remove_project(&p.id, false).await.unwrap();
        let (again, back) = h
            .core
            .add_project(
                &r.to_string_lossy(),
                "",
                "oklch(0.7 0.1 20)",
                false,
                None,
                true,
                None,
            )
            .await
            .unwrap();
        assert!(back && again.id == p.id);
    }
}
