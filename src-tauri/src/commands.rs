//! Tauri commands invoked by the frontend.

use crate::core::{Attachment, Core, NotOnBase, SyncOp};
use crate::fsedit;
use crate::integrations::{
    Account, AccountView, Container, ExternalIssue, IssuePage, Query, StatesView,
};
use crate::model::*;
use crate::pty::{self, ShellInfo, TermInfo};
use crate::stats::StatsView;
use crate::tickets::TicketDraft;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;

type Res<T> = Result<T, String>;
type CoreState<'a> = State<'a, Arc<Core>>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
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
    claude_found: bool,
    version: String,
    models: Vec<ModelInfo>,
    /// Why no ticket of a project's board starts, by project.
    board_issues: HashMap<String, String>,
    /// Why no ticket of any board starts for now (a quota, a usage limit), if none does.
    autopilot_pause: Option<AutopilotPause>,
    /// The external ticket systems' accounts, as the window knows them.
    accounts: Vec<AccountView>,
    /// The update installed since the app's last start, told once (« Voir les nouveautés »).
    installed: Option<crate::updates::Installed>,
    /// The automatic restart planned for an update: when it comes.
    restart_at: Option<i64>,
    /// A version that did not install at the last try (the window offers to try again).
    failed_update: Option<String>,
}

#[tauri::command]
pub fn subscribe(
    core: CoreState,
    updates: State<crate::updates::Updates>,
    channel: Channel<UiEvent>,
) -> InitialState {
    core.hub.set_channel(channel);
    core.reset_unsaved();
    // One lock at a time, each guard ending with its statement: in a struct literal the guards
    // would live together (the lock order is projects before tickets, see `Core`).
    let settings = core.settings.read().clone();
    let projects = core.projects.read().clone();
    let tickets = core.tickets.read().clone();
    let agents = core.agent_views();
    let ui = core.ui.read().clone();
    let usage = core.usage.lock().clone();
    let git = core.git_cache.read().clone();
    let terminals = core.pty.list();
    let models = core.models.read().clone();
    let board_issues = core.board_issues.lock().clone();
    // As it is now: the timer tells this window when it ends (and the board goes on then).
    let autopilot_pause = core.autopilot_pause();
    let accounts = core.integration_accounts();
    let version = core.app.package_info().version.to_string();
    InitialState {
        projects,
        tickets,
        agents,
        ui,
        shells: pty::detect_shells(&settings),
        claude_found: crate::claude::resolve_binary(&settings.claude_path).is_some(),
        settings,
        usage,
        git,
        terminals,
        installed: updates.take_installed(),
        restart_at: updates.restart_at(),
        failed_update: updates.failed(),
        version,
        models,
        board_issues,
        autopilot_pause,
        accounts,
    }
}

#[tauri::command]
pub fn set_ui(core: CoreState, ui: UiState) {
    *core.ui.write() = ui;
    core.request_save();
}

#[tauri::command(async)]
pub fn save_settings(core: CoreState, settings: Settings) -> Res<Vec<ShellInfo>> {
    core.save_settings(settings.clone()).map_err(err)?;
    Ok(pty::detect_shells(&settings))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInfo {
    exists: bool,
    is_repo: bool,
    branch: String,
    dirty: u32,
    name: String,
}

#[tauri::command]
pub async fn inspect_folder(path: String) -> FolderInfo {
    let p = std::path::Path::new(path.trim());
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !p.is_dir() {
        return FolderInfo {
            exists: false,
            is_repo: false,
            branch: String::new(),
            dirty: 0,
            name,
        };
    }
    let path = path.trim().to_string();
    match crate::git::toplevel(&path).await {
        Some(_) => {
            let st = crate::git::status(&path).await.unwrap_or_default();
            FolderInfo {
                exists: true,
                is_repo: true,
                branch: st.branch,
                dirty: st.entries.len() as u32,
                name,
            }
        }
        None => FolderInfo {
            exists: true,
            is_repo: false,
            branch: String::new(),
            dirty: 0,
            name,
        },
    }
}

#[tauri::command]
pub async fn create_project(
    core: CoreState<'_>,
    path: String,
    name: String,
    color: String,
    worktree_per_agent: bool,
    first_agent: Option<String>,
) -> Res<Project> {
    core.create_project(&path, &name, &color, worktree_per_agent, first_agent)
        .await
        .map_err(err)
}

#[tauri::command(async)]
pub fn update_project(core: CoreState, project: Project) -> Res<()> {
    core.update_project(project).map_err(err)
}

#[tauri::command(async)]
pub fn reorder_projects(core: CoreState, ids: Vec<String>) {
    core.reorder_projects(&ids);
}

#[tauri::command(async)]
pub fn remove_project(core: CoreState, id: String) -> Res<()> {
    core.remove_project(&id).map_err(err)
}

#[tauri::command]
pub async fn create_agent(
    core: CoreState<'_>,
    project_id: String,
    model: Option<String>,
) -> Res<AgentView> {
    core.create_agent(&project_id, model).await.map_err(err)
}

#[tauri::command]
pub fn warm_agent(core: CoreState, id: String) {
    core.warm(&id);
}

#[tauri::command(async)]
pub fn get_conversation(core: CoreState, id: String) -> Res<Vec<Value>> {
    // Buffered deltas are already in the snapshot: send them before it, not after.
    core.flush_conv();
    let h = core.agent(&id).map_err(err)?;
    let items = h.lock().conv.items();
    Ok(items)
}

#[tauri::command]
pub async fn send_message(
    core: CoreState<'_>,
    id: String,
    text: String,
    attachments: Vec<Attachment>,
) -> Res<()> {
    core.send_message(&id, text, attachments).await.map_err(err)
}

#[tauri::command]
pub async fn interrupt(core: CoreState<'_>, id: String) -> Res<()> {
    core.interrupt(&id).await.map_err(err)
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

#[tauri::command]
pub async fn rename_agent(core: CoreState<'_>, id: String, name: String) -> Res<()> {
    core.rename_agent(&id, &name).await.map_err(err)
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
    core.merge_agent(&id, squash, switch_to_base)
        .await
        .map_err(|e| match e.downcast_ref::<NotOnBase>() {
            // The one refusal the frontend turns into a question rather than an error.
            Some(refused) => refused.wire(),
            None => err(e),
        })
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
    core.git_sync(&project_id, SyncOp::Fetch).await.map_err(err)
}

#[tauri::command]
pub async fn git_pull(core: CoreState<'_>, project_id: String) -> Res<String> {
    core.git_sync(&project_id, SyncOp::Pull).await.map_err(err)
}

#[tauri::command]
pub async fn git_push(core: CoreState<'_>, project_id: String) -> Res<String> {
    core.git_sync(&project_id, SyncOp::Push).await.map_err(err)
}

#[tauri::command(async)]
pub fn stats(core: CoreState, range: String) -> StatsView {
    core.stats.query(&range)
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

// ---------- embedded editor ----------

#[tauri::command]
pub async fn fs_tree(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
) -> Res<fsedit::Tree> {
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    Ok(fsedit::tree(&root).await)
}

#[tauri::command]
pub async fn fs_read(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<fsedit::FileText> {
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    fsedit::read(std::path::Path::new(&root), &path).map_err(err)
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
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    let hash = fsedit::write(
        std::path::Path::new(&root),
        &path,
        &text,
        &eol,
        bom,
        expected_hash.as_deref(),
    )
    .map_err(err)?;
    core.git.refresh(&project_id);
    Ok(hash)
}

#[tauri::command]
pub async fn fs_create(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<()> {
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    fsedit::create(std::path::Path::new(&root), &path).map_err(err)?;
    core.git.refresh(&project_id);
    Ok(())
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

#[tauri::command(async)]
pub fn cancel_resume(core: CoreState, id: String) -> Res<()> {
    core.cancel_resume(&id).map_err(err)
}

// ---------- terminals ----------

#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn term_spawn(
    core: CoreState,
    project_id: String,
    shell: String,
    name: String,
    cols: u16,
    rows: u16,
    output: Channel<InvokeResponseBody>,
) -> Res<TermInfo> {
    let project = core.project(&project_id).map_err(err)?;
    let settings = core.settings.read().clone();
    let shells = pty::detect_shells(&settings);
    let sh = shells
        .iter()
        .find(|s| s.id == shell)
        .ok_or_else(|| format!("shell « {shell} » introuvable"))?;
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
            &project.path,
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
    Ok(info)
}

/// Runs one of the project's launch commands in its own terminal; it ends with the command
/// (TerminalExit carries its exit code). Stopped with term_kill, like a terminal.
/// `cursor_row` is the row of its log where it starts.
#[tauri::command(async)]
pub fn run_start(
    core: CoreState,
    project_id: String,
    command_id: String,
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
        .ok_or("commande de lancement introuvable")?;
    let settings = core.settings.read().clone();
    let shells = pty::detect_shells(&settings);
    // A command set up on another system (PowerShell on Windows, zsh on macOS) runs in the
    // default shell of this one.
    let sh = shells
        .iter()
        .find(|s| s.id == run.shell)
        .or_else(|| shells.first())
        .ok_or_else(|| format!("shell « {} » introuvable", run.shell))?;
    let cwd = pty::run_cwd(&project.path, &run.cwd).map_err(err)?;
    let info = TermInfo {
        id: new_id(),
        project_id,
        name: run.name.clone(),
        shell: sh.id.clone(),
    };
    let launch = Launch {
        info,
        shell: sh,
        cwd: &cwd,
        env: Vec::new(),
        command: &run.command,
    };
    spawn_launch(
        core.inner(),
        &settings,
        launch,
        (cols, rows),
        cursor_row,
        output,
    )
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

#[tauri::command]
pub fn term_write(core: CoreState, id: String, data: String) -> Res<()> {
    core.pty.write(&id, data.as_bytes()).map_err(err)
}

#[tauri::command]
pub fn term_resize(core: CoreState, id: String, cols: u16, rows: u16) -> Res<()> {
    core.pty.resize(&id, cols, rows).map_err(err)
}

#[tauri::command]
pub fn term_kill(core: CoreState, id: String) {
    core.pty.kill(&id);
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

#[tauri::command]
pub async fn set_remote_control(core: CoreState<'_>, id: String, enabled: bool) -> Res<()> {
    core.set_remote_control(&id, enabled).await.map_err(err)
}

// ---------- board ----------

#[tauri::command]
pub async fn ticket_create(
    core: CoreState<'_>,
    project_id: String,
    draft: TicketDraft,
) -> Res<Ticket> {
    core.ticket_create(&project_id, draft).await.map_err(err)
}

#[tauri::command(async)]
pub fn ticket_update(core: CoreState, id: String, draft: TicketDraft) -> Res<Ticket> {
    core.ticket_update(&id, draft).map_err(err)
}

#[tauri::command]
pub async fn ticket_delete(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_delete(&id).await.map_err(err)
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
    core.ticket_approve(&id).await.map_err(err)
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

/// Runs a step of an agent's recipe ("prep" or "run", its index) in its own read-only terminal,
/// like a launch command, with the system's default shell and the reserved ports exported. One
/// whose agent was archived, deleted or put to validation meanwhile is stopped at once, and the
/// error tells why.
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
    let sh = shells.first().ok_or("aucun shell détecté")?;
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
    Ok(info)
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

/// Looks for a newer release, through the network settings as they are now.
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    core: CoreState<'_>,
    updates: State<'_, crate::updates::Updates>,
) -> Res<Option<crate::updates::Found>> {
    let settings = core.settings.read().clone();
    crate::updates::check(&app, &settings, &updates)
        .await
        .map_err(err)
}

/// Downloads the update found as `id`, its signature checked, and keeps it until it installs:
/// true when it is the one ready now.
#[tauri::command]
pub async fn update_download(updates: State<'_, crate::updates::Updates>, id: u32) -> Res<bool> {
    updates.download(id).await.map_err(err)
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

// ---------- external ticket systems ----------

#[tauri::command]
pub async fn integration_connect(
    core: CoreState<'_>,
    service: Service,
    account: Account,
) -> Res<AccountView> {
    core.integration_connect(service, account)
        .await
        .map_err(err)
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
) -> Res<IssuePage> {
    let q = Query {
        text,
        filters,
        label: None,
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
    core.integration_import(&project_id, issues, max_loops)
        .await
        .map_err(err)
}
