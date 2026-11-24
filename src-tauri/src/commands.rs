//! Tauri commands invoked by the frontend, and by the remote API (`remoteapi`): each one is in the single list of
//! `remoteapi::list`, in its class.

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
use crate::remoteapi::{Maybe, State, TermChannel};
use crate::search;
use crate::stats::StatsView;
use crate::telemetry::{self, Event};
use crate::tickets::TicketDraft;
use ::tauri::ipc::Channel;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Tauri, as this file names it: `#[tauri::command]` is `escouade_command::command`, which gives each command to
/// Tauri's own attribute as it is and writes beside it its call by the remote API (`__remote_<name>`). A command is
/// written here as it always was; it goes in the list of `remoteapi::list`.
mod tauri {
    pub use ::tauri::*;
    pub use escouade_command::command;
}

type Res<T> = Result<T, String>;

/// A command's result, counted in the usage statistics when it went well (`telemetry`).
fn counted<T>(core: &Core, event: Event, r: Res<T>) -> Res<T> {
    if r.is_ok() {
        core.track(event);
    }
    r
}

/// The project a folder was added as, counted as created unless a closed project's history came
/// back with it (`Core::add_project`): that one is not new.
fn project_added(core: &Core, r: Res<(Project, bool)>) -> Res<Project> {
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
    // A file not opened (a program, an installer, a disk, a shortcut): the window offers its folder.
    if e.downcast_ref::<crate::sentfiles::NotOpened>().is_some() {
        return format!("not-opened|{text}");
    }
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
    /// The scheduled tasks of every project (ESC-448).
    scheduled: Vec<crate::scheduled::ScheduledView>,
    agents: Vec<AgentView>,
    ui: UiState,
    settings: Settings,
    usage: UsageSnapshot,
    git: HashMap<String, GitInfo>,
    shells: Vec<ShellInfo>,
    terminals: Vec<TermInfo>,
    /// The last terminals that ended (ESC-495): a client back after a cut missed their `TerminalExit`.
    exited_terminals: Vec<crate::hub::TermExit>,
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
    /// The harnesses whose plugin is off or not installed (ESC-391): not offered, though their program is found.
    engines_off: Vec<crate::model::EngineId>,
    /// The harnesses not on for a project (ESC-404), by project.
    harness_scope: BTreeMap<String, crate::harness_scope::HarnessScope>,
    /// The harnesses of a third party whose command does not run now (SEC-01, ESC-589).
    harness_launches: BTreeMap<crate::model::EngineId, crate::third_party::LaunchHeld>,
    sources: Vec<crate::integrations::source::SourceDecl>,
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
    /// What the page showed when its process last ended, for it to offer to open it again.
    page_crash: Option<crate::pagecrash::PageCrash>,
    /// What the policies of this device and a team lock (TS-11): the window greys what they impose.
    locks: crate::locks::Locks,
    /// The locks of each team's projects, by team id (TS-13).
    team_locks: std::collections::BTreeMap<String, crate::locks::Locks>,
    /// The files found unreadable at a start and recovered from their copies (ESC-469), until the user closes the
    /// notice that tells them.
    recovered: Vec<escouade_model::backups::Recovery>,
    /// The copies taken back by hand from the screen of the copies at the last starts (ESC-487), until the same notice
    /// closes.
    restored: Vec<escouade_model::backups::Restored>,
    /// The first launch's journey (ESC-504): open at a new installation's first start, or left half-way.
    onboarding: crate::onboarding::OnboardingView,
}

// Off the main thread: finding a `claude` of WSL may start its distribution (seconds).
#[tauri::command(async)]
pub fn subscribe(
    core: CoreState,
    updates: Maybe<crate::updates::Updates>,
    voice: Maybe<Arc<crate::voice::Voice>>,
    channel: Channel<UiEvent>,
) -> InitialState {
    // The window, or a client of the remote API (ESC-478): each gets the events, and its own state of the interface.
    let client = crate::remoteapi::current();
    let window = client == crate::remoteapi::Client::Window;
    core.hub.set_channel(channel);
    if window {
        // A page that loads again has no dictation under way: the one of the page before stops.
        if let Some(voice) = voice.get() {
            voice.cancel();
        }
        core.reset_unsaved();
        core.held.lock().clear();
    }
    // The updates are the window's: a client has its own (`docs/REMOTE.md`, « Ce qui change de machine »).
    let updates = updates.get().filter(|_| window);
    // One lock at a time, each guard ending with its statement: in a struct literal the guards
    // would live together (the lock order is projects before tickets, see `Core`).
    let mut settings = core.settings.read().clone();
    // SEC-10: the proxy's password kept in the file apart (no keychain) never leaves the backend;
    // `proxysecret::on_save` takes the backend's copy back anyway.
    settings.proxy_password_in_file.clear();
    let projects = core.projects.read().clone();
    let tickets = core.tickets.read().clone();
    let agents = core.agent_views();
    let scheduled = core.scheduled_views();
    let setup_output = core.setup_outputs();
    let ui = core.ui_of(&client);
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
    let version = core.host.version();
    let lang = core.lang();
    let claude_path_found = crate::engine::default()
        .locate(&settings.claude_path, &settings.wsl_distro)
        .is_some();
    InitialState {
        lang,
        projects,
        tickets,
        scheduled,
        agents,
        ui,
        shells: pty::detect_shells(&settings),
        claude_found: core.claude_found(),
        claude_path_found,
        settings,
        usage,
        git,
        terminals,
        exited_terminals: core.hub.exits(),
        installed: updates.and_then(|u| u.take_installed()),
        restart_at: updates.and_then(|u| u.restart_at()),
        failed_update: updates.and_then(|u| u.failed()),
        version,
        models,
        engine_models,
        account_models,
        engines: crate::engine::registry::declared(),
        engines_found: core.engines_found(),
        engines_off: core.engines_unplugged(),
        harness_scope: core.harness_scope(),
        harness_launches: core.harness_launches(),
        sources: crate::integrations::source::declared(),
        board_issues,
        autopilot_pause,
        project_pauses,
        accounts,
        setup_output,
        page_crash: if window { core.take_page_crash() } else { None },
        locks: core.locks.read().clone(),
        team_locks: core.team_locks.read().clone(),
        recovered: escouade_model::backups::notices(core.data.root()),
        restored: core.restored_backups(),
        onboarding: core.onboarding_view(),
    }
}

/// The notice of the files recovered at a start, closed by the user (ESC-469): forgotten, the files set aside and the
/// copies left as they are.
#[tauri::command(async)]
pub fn dismiss_recovery(core: CoreState) -> Res<()> {
    escouade_model::backups::dismiss(core.data.root()).map_err(|e| err(e.into()))?;
    escouade_model::backups::dismiss_restored(core.data.root()).map_err(|e| err(e.into()))
}

/// The screen of the copies (ESC-487): each file's copies, what they hold, the copy chosen for the next start, the
/// files set aside and the closed projects' histories to find again.
#[tauri::command(async)]
pub fn backups_view(core: CoreState) -> crate::copies::BackupsView {
    core.backups_view()
}

/// « Comparer »: what a copy gains and loses against the file of today; nothing written.
#[tauri::command(async)]
pub fn compare_backup(
    core: CoreState,
    file: String,
    name: String,
) -> Res<escouade_model::backup_view::Diff> {
    core.compare_backup(&file, &name).map_err(err)
}

/// « Faire une copie maintenant ».
#[tauri::command(async)]
pub fn back_up_now(core: CoreState) -> crate::copies::BackupsView {
    core.back_up_now();
    core.backups_view()
}

/// « Reprendre »: a closed project's history at once, any other file at the next start.
#[tauri::command(async)]
pub fn restore_backup(
    core: CoreState,
    file: String,
    name: String,
) -> Res<crate::copies::RestoreOutcome> {
    core.restore_backup(&file, &name).map_err(err)
}

/// The copy chosen for the next start called off.
#[tauri::command(async)]
pub fn cancel_restore(core: CoreState) -> Res<()> {
    core.cancel_restore().map_err(err)
}

/// « Redémarrer maintenant » to put the copy chosen in place: refused while an agent works.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn restart_for_restore(app: tauri::AppHandle, core: CoreState<'_>) -> Res<()> {
    let busy = core.agents_at_work();
    if busy > 0 {
        return Err(crate::copies::at_work(core.lang().ui, busy));
    }
    crate::updates::relaunch(app).await.map_err(err)
}

/// A link the window opens outside (the system's browser; Windows' under WSLg, `outside`): the
/// schemes Tauri's opener allowed it, http, https, mailto and tel.
#[tauri::command(async)]
pub fn open_url(core: CoreState<'_>, url: String) -> Res<()> {
    let scheme = url.split(':').next().unwrap_or("").to_ascii_lowercase();
    if !["http", "https", "mailto", "tel"].contains(&scheme.as_str()) {
        return Err(err(anyhow::anyhow!("not a link Escouade opens: {scheme}:")));
    }
    core.host
        .open(crate::host::Opened::Url(url), &[])
        .map_err(err)
}

/// A file or a folder the window opens outside (its program, the file manager; Windows' under
/// WSLg, `outside`): only inside the folders of the projects, their worktrees and the data folder,
/// where it really is (`opening`, audit SEC-12), and never a program or a script, as a file an agent
/// sent (ESC-171).
#[tauri::command(async)]
pub fn open_path(core: CoreState<'_>, path: String) -> Res<()> {
    open_path_in(&core, std::path::Path::new(&path)).map_err(err)
}

/// `open_path`.
pub(crate) fn open_path_in(core: &Core, path: &std::path::Path) -> anyhow::Result<()> {
    let roots = core.open_roots();
    let allowed = crate::opening::allow(&roots, path, false)?;
    if !allowed.real().is_dir() {
        let why = crate::sentfiles::runnable_path_why(path)
            .or_else(|| crate::sentfiles::runnable_path_why(allowed.real()));
        if let Some(why) = why {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return Err(crate::sentfiles::not_run(crate::i18n::ui(), &name, why));
        }
    }
    // Opened by the host, which judges it again where it is (a server publishes it to its client).
    core.host
        .open(crate::host::Opened::Path(path.to_path_buf()), &roots)
}

#[tauri::command]
pub fn set_ui(core: CoreState, ui: UiState) {
    // The window's, or a client's of the remote API: each keeps its own (ESC-478).
    core.set_ui_of(&crate::remoteapi::current(), ui);
}

/// The settings as the window saves them; the Claude accounts are left as they are (their tab
/// changes them with the `account_*` commands).
#[tauri::command(async)]
pub fn save_settings(
    core: CoreState,
    voice: Maybe<Arc<crate::voice::Voice>>,
    lsp: Maybe<Arc<crate::lsp::Manager>>,
    settings: Settings,
) -> Res<Vec<ShellInfo>> {
    let remote_was_on = core.settings.read().remote_control_all;
    core.save_window_settings(settings.clone()).map_err(err)?;
    // A feature switched off (F-1): the dictation and the conversation stop listening, the language servers stop.
    if !core.feature_on(crate::features::Feature::Voice) {
        if let Some(voice) = voice.get() {
            voice.cancel();
            voice.conversation_stop();
        }
    }
    if !core.feature_on(crate::features::Feature::Editor) {
        if let Some(lsp) = lsp.get() {
            lsp.shutdown();
        }
    }
    // An account saved (a code host's among them): the CI of the targets read at once (ESC-63).
    core.wake_ticket_prs();
    // A conversation under way takes the voice, its language, pace and volume (ESC-115).
    if let Some(voice) = voice.get() {
        voice.conversation_tune(
            &conv_choice(&core.settings.read(), &*core.secrets, &voice.catalog).0,
        );
    }
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
    // One installed since the app started is found (ESC-186).
    tokio::task::spawn_blocking(crate::syspath::refresh)
        .await
        .ok();
    crate::engine::registry::forget();
    let engines = core.engines().await;
    core.declare_in_harnesses();
    Ok(engines)
}

/// The harnesses the registry has now, as the window got them at its start (`InitialState::engines`): those the
/// plugins describe change with them (PLG-6).
#[tauri::command(async)]
pub fn engines_declared(core: CoreState) -> Vec<crate::engine::registry::EngineDecl> {
    core.sync_harnesses();
    crate::engine::registry::declared()
}

/// The harnesses found now, the PATH of today read first (ESC-186): the window back in front, the
/// menu « Harness » opened. A harness found since gets Escouade's server, and its accounts' models
/// are read.
#[tauri::command(async)]
pub fn engines_found_now(core: CoreState) -> Vec<crate::model::EngineId> {
    let before = core.engines_found();
    core.sync_harnesses();
    crate::syspath::refresh();
    let now = core.engines_found();
    if now != before {
        log::info!("harnesses found now: {now:?} (before: {before:?})");
        core.declare_in_harnesses();
        core.read_all_account_models();
    }
    now
}

/// The harnesses whose plugin is off or not installed now (ESC-391), after a plugin changed: the menus « Harness » do
/// not offer them.
#[tauri::command(async)]
pub fn engines_off_now(core: CoreState) -> Vec<crate::model::EngineId> {
    core.sync_harnesses();
    core.engines_unplugged()
}

/// What stopped the sign-in of an account before it was signed in, when it is known (ESC-133).
#[tauri::command(async)]
pub fn account_sign_in_ended(core: CoreState, id: String) -> Option<String> {
    core.sign_in_ended(&id)
}

/// Where Escouade's update of Claude Code stands (ESC-133), for « Réglages › Comptes et harness ».
#[tauri::command(async)]
pub fn claude_update_status(core: CoreState) -> crate::selfupdate::Status {
    crate::selfupdate::status(&core.settings.read())
}

/// The folder of Escouade's data (`~/.escouade`, another one for a development build or a sandbox), for
/// « Réglages › À propos › Données locales » and its « Ouvrir le dossier ».
#[tauri::command(async)]
pub fn data_dir(core: CoreState) -> String {
    core.data.root().display().to_string()
}

/// The installed plugins (ESC-173), as last read from their folders.
#[tauri::command(async)]
pub fn plugins_list(core: CoreState) -> Vec<crate::plugins::PluginView> {
    core.plugins.list(crate::i18n::ui())
}

/// What installing the plugin of a local folder would do (the install window, PLG-4): its copy
/// set aside until `plugin_install` or `plugin_discard`. « N'installer que des versions vérifiées » on, kept only when it
/// is the store's verified version of its id, the same files (SEC-01, ESC-589).
#[tauri::command(async)]
pub async fn plugin_inspect_folder(
    core: CoreState<'_>,
    path: String,
) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
    let lang = crate::i18n::ui();
    let preview = core
        .plugins
        .inspect_folder(std::path::Path::new(path.trim()), lang)?;
    core.held_to_store(preview, lang).await
}

/// The store as last read (STORE-9): its catalogue kept on this machine, when it was read, the
/// version this Escouade would install of each plugin, the revocation list's serial.
#[tauri::command(async)]
pub fn store_view(core: CoreState) -> crate::plugins::store::StoreView {
    core.store.view()
}

/// « Pour <projet> » (choice 215): up to 6 plugins of the catalogue that what Escouade sees on this
/// machine (a harness's program on the PATH) and in the project (ticket keys in its last 200
/// commits) calls for, none installed, each with its reason. Read here, nothing sent.
#[tauri::command(async)]
pub async fn store_suggestions(
    core: CoreState<'_>,
    project: Option<String>,
) -> Result<Vec<crate::plugins::store::Suggestion>, ()> {
    use crate::plugins::store;
    let Some(catalog) = core.store.view().catalog else {
        return Ok(Vec::new());
    };
    let names: Vec<String> = catalog
        .plugins
        .iter()
        .filter(|p| p.categories.iter().any(|c| c == "harness"))
        .flat_map(|p| p.keywords.clone())
        .collect();
    let programs = tauri::async_runtime::spawn_blocking(move || {
        names.into_iter().filter(|n| store::on_path(n)).collect()
    })
    .await
    .unwrap_or_default();
    let mut seen = store::Seen {
        programs,
        ticket_prefix: None,
    };
    if let Some(p) = project.and_then(|id| core.project(&id).ok()) {
        if let Ok(log) = crate::git::text(&p.path, &["log", "-200", "--format=%s%n%b"]).await {
            seen.ticket_prefix = store::ticket_prefix(&log);
        }
    }
    let installed: Vec<String> = core
        .plugins
        .list(crate::i18n::ui())
        .into_iter()
        .map(|p| p.id)
        .collect();
    Ok(store::suggest(
        &catalog,
        env!("CARGO_PKG_VERSION"),
        &seen,
        &installed,
        crate::i18n::ui(),
        6,
    ))
}

/// Reads the store's catalogue and revocation list again, now.
#[tauri::command(async)]
pub async fn store_refresh(core: CoreState<'_>) -> Result<crate::plugins::store::StoreView, ()> {
    core.refresh_store().await;
    Ok(core.store.view())
}

/// What installing version `version` of the store's plugin `id` would do (the last one this
/// Escouade can run when None): downloaded, every fingerprint and the signature checked, set aside
/// until `plugin_install` or `plugin_discard`. « N'installer que des versions vérifiées » refuses a
/// version without the seal.
#[tauri::command(async)]
pub async fn plugin_inspect_store(
    core: CoreState<'_>,
    id: String,
    version: Option<String>,
) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
    let lang = crate::i18n::ui();
    let settings = core.settings.read().clone();
    let http = crate::plugins::store::client(&settings)
        .map_err(|e| crate::plugins::Refusal::other(e.to_string()))?;
    let fetched = core
        .store
        .fetch(
            &http,
            &id,
            version.as_deref(),
            settings.plugins_verified_only,
            lang,
        )
        .await
        .map_err(crate::plugins::Refusal::other)?;
    core.plugins.inspect_store(fetched, lang)
}

/// What installing the seed's version of the plugin `id` would do (ESC-338): read from the installer, every fingerprint and
/// the signature checked, set aside until `plugin_install` or `plugin_discard`. No network.
#[tauri::command(async)]
pub fn plugin_inspect_seed(
    core: CoreState,
    id: String,
) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
    core.plugins.inspect_seed(&id, crate::i18n::ui())
}

/// « Ajouter un compte… » on `engine` (ESC-373): its plugin from the seed when it is not installed, with no network.
#[tauri::command(async)]
pub fn plugin_need_engine(core: CoreState, engine: EngineId) {
    core.need_engine_plugin(engine);
}

/// « Installer » of a plugin the seed proposes (ESC-373): installed at once from the seed, with no network, as the gesture
/// that needs it would; its view, or why the seed refuses it.
#[tauri::command(async)]
pub fn plugin_install_seed(
    core: CoreState,
    id: String,
) -> Result<Option<crate::plugins::PluginView>, crate::plugins::Refusal> {
    core.install_seed_plugin(&id)
}

/// What the app's seed holds that is not installed: what the user uninstalled, what it refused to install.
#[tauri::command(async)]
pub fn plugin_seed_offers(core: CoreState) -> Vec<crate::plugins::seed::Offer> {
    core.plugins.seed_offers(crate::i18n::ui())
}

/// What installing the plugin of a git repository at a tag would do: cloned and set aside. « N'installer que des versions
/// vérifiées » holds it as a folder (SEC-01, ESC-589).
#[tauri::command(async)]
pub async fn plugin_inspect_git(
    core: CoreState<'_>,
    url: String,
    tag: String,
) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
    let lang = crate::i18n::ui();
    let preview = core.plugins.inspect_git(&url, &tag, lang).await?;
    core.held_to_store(preview, lang).await
}

/// What loading a plugin's folder in development in place would do. A gesture of its own, said as such in the window:
/// « N'installer que des versions vérifiées » does not hold it (SEC-01, ESC-589).
#[tauri::command(async)]
pub fn plugin_inspect_dev(
    core: CoreState,
    path: String,
) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
    core.plugins
        .inspect_dev(std::path::Path::new(path.trim()), crate::i18n::ui())
}

/// The icon of an installed plugin (PLG-13), by its id alone: a `data:image/png` address, or
/// none (the window draws its letters). Réglages › Plugins and the store in the app show it.
#[tauri::command(async)]
pub fn plugin_icon(core: CoreState, id: String) -> Result<Option<String>, String> {
    core.plugins.icon(&id)
}

/// Installs what an inspection set aside, once the user agreed: in place of the same id (an
/// update), whose process stops and whose host is forgotten.
#[tauri::command(async)]
pub fn plugin_install(
    core: CoreState,
    token: String,
) -> Result<crate::plugins::PluginView, crate::plugins::Refusal> {
    let staged = core.plugins.staged(&token);
    let view = core.install_plugin(&token, crate::i18n::ui())?;
    // A run.js plugin's module compiled and precompiled now, in the cache, off this thread (ESC-320).
    core.plugin_runtime.prepare(&view.id);
    let from_store = matches!(view.origin, crate::plugins::Origin::Store { .. });
    let from_seed = matches!(
        view.origin,
        crate::plugins::Origin::Store { seed: true, .. }
    );
    core.track(crate::telemetry::Event::PluginInstalled { from_store });
    // The store's count, only with the user's consent to the statistics (docs/STORE.md §5.4); a version of the seed
    // through the counts the seed keeps, once (ESC-394).
    if from_seed {
        core.spawn_seed_counts();
    } else if from_store
        && core.settings.read().telemetry == Some(true)
        && crate::offline::allows(crate::offline::Flow::Statistics)
    {
        let settings = core.settings.read().clone();
        let (store, id, version) = (core.store.clone(), view.id.clone(), view.version.clone());
        let update = staged.is_some_and(|(_, update)| update);
        tauri::async_runtime::spawn(async move {
            if let Ok(http) = crate::plugins::store::client(&settings) {
                store.count(&http, &id, &version, update).await;
            }
        });
    }
    Ok(view)
}

/// Forgets what an inspection set aside: the user did not install it.
#[tauri::command(async)]
pub fn plugin_discard(core: CoreState, token: String) {
    core.plugins.discard(&token);
}

/// Uninstalls a plugin: its process stopped, its copy, its settings, its secrets and its journal
/// removed; `keep`: its settings and its secrets kept for a later install.
#[tauri::command(async)]
pub fn plugin_remove(core: CoreState, id: String, keep: Option<bool>) -> Result<bool, String> {
    core.plugin_uninstall(&id, keep.unwrap_or(false))
}

/// A plugin installed: `None` when it is not.
fn plugin_view(core: &CoreState, id: &str) -> Result<crate::plugins::PluginView, String> {
    core.plugins.get(id, crate::i18n::ui()).ok_or_else(|| {
        tr!(
            "Ce plugin n'est plus installé.",
            "This plugin is no longer installed."
        )
    })
}

/// The app's switch of a plugin: off, its process stops and nothing of it reaches a project.
#[tauri::command(async)]
pub fn plugin_set_enabled(
    core: CoreState,
    id: String,
    on: bool,
) -> Result<crate::plugins::PluginView, String> {
    core.plugins
        .choose(&id, |c| c.enabled = on)
        .map_err(|e| e.to_string())?;
    core.plugin_changed(&id);
    core.sync_voice_plugins();
    plugin_view(&core, &id)
}

/// A plugin's switch in a project (a private one included: the user sets it there one by one).
#[tauri::command(async)]
pub async fn plugin_set_project(
    core: CoreState<'_>,
    id: String,
    project: String,
    on: bool,
) -> Result<crate::plugins::PluginView, String> {
    // Off, the agents of its harnesses in the project stop (ESC-404).
    core.set_plugin_project(&id, &project, on)
        .await
        .map_err(|e| e.to_string())?;
    plugin_view(&core, &id)
}

/// « Activer » of a harness for a project (ESC-404): the user's agreement, asked by the window before the first agent
/// of a private project, or on the card of an agent or a ticket whose harness is off there. No tool does it.
#[tauri::command(async)]
pub fn harness_enable_in(core: CoreState, project: String, engine: EngineId) -> Res<()> {
    core.enable_harness_in(&project, engine).map_err(err)
}

/// The plugin a harness goes through (ESC-404): the window's question shows what it will get and its permissions.
#[tauri::command(async)]
pub fn harness_plugin_of(engine: EngineId) -> Option<String> {
    crate::plugins::harness::plugin_of(engine)
}

/// The whole command of a harness a plugin of a third party describes (audit SEC-01, ESC-589), for the agent `agent`
/// (its account's program) or the harness's own program: what the window of the agreement shows, and whether it runs
/// now. None for a harness that asks nothing (the app's, Escouade's own plugins').
#[tauri::command(async)]
pub fn harness_launch_ask(
    core: CoreState,
    engine: EngineId,
    agent: Option<String>,
) -> Res<Option<crate::third_party::LaunchAsk>> {
    core.sync_harnesses();
    core.launch_ask(engine, agent.as_deref()).map_err(err)
}

/// « Autoriser » or « Refuser » the command of a harness of a third party (the user's gesture, never a tool's): agreed,
/// the line whose fingerprint was read is kept and the agent starts; refused, nothing is kept and its conversation says
/// it does not start. The client that launches asks it (the window, or a client of the remote API).
#[tauri::command(async)]
pub fn harness_agree(
    core: CoreState,
    engine: EngineId,
    agent: Option<String>,
    fingerprint: String,
    allow: bool,
) -> Res<()> {
    core.agree_to_harness(engine, agent.as_deref(), &fingerprint, allow)
        .map_err(err)
}

/// The harnesses of a third party whose command does not run now (SEC-01): waiting for the user's agreement, or
/// refused by « Harness des plugins tiers ».
#[tauri::command(async)]
pub fn harness_launches(
    core: CoreState,
) -> std::collections::BTreeMap<EngineId, crate::third_party::LaunchHeld> {
    // A plugin installed or updated since: its harness in the registry first, as `engines_declared` does.
    core.sync_harnesses();
    core.harness_launches()
}

/// The harnesses that are not on, project by project (ESC-404): `off`, not offered; `asks`, offered behind the
/// question of a private project.
#[tauri::command(async)]
pub fn harness_scope(
    core: CoreState,
) -> std::collections::BTreeMap<String, crate::harness_scope::HarnessScope> {
    core.harness_scope()
}

/// Takes back a permission of a plugin, or gives it again: its process stops, and the next one
/// is given what is left (`plugins::granted`). Its program itself is not taken back: the plugin is
/// switched off instead.
#[tauri::command(async)]
pub fn plugin_set_permission(
    core: CoreState,
    id: String,
    word: String,
    granted: bool,
) -> Result<crate::plugins::PluginView, String> {
    let view = plugin_view(&core, &id)?;
    if !view.asks.contains(&word) || !crate::plugins::revocable(&word) {
        return Err(tr!(
            "Cette permission ne se retire pas : désactive le plugin à la place.",
            "This permission cannot be taken back: switch the plugin off instead."
        ));
    }
    core.plugins
        .choose(&id, |c| {
            c.revoked.retain(|w| w != &word);
            if !granted {
                c.revoked.push(word);
            }
        })
        .map_err(|e| e.to_string())?;
    core.plugin_changed(&id);
    plugin_view(&core, &id)
}

/// The value of a plugin's setting (one of `contributes.settings`); `null` puts it back to its
/// default.
#[tauri::command(async)]
pub fn plugin_set_setting(
    core: CoreState,
    id: String,
    key: String,
    value: serde_json::Value,
) -> Result<crate::plugins::PluginView, String> {
    let view = plugin_view(&core, &id)?;
    let known = view
        .manifest
        .as_ref()
        .and_then(|m| m.contributes.get("settings"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|s| {
            s.iter()
                .any(|s| s.get("key").and_then(|k| k.as_str()) == Some(&key))
        });
    if !known {
        return Err(format!("{key}: not a setting of {id}"));
    }
    core.plugins
        .choose(&id, |c| {
            if value.is_null() {
                c.settings.remove(&key);
            } else {
                c.settings.insert(key, value);
            }
        })
        .map_err(|e| e.to_string())?;
    plugin_view(&core, &id)
}

/// Which of a plugin's secrets are kept, and where (their values never leave the keychain or the file).
#[tauri::command(async)]
pub fn plugin_secrets(
    core: CoreState,
    id: String,
) -> Result<Vec<crate::plugin_secrets::SecretState>, String> {
    let view = plugin_view(&core, &id)?;
    let multiline = crate::plugin_secrets::multiline_secrets(view.manifest.as_ref());
    Ok(core.plugin_secret_states(&id, view.secret_keys, &multiline))
}

/// Keeps a plugin's secret in the keychain, under its own name, read back to check; empty forgets it. `inFile`: the
/// user chose « Garder dans un fichier lisible de toi seul » (ESC-499). Not kept, it says why
/// (`plugin_secrets::SecretRefused`).
#[tauri::command(async)]
pub fn plugin_set_secret(
    core: CoreState,
    id: String,
    key: String,
    value: String,
    in_file: Option<bool>,
) -> Result<(), crate::plugin_secrets::SecretRefused> {
    let view = plugin_view(&core, &id).map_err(|message| crate::plugin_secrets::SecretRefused {
        message,
        keychain_refused: false,
    })?;
    if !view.secret_keys.contains(&key) {
        return Err(crate::plugin_secrets::SecretRefused {
            message: format!("{key}: not a secret of {id}"),
            keychain_refused: false,
        });
    }
    core.plugin_secret_set(&id, &key, &value, in_file.unwrap_or(false))
}

/// The secrets of each profile of a plugin (ESC-530): kept or not, and where; never their values.
#[tauri::command(async)]
pub fn plugin_profile_secrets(
    core: CoreState,
    id: String,
) -> Result<Vec<crate::plugin_profiles::ProfileSecrets>, String> {
    core.plugin_profile_secrets(&id)
}

/// « Enregistrer » of a profile's window (ESC-530): its simple fields; a new one (`profile` none) is given its id here.
/// The plugin's view, and the profile's id.
#[tauri::command(async)]
pub fn plugin_profile_save(
    core: CoreState,
    id: String,
    kind: String,
    profile: Option<String>,
    values: serde_json::Map<String, serde_json::Value>,
) -> Result<(crate::plugins::PluginView, String), String> {
    let pid = core.plugin_profile_save(&id, &kind, profile.as_deref(), &values)?;
    Ok((plugin_view(&core, &id)?, pid))
}

/// « Garder » (empty: « Oublier ») of a secret field of a profile (ESC-530), under its own name in the keychain.
#[tauri::command(async)]
pub fn plugin_profile_secret(
    core: CoreState,
    id: String,
    kind: String,
    profile: String,
    key: String,
    value: String,
    in_file: Option<bool>,
) -> Result<(), crate::plugin_secrets::SecretRefused> {
    core.plugin_profile_secret(&id, &kind, &profile, &key, &value, in_file.unwrap_or(false))
}

/// « Supprimer le profil » (ESC-530): its secrets erased, the projects that took it with none. The plugin's view, and
/// those projects.
#[tauri::command(async)]
pub fn plugin_profile_delete(
    core: CoreState,
    id: String,
    kind: String,
    profile: String,
) -> Result<(crate::plugins::PluginView, crate::plugin_profiles::Deleted), String> {
    let gone = core.plugin_profile_delete(&id, &kind, &profile)?;
    Ok((plugin_view(&core, &id)?, gone))
}

/// A setting of each project of a plugin set on this machine (ESC-530); `value` absent: forgotten (« Reprendre la valeur
/// du fichier »).
#[tauri::command(async)]
pub fn plugin_project_set(
    core: CoreState,
    id: String,
    project: String,
    key: String,
    value: Option<serde_json::Value>,
    clear: Option<bool>,
) -> Result<crate::plugins::PluginView, String> {
    let value = if clear.unwrap_or(false) {
        None
    } else {
        Some(value.unwrap_or(serde_json::Value::Null))
    };
    core.plugin_project_set(&id, &project, &key, value)?;
    plugin_view(&core, &id)
}

/// « Écrire dans .escouade.yml » of a setting of each project of a plugin (ESC-530): written into the project's file,
/// key by key, its comments kept; forgotten on this machine.
#[tauri::command(async)]
pub fn plugin_project_write(
    core: CoreState,
    id: String,
    project: String,
    key: String,
) -> Result<(crate::plugins::PluginView, Project), String> {
    let p = core.plugin_project_write(&id, &project, &key)?;
    Ok((plugin_view(&core, &id)?, p))
}

/// « Déplacer vers le trousseau »: a plugin's secret kept in the file goes to the keychain, now that there is one.
#[tauri::command(async)]
pub fn plugin_secret_move(core: CoreState, id: String, key: String) -> Result<(), String> {
    core.plugin_secret_move(&id, &key)
}

/// Réglages › Plugins: the file of the plugins' secrets on this machine, if the user chose it.
#[tauri::command(async)]
pub fn secrets_file(core: CoreState) -> crate::plugin_secrets::SecretsFile {
    core.secrets_file()
}

/// The file of the plugins' secrets taken back: it goes, with what it holds.
#[tauri::command(async)]
pub fn secrets_file_remove(core: CoreState) -> Result<(), String> {
    core.secrets_file_remove()
}

/// Where a plugin's process stands, and its journal (the oldest entry first).
#[derive(serde::Serialize)]
pub struct PluginRun {
    status: crate::plugins::runtime::Status,
    journal: Vec<crate::plugins::journal::Entry>,
}

#[tauri::command(async)]
pub fn plugin_run(core: CoreState, id: String) -> PluginRun {
    PluginRun {
        status: core.plugin_runtime.status(&id),
        journal: core.plugin_runtime.journal(&id),
    }
}

/// Starts a plugin's process now (« Démarrer »), or again after it was stopped (« Relancer »).
#[tauri::command(async)]
pub fn plugin_start(core: CoreState, id: String) -> Result<PluginRun, String> {
    let rt = &core.plugin_runtime;
    let stopped = rt.status(&id).phase == crate::plugins::runtime::PhaseName::Stopped;
    if stopped {
        rt.restart(&id)
    } else {
        rt.start(&id)
    }
    .map_err(|e| e.to_string())?;
    Ok(plugin_run(core, id))
}

/// What « Rechercher des mises à jour » found: the plugins as they are now, and the store as it was read (`readAt`,
/// `error` when it did not answer: the offers of `readAt` were looked at).
#[derive(serde::Serialize)]
pub struct UpdatesLooked {
    plugins: Vec<crate::plugins::PluginView>,
    store: crate::plugins::store::StoreView,
}

/// Looks for newer versions of the installed plugins (offered, never installed), the store read again first (ESC-405).
#[tauri::command(async)]
pub async fn plugins_check_updates(core: CoreState<'_>) -> Result<UpdatesLooked, ()> {
    core.look_for_updates().await;
    Ok(UpdatesLooked {
        plugins: core.plugins.list(crate::i18n::ui()),
        store: core.store.view(),
    })
}

/// The user agrees to the commands a plugin runs on facts, as they are now, or takes the
/// agreement back (`on`: false), which stops them (PLG-5). Gives the plugin as it is now.
#[tauri::command]
pub fn plugin_approve_commands(
    core: CoreState,
    id: String,
    on: bool,
) -> Result<crate::plugins::PluginView, String> {
    core.plugins
        .approve_commands(&id, on)
        .map_err(|e| e.to_string())?;
    plugin_view(&core, &id)
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
    // Asked once a path is saved: every `claude` of WSL is looked for again (installed, removed),
    // and the PATH of today read (ESC-186: « Chercher à nouveau »).
    crate::wsl::forget(true);
    crate::syspath::refresh();
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
    core.shareable(engine.unwrap_or(EngineId::CLAUDE))
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
    let engine = engine.unwrap_or(EngineId::CLAUDE);
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

/// The menu of the models of the account `id` opened (ESC-186): read again when none were, or a
/// day ago (`Core::refresh_account_models`), in the background; the window is told by an event.
#[tauri::command]
pub fn account_models_refresh(core: CoreState, id: String) {
    let core = core.inner().clone();
    tauri::async_runtime::spawn(async move { core.refresh_account_models(&id).await });
}

/// Whether the account is signed in, with which email, and its folder.
#[tauri::command]
pub async fn account_status(core: CoreState<'_>, id: String) -> Res<AccountStatus> {
    core.claude_account_status(&id).await.map_err(err)
}

/// « Clé d’API » of a Cursor account (ESC-43): kept in the system keychain, its status as it then
/// is. Never logged.
#[tauri::command]
pub async fn account_key(core: CoreState<'_>, id: String, key: String) -> Res<AccountStatus> {
    core.set_account_key(&id, &key).await.map_err(err)
}

/// « Se connecter… »: the account's `claude` in an interactive terminal (written to, resized and
/// killed as the others: `term_write`, `term_resize`, `term_kill`).
#[tauri::command(async)]
pub fn account_login(
    core: CoreState,
    id: String,
    cols: u16,
    rows: u16,
    output: TermChannel,
) -> Res<TermInfo> {
    core.claude_login(&id, (cols, rows), crate::termlog::feed(output.sink()))
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
    output: TermChannel,
) -> Res<TermInfo> {
    core.open_agent_terminal_to(&id, (cols, rows), manual.unwrap_or(false), output.sink())
        .await
        .map_err(err)
}

/// Whether the agent's interface asks the question about the trust of its folder (Codex, Copilot
/// CLI), which the user answers in its terminal.
#[tauri::command(async)]
pub fn tui_asks_trust(core: CoreState, id: String) -> Res<bool> {
    core.tui_asks_trust(&id).map_err(err)
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
    /// Its repositories, when it holds several (ESC-169); empty otherwise.
    repos: Vec<String>,
    /// Those of `repos` without any commit: no worktree can start from them.
    repos_without_commit: Vec<String>,
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
            repos: Vec::new(),
            repos_without_commit: Vec::new(),
        });
    }
    let path = path.trim().to_string();
    let history = core
        .closed_project_info_for(&path, history_id.as_deref())
        .await;
    let repos = crate::repos::detect(p, &crate::paths::worktrees_root(&path, "")).await;
    if !repos.is_empty() {
        let mut repos_without_commit = Vec::new();
        for r in &repos {
            if !crate::git::has_commit(&crate::repos::join(&path, r)).await {
                repos_without_commit.push(r.clone());
            }
        }
        return Ok(FolderInfo {
            exists: true,
            is_repo: true,
            branch: String::new(),
            dirty: 0,
            name,
            history,
            repos,
            repos_without_commit,
        });
    }
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
                repos: Vec::new(),
                repos_without_commit: Vec::new(),
            }
        }
        None => FolderInfo {
            exists: true,
            is_repo: false,
            branch: String::new(),
            dirty: 0,
            name,
            history,
            repos: Vec::new(),
            repos_without_commit: Vec::new(),
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
    let missing_dirs: Vec<String> = dirs
        .iter()
        .filter(|d| !std::path::Path::new(&path).join(d.trim()).is_dir())
        .cloned()
        .collect();
    // Of several repositories (ESC-169): tickets start once one of them has a commit; the branches
    // offered are those of the first that has one.
    let repos = crate::repos::detect(
        std::path::Path::new(&path),
        &crate::paths::worktrees_root(&path, &worktree_dir),
    )
    .await;
    if !repos.is_empty() {
        for r in &repos {
            let dir = crate::repos::join(&path, r);
            if crate::git::has_commit(&dir).await {
                return Ok(SetupInfo {
                    has_commit: true,
                    branches: crate::git::branches(&dir).await.unwrap_or_default(),
                    missing_dirs,
                    worktrees_tracked: false,
                });
            }
        }
        return Ok(SetupInfo {
            has_commit: false,
            branches: Vec::new(),
            missing_dirs,
            worktrees_tracked: false,
        });
    }
    let repo = crate::git::toplevel(&path).await.is_some();
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

/// « Rechercher à nouveau » (ESC-169): the project's repositories looked for again; the project.
#[tauri::command]
pub async fn refresh_repos(core: CoreState<'_>, id: String) -> Res<Project> {
    core.refresh_repos(&id, false).await.map_err(err)?;
    core.project(&id).map_err(err)
}

/// The workflows of the project (ESC-192): the last reading, read again if a file it read changed.
#[tauri::command]
pub async fn get_workflows(core: CoreState<'_>, id: String) -> Res<crate::workflow::Scan> {
    core.refresh_workflows(&id, false).await.map_err(err)
}

/// Where each agent of the project is in its workflows, and where it has been (ESC-193).
#[tauri::command]
pub fn get_workflow_tracks(core: CoreState, id: String) -> Vec<crate::workflow_track::TrackView> {
    core.workflow_tracks(&id)
}

/// « Relire » the project's workflows (ESC-192).
#[tauri::command]
pub async fn refresh_workflows(core: CoreState<'_>, id: String) -> Res<crate::workflow::Scan> {
    core.refresh_workflows(&id, true).await.map_err(err)
}

/// What « Affiner… » will do, said before (ESC-196): the model, the account, the cost estimated.
#[tauri::command]
pub async fn workflow_refine_plan(
    core: CoreState<'_>,
    id: String,
) -> Res<crate::workflowrefine::RefinePlan> {
    core.workflow_refine_plan(&id).await.map_err(err)
}

/// « Affiner… »: a short agent refines the project's workflows, the rules check what it says
/// (ESC-196).
#[tauri::command]
pub async fn refine_workflows(
    core: CoreState<'_>,
    id: String,
) -> Res<crate::workflowrefine::Outcome> {
    core.refine_workflows(&id).await.map_err(err)
}

/// The project's workflows as the user corrected them (ESC-195): what follows the agents reads.
#[tauri::command]
pub async fn effective_workflows(
    core: CoreState<'_>,
    id: String,
) -> Res<Vec<crate::workflow::Workflow>> {
    let scan = core.refresh_workflows(&id, false).await.map_err(err)?;
    let fixes = core.project(&id).map_err(err)?.workflow_fixes;
    Ok(crate::workflowfix::effective(&scan.workflows, &fixes))
}

/// « Retirer du dépôt » (ESC-195): the file of a workflow this machine does not read goes.
#[tauri::command]
pub async fn remove_workflow_file(
    core: CoreState<'_>,
    id: String,
    workflow: String,
) -> Res<crate::workflow::Scan> {
    core.remove_workflow_file(&id, &workflow).await.map_err(err)
}

/// « Supprimer » a workflow (`removed`: its file goes, never written again), « Laisser supprimé »
/// one proposed again, or « Rétablir » it (ESC-309).
#[tauri::command]
pub async fn set_workflow_removed(
    core: CoreState<'_>,
    id: String,
    workflow: String,
    removed: bool,
) -> Res<crate::workflow::Scan> {
    core.set_workflow_removed(&id, &workflow, removed)
        .await
        .map_err(err)
}

/// « Oublier » a repository gone from the project's folder (ESC-169).
#[tauri::command(async)]
pub fn forget_repo(core: CoreState, id: String, repo: String) -> Res<Project> {
    core.forget_repo(&id, &repo).map_err(err)
}

/// « Choisir une image… » (`path`) or « Retirer » (None) of the project's logo (ESC-167).
#[tauri::command(async)]
pub fn set_project_logo(core: CoreState, id: String, path: Option<String>) -> Res<Project> {
    core.set_project_logo(&id, path.as_deref()).map_err(err)
}

/// The project's logo from an image's bytes, raw (ESC-538): on a server, the image chosen on this computer is sent,
/// never its path. Headers: `x-project`, `x-name` (the file's name, for what is said).
#[tauri::command(async)]
pub fn set_project_logo_bytes(core: CoreState, request: crate::remoteapi::Body) -> Res<Project> {
    let Some(bytes) = request.raw() else {
        return Err(tr!(
            "Les octets d’un logo viennent bruts, pas en JSON.",
            "The bytes of a logo come raw, not as JSON."
        ));
    };
    let Some(id) = header(&request, "x-project") else {
        return Err(tr!("Projet non dit.", "No project said."));
    };
    let name = header(&request, "x-name").unwrap_or_else(|| "logo".into());
    core.set_project_logo_bytes(&id, &name, bytes.to_vec())
        .map_err(err)
}

/// « Recherche par le sens » (ESC-254): the vectors' model, whether it is downloaded.
#[tauri::command(async)]
pub fn vectors_model(core: CoreState) -> crate::vectors::wiring::VectorsModelInfo {
    core.vectors_model_info()
}

/// Downloads the vectors' model (its progress in `UiEvent::VectorsDownload`); « Annuler » ends it with what came kept.
#[tauri::command]
pub async fn vectors_model_download(
    core: CoreState<'_>,
) -> Res<crate::vectors::wiring::VectorsModelInfo> {
    core.vectors_model_download().await.map_err(err)
}

#[tauri::command]
pub fn vectors_model_cancel(core: CoreState) {
    core.vectors_model_cancel();
}

#[tauri::command(async)]
pub fn vectors_model_delete(core: CoreState) -> Res<crate::vectors::wiring::VectorsModelInfo> {
    core.vectors_model_delete().map_err(err)
}

/// The project's logo for its tile, as a `data:` URL; null when it has none.
#[tauri::command(async)]
pub fn project_logo(core: CoreState, id: String) -> Option<String> {
    core.project_logo(&id)
}

/// « Un dossier du serveur » (ESC-538): a folder of the core's machine listed for « Nouveau projet », its folders only
/// (repository and branch, project already), the room left on its disk; without a path, where the browser opens.
#[tauri::command]
pub async fn fs_browse(
    core: CoreState<'_>,
    path: Option<String>,
) -> Res<crate::serverfolders::Listing> {
    // The e2e's « home » (a build of development): a folder of the test's, never the user's.
    let test_home = cfg!(debug_assertions)
        .then(|| std::env::var_os("ESCOUADE_BROWSE_HOME").filter(|v| !v.is_empty()))
        .flatten()
        .map(std::path::PathBuf::from);
    let home = test_home
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));
    let projects: Vec<(String, String)> = core
        .projects
        .read()
        .iter()
        .map(|p| (p.path.clone(), p.name.clone()))
        .collect();
    tokio::task::spawn_blocking(move || {
        let at = match path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            Some(p) => std::path::PathBuf::from(p),
            None => crate::serverfolders::start(&home),
        };
        crate::serverfolders::list(&at, &home, &projects)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// « Cloner un dépôt » (ESC-538): `url` (https or ssh) cloned into `parent/folder` on the core's machine; the folder.
#[tauri::command]
pub async fn clone_repo(url: String, parent: String, folder: String) -> Res<String> {
    crate::serverfolders::clone(&url, std::path::Path::new(parent.trim()), &folder, &[])
        .await
        .map_err(err)
}

/// The image of `path` as a logo would be taken, as a `data:` URL: the preview of a logo chosen
/// before it is saved, or why it would be refused.
#[tauri::command(async)]
pub fn read_logo(path: String) -> Res<String> {
    crate::projectlogo::read_in(crate::i18n::ui(), std::path::Path::new(path.trim()))
        .map(|l| crate::projectlogo::data_url(l.media_type, &l.bytes))
        .map_err(err)
}

/// How many certificates the `.pem` file of « Autorités de certification supplémentaires »
/// holds, or why it would not be read (`crate::tls`).
#[tauri::command(async)]
pub fn check_authorities(path: String) -> Res<usize> {
    crate::tls::read_authorities(path.trim())
        .map(|c| c.len())
        .map_err(err)
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

/// « Parties du projet » (ESC-658): this device's choice for a part of the project (`on` none: « Reprendre le défaut »,
/// the repository's or on). Refused when its team imposes it.
#[tauri::command(async)]
pub fn project_set_part(
    core: CoreState,
    project_id: String,
    part: crate::parts::Part,
    on: Option<bool>,
) -> Res<Project> {
    core.set_project_part(&project_id, part, on).map_err(err)
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

/// A folder of agents; with `agent_id`, it takes that agent's place and holds it.
#[tauri::command(async)]
pub fn agent_folder_create(
    core: CoreState,
    project_id: String,
    agent_id: Option<String>,
) -> Res<AgentFolder> {
    let agents: Vec<String> = agent_id.iter().cloned().collect();
    core.folder_create(&project_id, None, &agents, None, agent_id.as_deref())
        .map_err(err)
}

#[tauri::command(async)]
pub fn agent_folder_rename(
    core: CoreState,
    project_id: String,
    folder_id: String,
    name: String,
) -> Res<()> {
    core.folder_rename(&project_id, &folder_id, &name)
        .map_err(err)
}

#[tauri::command(async)]
pub fn agent_folder_delete(core: CoreState, project_id: String, folder_id: String) -> Res<()> {
    core.folder_delete(&project_id, &folder_id).map_err(err)
}

#[tauri::command(async)]
pub fn agent_folder_principal(
    core: CoreState,
    project_id: String,
    folder_id: String,
    agent_id: Option<String>,
) -> Res<()> {
    core.folder_principal(&project_id, &folder_id, agent_id.as_deref())
        .map_err(err)
}

#[tauri::command(async)]
pub fn agent_folder_fold(
    core: CoreState,
    project_id: String,
    folder_id: String,
    folded: bool,
) -> Res<()> {
    core.folder_fold(&project_id, &folder_id, folded)
        .map_err(err)
}

#[tauri::command(async)]
pub fn agent_folder_move(
    core: CoreState,
    project_id: String,
    item_id: String,
    folder_id: Option<String>,
    index: Option<usize>,
) -> Res<()> {
    core.item_move(&project_id, &item_id, folder_id.as_deref(), index)
        .map_err(err)
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
    let files = files.unwrap_or_default();
    // Sent by hand: what the proposals at the right moment notice of it, for the features that are off (F-3).
    let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    core.offer_note_message(&id, &text, &names);
    let r = core
        .send_message_files(&id, text, attachments, files)
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
pub fn attach_chunk(core: CoreState, request: crate::remoteapi::Body) -> Res<()> {
    let Some(bytes) = request.raw() else {
        return Err(tr!(
            "Les octets d’une copie viennent bruts, pas en JSON.",
            "The bytes of a copy come raw, not as JSON."
        ));
    };
    let handle = request.header("x-handle").unwrap_or_default();
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
pub async fn clipboard_image(core: CoreState<'_>) -> Res<tauri::ipc::Response> {
    core.host
        .clipboard_image()
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

/// `/login` typed to the agent (ESC-401): its account was just signed in; it goes on, on it,
/// with its session.
#[tauri::command]
pub async fn agent_signed_in(core: CoreState<'_>, id: String) -> Res<()> {
    core.agent_signed_in(&id).await.map_err(err)
}

/// `/logout` typed to the agent (ESC-401), confirmed: its account signs out by its harness.
#[tauri::command]
pub async fn agent_logout(core: CoreState<'_>, id: String) -> Res<()> {
    core.agent_logout(&id).await.map_err(err)
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

/// « Refuser » the deletion its maker asked for (`delete_agent`, ESC-511): the agent stays.
#[tauri::command]
pub fn refuse_delete(core: CoreState<'_>, id: String) -> Res<()> {
    core.refuse_delete(&id).map_err(err)
}

/// What deleting the agent's worktree would lose, as git says it in time (« Supprimer l'agent »).
#[tauri::command]
pub async fn delete_loss(core: CoreState<'_>, id: String) -> Res<crate::deleteask::Loss> {
    core.delete_loss(&id, crate::deleteask::LOSS_TIMEOUT)
        .await
        .map_err(err)
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

/// What merging the agent would do in each of its repositories (ESC-169).
#[tauri::command]
pub async fn agent_merge_check(
    core: CoreState<'_>,
    id: String,
) -> Res<Vec<crate::multiwt::RepoMerge>> {
    core.agent_merge_check(&id).await.map_err(err)
}

/// « Merger api seul » (ESC-169): the merge of some of the agent's repositories.
#[tauri::command]
pub async fn merge_agent_repos(
    core: CoreState<'_>,
    id: String,
    squash: bool,
    switch_to_base: bool,
    repos: Vec<String>,
) -> Res<String> {
    let r = core
        .merge_agent_repos(&id, squash, switch_to_base, repos)
        .await
        .map_err(|e| match e.downcast_ref::<NotOnBase>() {
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
    // A harness's own few (ESC-205: Codex's `/compact`): no process started for them.
    let fixed = crate::engine::fixed_commands(crate::engine::of(&h.lock().meta));
    if !fixed.is_empty() {
        return Ok(fixed);
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
    repo: Option<String>,
) -> Res<GitLog> {
    core.git_log(&project_id, agent_id, repo).await.map_err(err)
}

#[tauri::command]
pub async fn git_show(
    core: CoreState<'_>,
    project_id: String,
    hash: String,
    repo: Option<String>,
) -> Res<String> {
    core.git_show(&project_id, &hash, repo).await.map_err(err)
}

#[tauri::command]
pub async fn git_fetch(
    core: CoreState<'_>,
    project_id: String,
    repo: Option<String>,
) -> Res<String> {
    let r = core
        .git_sync_in(&project_id, repo.as_deref(), SyncOp::Fetch)
        .await
        .map_err(err);
    counted(&core, Event::GitSync { op: "fetch" }, r)
}

#[tauri::command]
pub async fn git_pull(
    core: CoreState<'_>,
    project_id: String,
    repo: Option<String>,
) -> Res<String> {
    let r = core
        .git_sync_in(&project_id, repo.as_deref(), SyncOp::Pull)
        .await
        .map_err(err);
    counted(&core, Event::GitSync { op: "pull" }, r)
}

#[tauri::command]
pub async fn git_push(
    core: CoreState<'_>,
    project_id: String,
    repo: Option<String>,
) -> Res<String> {
    let r = core
        .git_sync_in(&project_id, repo.as_deref(), SyncOp::Push)
        .await
        .map_err(err);
    // A push of Escouade: the pull requests of the tickets « En revue » are read at once (ESC-59).
    if r.is_ok() {
        core.wake_ticket_prs();
    }
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
    repo: Option<String>,
) -> Res<Vec<crate::git::BranchInfo>> {
    core.branches_in(&project_id, repo.as_deref())
        .await
        .map_err(err)
}

/// Checks a name for a new branch: as git does, and not taken already.
#[tauri::command]
pub async fn branch_check(
    core: CoreState<'_>,
    project_id: String,
    name: String,
    repo: Option<String>,
) -> Res<()> {
    core.branch_check_in(&project_id, repo.as_deref(), &name)
        .await
        .map_err(err)
}

/// Switches the project's folder to a branch; the stash's name when its changes were put aside.
#[tauri::command]
pub async fn branch_switch(
    core: CoreState<'_>,
    project_id: String,
    name: String,
    stash: bool,
    repo: Option<String>,
) -> Res<Option<String>> {
    core.branch_switch_in(&project_id, repo.as_deref(), &name, stash)
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
    repo: Option<String>,
) -> Res<Option<String>> {
    let r = core
        .branch_create_in(&project_id, repo.as_deref(), &name, &start, switch, stash)
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
    repo: Option<String>,
) -> Res<Option<String>> {
    core.branch_delete_in(&project_id, repo.as_deref(), &name, remote, force)
        .await
        .map_err(branch_err)
}

/// The local branches « Branches mergées » offers to delete.
#[tauri::command]
pub async fn branches_merged(
    core: CoreState<'_>,
    project_id: String,
    repo: Option<String>,
) -> Res<Vec<String>> {
    core.branches_merged_in(&project_id, repo.as_deref())
        .await
        .map_err(err)
}

/// The diff from `a` to `b`, two branches or commits of the project's repository.
#[tauri::command]
pub async fn git_diff_refs(
    core: CoreState<'_>,
    project_id: String,
    a: String,
    b: String,
    repo: Option<String>,
) -> Res<String> {
    core.diff_refs_in(&project_id, repo.as_deref(), &a, &b)
        .await
        .map_err(err)
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

/// The reviews written by an agent of a project (ESC-66).
#[tauri::command]
pub fn pr_reviews(core: CoreState<'_>, project_id: String) -> Vec<crate::prreview::PrReview> {
    core.pr_reviews_of(&project_id)
}

/// « Faire relire par un agent » (ESC-66): the review starts; nothing leaves for the host.
#[tauri::command]
pub async fn pr_review_start(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
) -> Res<crate::prreview::PrReview> {
    core.pr_review_start(&project_id, number).await.map_err(err)
}

/// A remark of an agent's review kept, dropped (its thread resolved), opened again or corrected.
#[tauri::command]
pub fn pr_review_note(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    id: u32,
    choice: crate::prreview::NoteChoice,
    body: Option<String>,
) -> Res<crate::prreview::PrReview> {
    core.pr_review_note(&project_id, number, id, choice, body)
        .map_err(err)
}

/// « Publier » (ESC-490): the only way a remark of an agent reaches the host, as a comment of the
/// user, on their click.
#[tauri::command]
pub async fn pr_review_send(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    notes: Vec<crate::prreview::NoteSend>,
) -> Res<crate::prreview::Sent> {
    core.pr_review_send(&project_id, number, notes)
        .await
        .map_err(err)
}

/// « Ta relecture » (ESC-494): the user's review waiting on a pull request, its remarks placed against the diff read last.
#[tauri::command]
pub fn pr_pending(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
) -> crate::myreview::PendingRead {
    core.pr_pending(&project_id, number, host_repo.as_deref().unwrap_or(""))
}

/// A remark of the user's review added, written again or taken out, its text, or all of it given up: kept on this machine,
/// nothing sent to the host.
#[tauri::command]
pub fn pr_pending_change(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
    change: crate::myreview::PendingChange,
) -> Res<crate::myreview::PendingRead> {
    core.pr_pending_change(
        &project_id,
        number,
        host_repo.as_deref().unwrap_or(""),
        change,
    )
    .map_err(err)
}

/// « Publier » of « Ta relecture »: the user's review in one review of the host, its verdict, its text and its remarks, on
/// their click. Only the window calls it: no tool of the MCP server, no agent.
#[tauri::command]
pub async fn pr_review_submit(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
    ask: crate::myreview::ReviewAsk,
) -> Res<crate::prview::Read<crate::myreview::Submitted>> {
    Ok(core
        .pr_review_submit(&project_id, number, host_repo.as_deref().unwrap_or(""), ask)
        .await)
}

/// A review dropped.
#[tauri::command]
pub fn pr_review_discard(core: CoreState<'_>, project_id: String, number: u64) {
    core.pr_review_discard(&project_id, number);
}

/// « Lancer un agent sur cette PR » (ESC-65): an agent in a worktree on the branch of the project's
/// open pull request `number`, a fork's too.
#[tauri::command]
pub async fn create_agent_on_pr(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
) -> Res<AgentView> {
    let agent = core
        .create_agent_on_pr(&project_id, number)
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

/// Switches the agent's worktree to a branch (the status bar's picker); the branch it is on.
#[tauri::command]
pub async fn agent_branch_switch(core: CoreState<'_>, id: String, name: String) -> Res<String> {
    core.agent_branch_switch(&id, &name)
        .await
        .map_err(branch_err)
}

/// Fetch, pull or push (`op`) of the agent's own folder; a summary for the user.
#[tauri::command]
pub async fn agent_git_sync(core: CoreState<'_>, id: String, op: String) -> Res<String> {
    let (sync, op) = match op.as_str() {
        "pull" => (SyncOp::Pull, "pull"),
        "push" => (SyncOp::Push, "push"),
        _ => (SyncOp::Fetch, "fetch"),
    };
    let r = core.agent_git_sync(&id, sync).await.map_err(err);
    counted(&core, Event::GitSync { op }, r)
}

/// « L'agent corrige » / « L'agent corrige les retours » on pull request `number`.
#[tauri::command]
pub async fn agent_pr_fix(
    core: CoreState<'_>,
    id: String,
    number: u64,
    fix: crate::agentprs::PrFix,
    // The repository it was read in, when not the target's (`PullRequest::host_repo`).
    host_repo: Option<String>,
) -> Res<()> {
    core.agent_pr_fix(&id, number, host_repo.as_deref().unwrap_or(""), fix)
        .await
        .map_err(err)
}

/// The « PR et CI » tab (ESC-305): the checks, reviews, comments and deployments of the agent's
/// pull request `number`, or why the host did not give them.
#[tauri::command]
pub async fn agent_pr_detail(
    core: CoreState<'_>,
    id: String,
    number: u64,
    host_repo: Option<String>,
) -> Res<crate::prview::Read<crate::prview::PrDetail>> {
    Ok(core
        .agent_pr_detail(&id, number, host_repo.as_deref().unwrap_or(""))
        .await)
}

/// A pull request of the project read whole (ESC-452): its description, commits, files with their
/// lines (git's) and threads, kept while it does not change; or why the host did not give it. Its
/// texts are a third party's (`PrContents::third_party`).
#[tauri::command]
pub async fn pr_contents(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    // The repository it was read in, when not the target's (`PullRequest::host_repo`).
    host_repo: Option<String>,
) -> Res<crate::prview::Read<crate::prread::PrContents>> {
    Ok(core
        .pr_contents(
            &project_id,
            number,
            host_repo.as_deref().unwrap_or(""),
            true,
        )
        .await)
}

/// The files of one commit of a pull request of the project (ESC-467, its tab « Fichiers »): its
/// diff from its first parent, git's; or why not (a commit the host does not list for it).
#[tauri::command]
pub async fn pr_commit_files(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
    sha: String,
) -> Res<crate::prview::Read<Vec<crate::prread::PrFileRead>>> {
    Ok(core
        .pr_commit_files(
            &project_id,
            number,
            host_repo.as_deref().unwrap_or(""),
            &sha,
        )
        .await)
}

/// A write on a pull request of the project, on the user's click in its view (ESC-474): a comment of the
/// conversation or on lines, an answer, a thread resolved or opened again, in the user's name on its host. Only the
/// window calls it: no tool of the MCP server, no agent.
#[tauri::command]
pub async fn pr_write(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
    write: crate::prwrite::PrWrite,
) -> Res<crate::prview::Read<bool>> {
    Ok(core
        .pr_write(
            &project_id,
            number,
            host_repo.as_deref().unwrap_or(""),
            write,
        )
        .await)
}

/// « Merger », confirmed in the merge box of a pull request's view (ESC-488): merged by its host in the user's name with
/// the strategy chosen, its ticket « Terminé », its branch and its agent's worktree removed when asked. Only the window
/// calls it: no tool of the MCP server, no agent.
#[tauri::command]
pub async fn pr_merge(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
    ask: crate::prwrite::MergeAsk,
) -> Res<crate::prview::Read<crate::prwrite::Merged>> {
    Ok(core
        .pr_merge(&project_id, number, host_repo.as_deref().unwrap_or(""), ask)
        .await)
}

/// The merge box of a pull request's view (ESC-488): where it stands for its merge, how it may merge.
#[tauri::command]
pub async fn pr_control(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
) -> Res<crate::prview::Read<crate::prwrite::PrControlRead>> {
    Ok(core
        .pr_control(&project_id, number, host_repo.as_deref().unwrap_or(""))
        .await)
}

/// Who may be asked for a review and which labels may be put, for the pickers of a pull request's view (ESC-488).
#[tauri::command]
pub async fn pr_candidates(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
) -> Res<crate::prview::Read<crate::forge::Candidates>> {
    Ok(core
        .pr_candidates(&project_id, number, host_repo.as_deref().unwrap_or(""))
        .await)
}

/// The view of a pull request (ESC-457): its checks, reviews, comments and deployments, read by
/// whoever holds it in the project; or why the host did not give them.
#[tauri::command]
pub async fn pr_detail(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
) -> Res<crate::prview::Read<crate::prview::PrDetail>> {
    Ok(core
        .pr_detail(&project_id, number, host_repo.as_deref().unwrap_or(""))
        .await)
}

/// The end of the log of a check's job of a pull request of the project (its view).
#[tauri::command]
pub async fn pr_log(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    job: String,
    host_repo: Option<String>,
) -> Res<crate::prview::Read<String>> {
    Ok(core
        .pr_log(
            &project_id,
            number,
            host_repo.as_deref().unwrap_or(""),
            &job,
        )
        .await)
}

/// « Relancer les échecs » from the view of a pull request: how many runs started.
#[tauri::command]
pub async fn pr_rerun(
    core: CoreState<'_>,
    project_id: String,
    number: u64,
    host_repo: Option<String>,
) -> Res<usize> {
    core.pr_rerun(&project_id, number, host_repo.as_deref().unwrap_or(""))
        .await
        .map_err(err)
}

/// The end of the log of a check's job of the agent's pull request `number`.
#[tauri::command]
pub async fn agent_pr_log(
    core: CoreState<'_>,
    id: String,
    number: u64,
    job: String,
    host_repo: Option<String>,
) -> Res<crate::prview::Read<String>> {
    Ok(core
        .agent_pr_log(&id, number, host_repo.as_deref().unwrap_or(""), &job)
        .await)
}

/// « Relancer les échecs » on the agent's pull request `number`: how many runs started.
#[tauri::command]
pub async fn agent_pr_rerun(
    core: CoreState<'_>,
    id: String,
    number: u64,
    host_repo: Option<String>,
) -> Res<usize> {
    core.agent_pr_rerun(&id, number, host_repo.as_deref().unwrap_or(""))
        .await
        .map_err(err)
}

/// Why the agent has no « Ouvrir une PR… »; none when it can open one.
#[tauri::command]
pub async fn agent_no_pr(core: CoreState<'_>, id: String) -> Res<Option<crate::prview::NoPr>> {
    core.agent_no_pr(&id).await.map_err(err)
}

/// The open pull requests of the project, or why they could not be read.
#[tauri::command]
pub async fn project_prs(
    core: CoreState<'_>,
    project_id: String,
) -> Res<crate::prview::Read<crate::prview::ProjectPrs>> {
    Ok(core.project_prs(&project_id).await)
}

/// The background reading of the project on screen's list, for the PR tab's badge (ESC-403).
#[tauri::command]
pub async fn project_prs_watch(core: CoreState<'_>, project_id: String) -> Res<bool> {
    Ok(core.project_prs_watch(&project_id).await)
}

/// The open pull requests of every project, as recently read, for the overview (ESC-68).
#[tauri::command]
pub async fn overview_prs(core: CoreState<'_>) -> Res<Vec<crate::prview::OverviewPrs>> {
    Ok(core.overview_prs().await)
}

/// The environments of the project's host as last read (ESC-67): the view « Environnements »
/// opens on them, and its project is followed while it is open.
#[tauri::command]
pub fn project_environments(
    core: CoreState<'_>,
    project_id: String,
) -> Option<crate::deploys::ProjectEnvironments> {
    core.project_environments(&project_id)
}

/// « Rafraîchir » of the view « Environnements »: read now.
#[tauri::command]
pub async fn project_environments_read(
    core: CoreState<'_>,
    project_id: String,
) -> Res<crate::deploys::ProjectEnvironments> {
    core.project_environments(&project_id);
    Ok(core.project_environments_read(&project_id).await)
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

/// What is at `path` of the source (`""`: its root), for the VS Code view (ESC-447): its kind, size
/// and times, which VS Code compares to tell a file changed on disk since it was read.
#[tauri::command]
pub async fn fs_stat(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<fsedit::Stat> {
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    fsedit::stat(std::path::Path::new(&root), &path).map_err(|e| {
        match e.downcast_ref::<fsedit::NotFound>() {
            Some(missing) => missing.wire(),
            None => err(e),
        }
    })
}

/// The bytes of a file of the source as they are, text or not, for the VS Code view (ESC-447):
/// raw, no base64 through JSON.
#[tauri::command]
pub async fn fs_read_bytes(
    core: CoreState<'_>,
    project_id: String,
    agent_id: Option<String>,
    path: String,
) -> Res<tauri::ipc::Response> {
    let (root, _) = core.edit_root(&project_id, agent_id).await.map_err(err)?;
    let bytes =
        tokio::task::spawn_blocking(move || fsedit::read_bytes(std::path::Path::new(&root), &path))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| match e.downcast_ref::<fsedit::NotFound>() {
                Some(missing) => missing.wire(),
                None => err(e),
            })?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// One header of a raw call, decoded (`encodeURIComponent` on the window's side: a path may be
/// any text).
fn header(request: &crate::remoteapi::Body, name: &str) -> Option<String> {
    let raw = request.header(name)?;
    percent_decoded(raw)
}

/// `%XX` decoded, the result UTF-8 or nothing.
pub(crate) fn percent_decoded(raw: &str) -> Option<String> {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = raw.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Writes the bytes of a file of the source as they are, for the VS Code view (ESC-447):
/// `invoke('fs_write_bytes', bytes, { headers })` with `x-project`, `x-agent` (absent: the
/// project's checkout), `x-path`, and `x-expect`: `absent` (a new file, refused when something is
/// there), `any` (VS Code's « Overwrite »), or `<mtime>:<size>` (refused with `changed` when the
/// file is no longer the one VS Code read). Its new stamp.
#[tauri::command]
pub async fn fs_write_bytes(
    core: CoreState<'_>,
    request: crate::remoteapi::Body<'_>,
) -> Res<fsedit::Stamp> {
    let Some(bytes) = request.raw() else {
        return Err(tr!(
            "Les octets d’un fichier viennent bruts, pas en JSON.",
            "The bytes of a file come raw, not as JSON."
        ));
    };
    let (Some(project_id), Some(path)) =
        (header(&request, "x-project"), header(&request, "x-path"))
    else {
        return Err(tr!("Fichier à écrire non dit.", "No file to write said."));
    };
    let agent_id = header(&request, "x-agent").filter(|a| !a.is_empty());
    let expect = header(&request, "x-expect").unwrap_or_default();
    let stamp = match expect.as_str() {
        "absent" | "any" => None,
        s => {
            let parsed = s.split_once(':').and_then(|(m, z)| {
                Some(fsedit::Stamp {
                    mtime: m.parse().ok()?,
                    size: z.parse().ok()?,
                })
            });
            Some(parsed.ok_or_else(|| format!("x-expect: {s}"))?)
        }
    };
    let (root, worktrees) = core
        .edit_worktrees(&project_id, agent_id)
        .await
        .map_err(err)?;
    let bytes = bytes.to_vec();
    let stamp = tokio::task::spawn_blocking(move || {
        let check = match (expect.as_str(), stamp) {
            ("absent", _) => fsedit::Expect::Absent,
            (_, Some(s)) => fsedit::Expect::Stamp(s),
            _ => fsedit::Expect::Any,
        };
        fsedit::write_bytes(
            std::path::Path::new(&root),
            &path,
            &bytes,
            check,
            &worktrees,
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)?;
    core.git.refresh(&project_id);
    core.telemetry.track(Event::FileSaved);
    Ok(stamp)
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
    let roots = core.open_roots();
    tokio::task::spawn_blocking(move || fsedit::reveal(what, roots))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

/// The files an agent sent by a call of `SendUserFile` (ESC-171), as shown now: its images
/// read, its other files looked at.
#[tauri::command]
pub async fn sent_files(
    core: CoreState<'_>,
    agent_id: String,
    item_id: String,
) -> Res<Vec<crate::sentfiles::SentView>> {
    core.sent_files(&agent_id, &item_id).await.map_err(err)
}

/// The file `index` an agent sent opened with its application, or shown in its folder.
#[tauri::command]
pub async fn sent_file_open(
    core: CoreState<'_>,
    agent_id: String,
    item_id: String,
    index: usize,
    reveal: bool,
) -> Res<()> {
    core.open_sent_file(&agent_id, &item_id, index, reveal)
        .await
        .map_err(err)
}

/// The file `index` an agent sent, in raw bytes (ESC-538): what a client on a server downloads to its own computer
/// (S12), a program or a script too (kept there, never opened). Binary frames on the remote API, never base64.
#[tauri::command]
pub async fn sent_file_bytes(
    core: CoreState<'_>,
    agent_id: String,
    item_id: String,
    index: usize,
) -> Res<tauri::ipc::Response> {
    core.sent_file_bytes(&agent_id, &item_id, index)
        .await
        .map(tauri::ipc::Response::new)
        .map_err(err)
}

/// The image a message of the agent shows by a local path, in raw bytes (ESC-538), for a client on a server.
#[tauri::command]
pub async fn agent_local_bytes(
    core: CoreState<'_>,
    agent_id: String,
    src: String,
) -> Res<tauri::ipc::Response> {
    core.agent_local_bytes(&agent_id, &src)
        .await
        .map(tauri::ipc::Response::new)
        .map_err(err)
}

/// A notification of the core's host was clicked on the client that showed it (ESC-538): what its click does (the
/// agent shown) is done, its events to the clients. False when it is no longer known.
#[tauri::command(async)]
pub fn host_notification_click(core: CoreState, id: u64) -> bool {
    core.host.clicked(id)
}

/// An image of the web a message shows (https), downloaded by the core at the user's click: its
/// `data:` URL.
#[tauri::command]
pub async fn remote_image(core: CoreState<'_>, src: String) -> Res<String> {
    core.remote_image(&src).await.map_err(err)
}

/// The image a message of the agent shows in Markdown by a local path: its `data:` URL, or none.
#[tauri::command]
pub async fn agent_local_image(
    core: CoreState<'_>,
    agent_id: String,
    src: String,
) -> Res<Option<String>> {
    core.agent_local_image(&agent_id, &src).await.map_err(err)
}

/// That image opened with its application, or shown in its folder (`reveal`): the image viewer.
#[tauri::command]
pub async fn agent_local_open(
    core: CoreState<'_>,
    agent_id: String,
    src: String,
    reveal: bool,
) -> Res<()> {
    core.open_local_image(&agent_id, &src, reveal)
        .await
        .map_err(err)
}

/// « Enregistrer sous… » of the image viewer: the image it shows (a `data:` URL the core made),
/// written where the user picks in the system's dialog, which the core opens: the window never
/// gives a path. False when the user cancels.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn save_image(app: tauri::AppHandle, data_url: String, name: String) -> Res<bool> {
    use tauri_plugin_dialog::DialogExt;
    let (bytes, name) = crate::sentfiles::image_to_save(&data_url, &name).map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<bool> {
        let Some(path) = app
            .dialog()
            .file()
            .set_file_name(&name)
            .blocking_save_file()
        else {
            return Ok(false);
        };
        std::fs::write(path.into_path()?, bytes)?;
        Ok(true)
    })
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
    let (root, base) = core
        .edit_root(&project_id, agent_id.clone())
        .await
        .map_err(err)?;
    // Of several repositories (ESC-169): its repository's version.
    let (root, base, path) = core.edit_route(&project_id, agent_id.as_deref(), root, base, path);
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
    output: TermChannel,
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
            crate::termlog::feed(output.sink()),
            move |code| {
                if let Some(c) = hub_core.upgrade() {
                    c.test_run_ended(&id);
                    c.hub.emit(UiEvent::TerminalExit { id, code });
                }
            },
        )
        .map_err(err)?;
    core.track(Event::TerminalOpened);
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
    output: TermChannel,
) -> Res<TermInfo> {
    // Its launch off (ESC-658): its commands stay, none runs.
    core.part_check(&project_id, crate::parts::Part::Launch)?;
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
        core.track_test_run(a, &info.id, &info.name).map_err(err)?;
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
    output: TermChannel,
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
            crate::termlog::feed(output.sink()),
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
#[cfg(feature = "desktop")]
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

/// The agents the window holds a message for (`outbox.svelte.ts`).
#[tauri::command]
pub fn set_held(core: CoreState, agents: Vec<String>) {
    *core.held.lock() = agents.into_iter().collect();
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

/// « Réveiller » (ESC-544): the agent holds Remote Control with no process; it starts, and takes
/// its remote session back.
#[tauri::command]
pub async fn wake_remote(core: CoreState<'_>, id: String) -> Res<()> {
    core.wake_remote(&id).await.map_err(err)
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

/// « Partager avec l'équipe », « Arrêter le partage » (ESC-190).
#[tauri::command(async)]
pub fn ticket_share(core: CoreState, id: String, on: bool) -> Res<Ticket> {
    core.ticket_share(&id, on).map_err(err)
}

/// « Kanban de l'équipe » (TS-6): the project's Kanban made its team's, a new journal (`team_project` null) or a
/// teammate's (its id).
#[tauri::command]
pub async fn project_link_team_board(
    core: CoreState<'_>,
    project: String,
    team_project: Option<String>,
) -> Res<crate::model::Project> {
    core.link_team_board(&project, team_project)
        .await
        .map_err(err)
}

/// What « Verser à l'équipe » would pour (TS-7): the tickets of ESC-190's files and those of this machine.
#[tauri::command]
pub async fn project_team_pour_preview(
    core: CoreState<'_>,
    project: String,
) -> Res<crate::teampour::PourPreview> {
    core.team_pour_preview(&project).await.map_err(err)
}

/// « Verser à l'équipe » (TS-7): the tickets of the files (`files`) and those of this machine named, into the team's
/// Kanban, and the docs of `.escouade/` (`docs`) when the team has none on its server yet; the files frozen, named to
/// be removed by hand.
#[tauri::command]
pub async fn project_team_pour(
    core: CoreState<'_>,
    project: String,
    files: bool,
    tickets: Vec<String>,
    docs: Option<bool>,
) -> Res<crate::teampour::Poured> {
    core.team_pour(&project, files, tickets, docs.unwrap_or(false))
        .await
        .map_err(err)
}

/// « Ne plus proposer pour ce projet » on the band of the team a repository names (TS-7).
#[tauri::command(async)]
pub fn project_team_hint_dismiss(core: CoreState, project: String) -> Res<crate::model::Project> {
    core.team_hint_dismiss(&project).map_err(err)
}

/// « Reprendre » a ticket of the team whose holder's lease ran out (TS-6).
#[tauri::command(async)]
pub fn ticket_team_resume(core: CoreState, id: String) -> Res<Ticket> {
    core.team_resume(&id).map_err(err)
}

/// What the window of the passage of a ticket's agent shows (TS-17): its branch, its remote, its work, whom it can go to.
#[tauri::command]
pub async fn ticket_team_handoff_plan(
    core: CoreState<'_>,
    id: String,
) -> Res<crate::teamhandoff::HandoffPlan> {
    core.team_handoff_plan(&id).await.map_err(err)
}

/// « Passer » a ticket's agent to a teammate or to the team's machine (TS-17).
#[tauri::command]
pub async fn ticket_team_handoff(
    core: CoreState<'_>,
    id: String,
    to: String,
    wip: crate::teamhandoff::Wip,
) -> Res<()> {
    core.team_handoff(&id, &to, wip).await.map_err(err)
}

/// « Annuler le passage » of a ticket's agent not taken yet (TS-17).
#[tauri::command]
pub async fn ticket_team_handoff_cancel(core: CoreState<'_>, id: String) -> Res<()> {
    core.team_handoff_cancel(&id).await.map_err(err)
}

/// The harnesses this machine can take a passage on (TS-17).
#[tauri::command(async)]
pub fn ticket_team_handoff_engines(
    core: CoreState,
    id: String,
) -> Res<crate::teamhandoff::TakeChoice> {
    core.team_handoff_engines(&id).map_err(err)
}

/// « Prendre » the agent a teammate passed (TS-17), on `engine` when given.
#[tauri::command]
pub async fn ticket_team_handoff_take(
    core: CoreState<'_>,
    id: String,
    engine: Option<crate::model::EngineId>,
) -> Res<()> {
    core.team_handoff_take(&id, engine).await.map_err(err)
}

/// « Reprendre le mien » (`keepMine`) or « Prendre le sien » of a conflict of a ticket of the team (TS-6).
#[tauri::command(async)]
pub fn ticket_team_settle(
    core: CoreState,
    id: String,
    field: String,
    keep_mine: bool,
) -> Res<Ticket> {
    core.team_settle(&id, &field, keep_mine).map_err(err)
}

/// « Me l'assigner », « Ne plus me l'assigner » (TS-6).
#[tauri::command]
pub async fn ticket_team_assign(core: CoreState<'_>, id: String, me: bool) -> Res<Ticket> {
    core.team_assign(&id, me).await.map_err(err)
}

/// « Ne pas partager », « Partager avec l'équipe » of a ticket of a team's Kanban (TS-6).
#[tauri::command(async)]
pub fn ticket_team_local(core: CoreState, id: String, local: bool) -> Res<Ticket> {
    core.team_set_local(&id, local).map_err(err)
}

/// « Masquer chez moi », or back on the board (ESC-190).
#[tauri::command(async)]
pub fn ticket_hide(core: CoreState, id: String, hidden: bool) -> Res<Ticket> {
    core.ticket_hide(&id, hidden).map_err(err)
}

/// « Garder le mien », « Prendre le sien » of a shared ticket's file in conflict (ESC-190).
#[tauri::command(async)]
pub fn shared_resolve(core: CoreState, id: String, keep_mine: bool) -> Res<Ticket> {
    core.shared_resolve(&id, keep_mine).map_err(err)
}

/// `git config user.name`, offered by the first launch's window (ESC-190).
#[tauri::command]
pub async fn git_user_name(core: CoreState<'_>, project_id: Option<String>) -> Res<String> {
    Ok(core.git_user_name(project_id).await)
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

/// « Choisir la cible de ce ticket » (ESC-552): its own target, the board's when empty.
#[tauri::command]
pub async fn ticket_set_target(core: CoreState<'_>, id: String, target: String) -> Res<Ticket> {
    core.ticket_set_target(&id, &target).await.map_err(err)
}

/// « Rafraîchir » of the column « En revue » (ESC-59): its pull requests are read now.
#[tauri::command]
pub async fn ticket_prs_refresh(core: CoreState<'_>, project_id: String) -> Res<usize> {
    core.ticket_prs_refresh(Some(&project_id))
        .await
        .map_err(err)
}

/// The CI of each project's target branch as last read (ESC-63): what the headers start with.
#[tauri::command(async)]
pub fn target_cis(
    core: CoreState,
) -> std::collections::BTreeMap<String, crate::branchci::BranchCi> {
    core.target_cis()
}

/// « Rafraîchir » of a CI's pill (ESC-63): the project's target and the branches its tickets
/// pushed are read now.
#[tauri::command]
pub async fn ci_refresh(core: CoreState<'_>, project_id: String) -> Res<()> {
    core.branch_cis_refresh(Some(&project_id), true).await;
    Ok(())
}

/// The checks of a target, of a pushed branch, of a ticket's pull request or of an agent's branch.
#[tauri::command]
pub async fn ci_detail(
    core: CoreState<'_>,
    of: crate::branchci::CiOf,
) -> Res<crate::prview::Read<crate::branchci::CiDetail>> {
    Ok(core.ci_detail(&of).await)
}

/// The end of a job's log, in reading only.
#[tauri::command]
pub async fn ci_job_log(
    core: CoreState<'_>,
    of: crate::branchci::CiOf,
    job: String,
) -> Res<crate::prview::Read<String>> {
    Ok(core.ci_job_log(&of, &job).await)
}

/// « Relancer les jobs en échec », on the user's click: how many runs started.
#[tauri::command]
pub async fn ci_rerun(core: CoreState<'_>, of: crate::branchci::CiOf) -> Res<usize> {
    core.ci_rerun(&of).await.map_err(err)
}

/// « Envoyer à un agent »: a failed job and the end of its log go to an agent of the project.
#[tauri::command]
pub async fn ci_log_to_agent(
    core: CoreState<'_>,
    of: crate::branchci::CiOf,
    job: Option<String>,
    check: String,
    url: Option<String>,
    agent_id: String,
) -> Res<()> {
    core.ci_log_to_agent(&of, job.as_deref(), &check, url.as_deref(), &agent_id)
        .await
        .map_err(err)
}

/// « Merger » / « Merger quand c'est vert » on a ticket « En revue » (ESC-62).
#[tauri::command]
pub async fn ticket_pr_merge(
    core: CoreState<'_>,
    id: String,
    when_green: bool,
) -> Res<crate::ticketprs::MergeHow> {
    core.ticket_pr_merge(&id, when_green).await.map_err(err)
}

#[tauri::command(async)]
pub fn ticket_pr_stop_waiting(core: CoreState, id: String) -> Res<()> {
    core.ticket_pr_stop_waiting(&id).map_err(err)
}

/// « Terminer quand même »: a ticket whose pull request was closed without a merge is done.
#[tauri::command(async)]
pub fn ticket_finish_anyway(core: CoreState, id: String) -> Res<()> {
    core.ticket_finish_anyway(&id).map_err(err)
}

/// « Remettre à faire »: a ticket whose pull request was closed without a merge goes back to do.
#[tauri::command]
pub async fn ticket_pr_back_to_todo(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_pr_back_to_todo(&id).await.map_err(err)
}

/// « L'agent corrige » on a ticket « En revue » whose CI failed (ESC-60): its agent gets the
/// failed jobs and the end of their logs, its worktree made again from its branch.
#[tauri::command]
pub async fn ticket_pr_fix_ci(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_pr_fix_ci(&id).await.map_err(err)
}

/// « L'agent corrige les retours » on a ticket « En revue » (ESC-61): its agent gets the review's
/// comments not resolved that came since its last push of fixes. On a click only.
#[tauri::command]
pub async fn ticket_pr_fix_reviews(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_pr_fix_reviews(&id).await.map_err(err)
}

/// « L'agent met à jour la branche » on a ticket « En revue » in conflict with its target (ESC-61).
#[tauri::command]
pub async fn ticket_pr_update_branch(core: CoreState<'_>, id: String) -> Res<()> {
    core.ticket_pr_update_branch(&id).await.map_err(err)
}

#[tauri::command(async)]
pub fn board_set(core: CoreState, project_id: String, settings: BoardSettings) -> Res<Project> {
    core.board_set(&project_id, settings).map_err(err)
}

/// « En parallèle » of a project's Kanban (ESC-620): its bound, the ceiling imposed and where it comes from, and what
/// the machine holds; a server's for a server's space (served by the remote API).
#[tauri::command(async)]
pub fn board_capacity(
    core: CoreState,
    project_id: Option<String>,
) -> Res<crate::parallel::Capacity> {
    core.board_capacity(project_id.as_deref()).map_err(err)
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
    output: TermChannel,
) -> Res<TermInfo> {
    let spec = core.test_run_spec(&agent_id, &kind, index).map_err(err)?;
    let project_id = core
        .agent(&agent_id)
        .map_err(err)?
        .lock()
        .meta
        .project_id
        .clone();
    // Its project's launch off (ESC-658): nothing runs.
    core.part_check(&project_id, crate::parts::Part::Launch)?;
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
    core.track_test_run(&agent_id, &info.id, &info.name)
        .map_err(err)?;
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
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    core: CoreState<'_>,
    updates: State<'_, crate::updates::Updates>,
    manual: bool,
) -> Res<crate::updates::Checked> {
    if !crate::offline::allows(crate::offline::Flow::Updates) {
        // « Mode hors ligne » (ESC-596): nothing asked; said to the user who asked alone.
        return Ok(crate::updates::Checked::Failed {
            trouble: crate::updates::Trouble {
                message: crate::offline::refusal(crate::i18n::ui()),
                show: manual,
                page: None,
            },
        });
    }
    let settings = core.settings.read().clone();
    Ok(crate::updates::check(&app, &settings, &updates, manual).await)
}

/// « Mode hors ligne » (ESC-596): whether the four flows are stopped, and whether by a policy of this device.
#[tauri::command]
pub fn offline_state() -> crate::offline::State {
    crate::offline::read_again();
    crate::offline::state()
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
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn update_restart(app: tauri::AppHandle) -> Res<()> {
    crate::updates::restart_now(app).await.map_err(err)
}

/// « Plus tard »: the automatic restart planned is called off.
#[cfg(feature = "desktop")]
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
pub async fn http_ready(core: CoreState<'_>, url: String, agent_id: Option<String>) -> Res<bool> {
    // The probe of a launch of a project whose launch is off (ESC-658): none.
    if let Some(p) = agent_id
        .as_deref()
        .and_then(|a| core.agent(a).ok())
        .map(|h| h.lock().meta.project_id.clone())
    {
        if !core.part_on(&p, crate::parts::Part::Launch) {
            return Ok(false);
        }
    }
    let up = crate::testlaunch::http_ready(&url).await;
    // An agent's test launch answers: said on the facts' bus (PLG-5).
    if let (true, Some(a)) = (up, agent_id) {
        core.test_run_ready(&a, &url);
    }
    Ok(up)
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
    let r = core.forge_connect(form).await.map_err(err);
    // An account there: the CI of the targets and the pull requests read at once (ESC-63).
    if r.is_ok() {
        core.wake_ticket_prs();
        // A host known by its account only (a GitLab of its own) is a project's code host now (ESC-390).
        core.hub.emit(crate::model::UiEvent::CodeHosts);
    }
    r
}

#[tauri::command]
pub async fn forge_disconnect(
    core: CoreState<'_>,
    host: String,
) -> Res<crate::forge::connect::ForgesView> {
    let r = core.forge_disconnect(&host).await.map_err(err);
    core.hub.emit(crate::model::UiEvent::CodeHosts);
    r
}

/// The project's code host (ESC-390): none without a remote on a host whose plugin is on.
#[tauri::command]
pub async fn project_code_host(
    core: CoreState<'_>,
    project_id: String,
) -> Res<Option<crate::forge::connect::CodeHost>> {
    Ok(core.project_code_host(&project_id).await)
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
    // github.com's account is its code host's too: the CI of the targets read at once (ESC-63).
    if r.is_ok() && service == Service::GITHUB {
        core.wake_ticket_prs();
    }
    let service = telemetry::service(service);
    counted(&core, Event::IntegrationConnected { service }, r)
}

#[tauri::command]
pub fn integration_disconnect(
    core: CoreState,
    service: Service,
    account: Option<String>,
) -> Res<Vec<AccountView>> {
    core.integration_disconnect(service, account.as_deref().unwrap_or(""))
        .map_err(err)
}

#[tauri::command]
pub async fn integration_containers(
    core: CoreState<'_>,
    service: Service,
    account: Option<String>,
    project_id: Option<String>,
) -> Res<Vec<Container>> {
    core.integration_containers(
        service,
        account.as_deref().unwrap_or(""),
        project_id.as_deref(),
    )
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn integration_states(
    core: CoreState<'_>,
    service: Service,
    account: Option<String>,
    container: String,
) -> Res<StatesView> {
    core.integration_states(service, account.as_deref().unwrap_or(""), &container)
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
        core.track(Event::McpEnabled);
    }
    Ok(())
}

/// The first launch's journey (ESC-504) as the window shows it: open at a new installation's first start, or left half-way.
#[tauri::command(async)]
pub fn onboarding_view(core: CoreState) -> crate::onboarding::OnboardingView {
    core.onboarding_view()
}

/// A gesture on the journey: a station done or passed, « Tout passer », its end, « Refaire le premier lancement ».
#[tauri::command(async)]
pub fn onboarding_act(
    core: CoreState,
    act: crate::onboarding::Act,
) -> crate::onboarding::OnboardingView {
    core.onboarding_act(act)
}

/// What « Ce que tu veux brancher » offers (ESC-512): the sources of tickets, the voice services, the code hosts, read
/// in the registry, the seed and the installed plugins, with what a policy denies.
#[tauri::command(async)]
pub fn onboarding_offers(core: CoreState) -> Vec<crate::onboarding::Offer> {
    core.onboarding_offers()
}

/// A thing chosen on a card of « Ce que tu veux brancher » (ESC-512): its plugin from the seed, with no network, and
/// nothing else; its view, or why it is refused.
#[tauri::command(async)]
pub fn onboarding_need(
    core: CoreState,
    kind: crate::model::Hookup,
    id: String,
) -> Result<Option<crate::plugins::PluginView>, crate::plugins::Refusal> {
    core.onboarding_need(kind, &id)
}

/// The station of the first project (ESC-505) reads a folder: there, written to, a test command it can run.
#[tauri::command(async)]
pub fn onboarding_folder(path: String) -> crate::onboarding::FolderCheck {
    let node = crate::onboarding::on_path("node");
    crate::onboarding::check_folder(std::path::Path::new(path.trim()), node)
}

/// « Cloner » of the station of the first project: a repository by its address, into `parent/folder`.
#[tauri::command]
pub async fn onboarding_clone(
    url: String,
    parent: String,
    folder: String,
) -> crate::onboarding::Cloned {
    crate::onboarding::clone(&url, std::path::Path::new(parent.trim()), &folder).await
}

/// « Un projet de démonstration »: the demonstration written on this machine, without the network, and opened as a
/// project (the one there already, when it is).
#[tauri::command]
pub async fn demo_create(core: CoreState<'_>, color: String) -> Res<Project> {
    core.demo_create(&color).await.map_err(err)
}

/// « Supprimer » of the demonstration's band: its project, its agents and its folder gone; refused for any other project.
#[tauri::command]
pub async fn demo_remove(core: CoreState<'_>, id: String) -> Res<()> {
    core.demo_remove(&id).await.map_err(err)
}

/// The assistant of the features (F-2, ESC-559) as the window shows it: open by itself at the first window of the version
/// that brings the switches, what is used, the features a later version brought.
#[tauri::command(async)]
pub fn features_tour_view(core: CoreState) -> crate::features::TourView {
    core.features_tour_view()
}

/// A gesture on the assistant of the features: ended or passed, features met in the window of the novelties.
#[tauri::command(async)]
pub fn features_tour_act(
    core: CoreState,
    act: crate::features::TourAct,
) -> crate::features::TourView {
    core.features_tour_act(act)
}

/// The discovery of the features (ESC-506) as the window shows it. `microphones`: those the window found (`voice_info`),
/// None when it cannot tell.
#[tauri::command(async)]
pub fn discover_view(core: CoreState, microphones: Option<usize>) -> crate::discover::DiscoverView {
    core.discover_view(microphones)
}

/// A gesture on the discovery: seen by the window, « Écarter », « Remettre », « Tout masquer », a bubble shown, the
/// band « Nouveau dans la … » closed.
#[tauri::command(async)]
pub fn discover_act(
    core: CoreState,
    act: crate::discover::Act,
    microphones: Option<usize>,
) -> crate::discover::DiscoverView {
    core.discover_act(act, microphones)
}

/// The proposal of a feature switched off to show now, if any (F-3, ESC-558), for a client whose screen shows the agents
/// `visible` (never during their turn); the proposals set apart for good. A client of the remote API shows its core's.
#[tauri::command]
pub async fn feature_offer(
    core: CoreState<'_>,
    visible: Option<Vec<String>>,
) -> Res<crate::offers::OfferView> {
    Ok(core.feature_offer(&visible.unwrap_or_default()).await)
}

/// A gesture on a proposal: shown, « Activer », « Pas maintenant », « Ne plus proposer », « Reproposer ».
#[tauri::command(async)]
pub fn feature_offer_act(
    core: CoreState,
    act: crate::offers::OfferAct,
) -> Res<crate::offers::OfferView> {
    core.feature_offer_act(act).map_err(err)
}

/// The proposals as they are, without one to show: those set apart for good (Réglages › Fonctionnalités).
#[tauri::command(async)]
pub fn feature_offers(core: CoreState) -> crate::offers::OfferView {
    core.feature_offers()
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
    // The feature Voix off (F-1).
    core.feature_check(crate::features::Feature::Voice)?;
    // The dictation turned off by this device's policy (TS-12).
    core.locks
        .read()
        .check_dictation()
        .map_err(|e| e.to_string())?;
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
        core.track(Event::DictationUsed {
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
    let settings = core.settings.read().clone();
    crate::voice::online::usable_keys(&settings, &*core.secrets)
}

/// Réglages › Voix and the accounts of Mistral Vibe: the Mistral account's key by its end, or the Vibe account the voice
/// goes by (ESC-327).
#[tauri::command(async)]
pub fn voice_mistral_key(core: CoreState) -> crate::voice::online::MistralKey {
    crate::voice::online::mistral_state(&core.settings.read())
}

/// Keeps the key of `service` in the system's keychain, read back to check; never in a file: a
/// keychain that refuses it (or none, on Linux) leaves it unsaved, and says so.
#[tauri::command(async)]
pub fn voice_online_key_set(core: CoreState, service: String, key: String) -> Res<()> {
    // A key given to a service of the first hand: its plugin from the seed if it is not installed (ESC-373).
    core.need_voice_plugin(&service);
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
    let svc = crate::voice::service::find(&service)
        .ok_or_else(|| format!("unknown service {service}"))?;
    let p = crate::voice::online::Provider::of(&svc.id());
    let kept = voice.catalog.get(p);
    if !refresh && kept.is_some() {
        return Ok(svc.catalog(kept.as_ref(), None));
    }
    let settings = core.settings.read().clone();
    let spec = match crate::voice::online::keyed(p, &settings, &*core.secrets) {
        Ok(s) => s,
        // No key: the documentation's, said only when asked.
        Err(e) => {
            let error = refresh.then(|| voice_err(e));
            return Ok(svc.catalog(kept.as_ref(), error));
        }
    };
    match svc.list(&spec).await {
        Ok(listed) => {
            if let Err(e) = voice.catalog.put(p, &listed) {
                log::warn!("voice: the catalog of {} not kept: {e:#}", p.label());
            }
            Ok(svc.catalog(Some(&listed), None))
        }
        Err(e) => Ok(svc.catalog(kept.as_ref(), Some(e.message(i18n::kit(i18n::ui()))))),
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

/// The Overseer's screen starts talking (ESC-468): as `conversation_start`, but never while this device's policy turns
/// the dictation off (TS-12): its voice is cut too.
#[tauri::command(async)]
pub fn overseer_voice_start(
    core: CoreState,
    voice: VoiceState,
) -> Res<crate::voice::conversation::ConvStarted> {
    core.locks
        .read()
        .check_dictation()
        .map_err(|e| e.to_string())?;
    conversation_start(core, voice)
}

/// Starts a conversation with the settings' microphone, model and voice; or says what is to be downloaded first.
#[tauri::command(async)]
pub fn conversation_start(
    core: CoreState,
    voice: VoiceState,
) -> Res<crate::voice::conversation::ConvStarted> {
    // The feature Voix off (F-1).
    core.feature_check(crate::features::Feature::Voice)?;
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
        core.track(Event::VoiceConversation {
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

/// « Raccourci global » of Réglages › Overseer (ESC-468): registered, then kept; refused when another app holds it.
#[tauri::command(async)]
pub fn overseer_shortcut_set(core: CoreState, shortcut: String) -> Res<()> {
    core.overseer_shortcut_set(&shortcut).map_err(err)
}

/// The Overseer's shortcut pressed, in a sandbox only (the end-to-end tests): the system's keys are not reached there.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn overseer_hotkey_press(app: tauri::AppHandle) -> Res<()> {
    if crate::paths::sandbox_dir().is_none() {
        return Err("only in a sandbox".into());
    }
    crate::hotkey::pressed(&app);
    Ok(())
}

/// Ends the sentence being said at once (ESC-468, the Overseer's Space let go): sent, or dropped (Escape).
#[tauri::command]
pub fn conversation_end(voice: VoiceState, session: u64, send: bool) {
    voice.conversation_end(session, send);
}

#[tauri::command]
pub fn conversation_stop(voice: VoiceState) {
    voice.conversation_stop();
}

/// A sentence said in the conversation, sent to the agent with the instruction to end with a passage to be read.
#[tauri::command]
pub async fn conversation_send(core: CoreState<'_>, id: String, text: String) -> Res<()> {
    // The Overseer's screen talks with it (ESC-468): a message of the user « dit », kept for its instructions.
    if core.is_overseer(&id) {
        core.locks
            .read()
            .check_dictation()
            .map_err(|e| e.to_string())?;
        return core.overseer_said(&text).await.map(|_| ()).map_err(err);
    }
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
    let p =
        crate::voice::online::find(&service).ok_or_else(|| format!("unknown service {service}"))?;
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
        .map_err(|e| e.message(i18n::kit(i18n::ui())))
}

fn preview_text(lang: &str) -> &'static str {
    if lang == "fr" {
        "Bonjour ! C’est la voix qui te lira les réponses de tes agents dans Escouade."
    } else {
        "Hello! This is the voice that will read your agents’ answers to you in Escouade."
    }
}

// ---------- the docs (ESC-256) ----------

/// The spec's tree by subject, with what the project already has that could be imported when the
/// spec is empty, and where the docs live (`branch`: current, or the fixed branch).
#[tauri::command]
pub async fn docs_spec_tree(core: CoreState<'_>, id: String) -> Res<Value> {
    let project = core.project(&id).map_err(err)?;
    let space = core.docs_space(&id, None).await.map_err(err)?;
    let subjects = space.spec_tree();
    let pages: usize = subjects.iter().map(|s| s.pages.len()).sum();
    Ok(serde_json::json!({
        "branch": project.docs_branch.trim(),
        "pages": pages,
        "subjects": subjects,
        "existingSpec": if pages == 0 { space.detect() } else { Vec::new() },
        "comparing": crate::docscompare::comparing(&id),
        "planning": core.docs_planning(&id).await.ok(),
    }))
}

/// « Comparer au code » (ESC-263): `pages`, or the whole spec when None. Ends with them compared.
#[tauri::command]
pub async fn docs_compare(
    core: CoreState<'_>,
    id: String,
    pages: Option<Vec<String>>,
) -> Res<crate::docscompare::Compared> {
    core.inner().docs_compare(&id, pages).await.map_err(err)
}

/// The spec already in the project (`SPEC.md`, a `docs/` folder…), offered for import.
#[tauri::command]
pub async fn docs_detect_spec(core: CoreState<'_>, id: String) -> Res<Vec<crate::docs::Candidate>> {
    Ok(core.docs_space(&id, None).await.map_err(err)?.detect())
}

#[tauri::command]
pub async fn docs_spec_page(
    core: CoreState<'_>,
    id: String,
    path: String,
) -> Res<crate::docs::Page> {
    core.docs_space(&id, None)
        .await
        .and_then(|s| s.spec_read(&path))
        .map_err(err)
}

/// A page written from the window: the fields left out stay as they are.
#[tauri::command]
pub async fn docs_spec_write(
    core: CoreState<'_>,
    id: String,
    path: String,
    title: Option<String>,
    body: Option<String>,
    subject: Option<String>,
) -> Res<crate::docs::Page> {
    let space = core.docs_space(&id, None).await.map_err(err)?;
    let changes_body = body.is_some();
    let written = space
        .spec_write(&path, |p| {
            if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
                p.title = t.trim().to_string();
            }
            if let Some(s) = subject.filter(|s| !s.trim().is_empty()) {
                p.subject = s.trim().to_string();
            }
            if let Some(b) = body {
                p.body = b;
            }
        })
        .map(|(page, _, _)| page)
        .map_err(err);
    // Its text written by hand is to plan (« Planifier », ESC-262); a page only made is not yet.
    if let (Ok(page), true) = (&written, changes_body) {
        if !page.body.trim().is_empty() {
            core.docs_to_plan(&id, std::slice::from_ref(&page.path), &[]);
        }
    }
    core.docs_changed(&id).await;
    written
}

#[tauri::command]
pub async fn docs_spec_search(
    core: CoreState<'_>,
    id: String,
    query: String,
) -> Res<Vec<crate::docs::Hit>> {
    Ok(core
        .docs_space(&id, None)
        .await
        .map_err(err)?
        .spec_search(&query, 50))
}

#[tauri::command]
pub async fn docs_decisions(core: CoreState<'_>, id: String) -> Res<Vec<crate::docs::Decision>> {
    Ok(core.docs_space(&id, None).await.map_err(err)?.decisions())
}

/// A decision proposed from the window, or a proposal still open revised (`reference`: its id):
/// the fields the call leaves out keep their value, the ones it gives replace it, empty included.
#[tauri::command]
pub async fn docs_decision_propose(
    core: CoreState<'_>,
    id: String,
    reference: Option<crate::docs::Ref>,
    decision: crate::docs::DecisionPatch,
) -> Res<crate::docs::Decision> {
    let out = core
        .docs_space(&id, None)
        .await
        .and_then(|s| s.decision_revise(reference.as_ref(), decision))
        .map_err(err);
    if let Ok(d) = &out {
        core.docs_to_plan(&id, &[], &[d.reference()]);
    }
    core.docs_changed(&id).await;
    out
}

/// A proposal that replaces the accepted decision `number`.
#[tauri::command]
pub async fn docs_decision_replace(
    core: CoreState<'_>,
    id: String,
    number: u32,
    decision: crate::docs::Decision,
) -> Res<crate::docs::Decision> {
    let out = core
        .docs_space(&id, None)
        .await
        .and_then(|s| s.decision_replace(number, decision))
        .map_err(err);
    if let Ok(d) = &out {
        core.docs_to_plan(&id, &[], &[d.reference()]);
    }
    core.docs_changed(&id).await;
    out
}

/// « Accepter »: a human accepts a proposed decision (`reference`: its id). The only way to: no
/// MCP tool does.
#[tauri::command]
pub async fn docs_decision_accept(
    core: CoreState<'_>,
    id: String,
    reference: crate::docs::Ref,
) -> Res<crate::docs::Decision> {
    let out = core
        .docs_accept_decision(&id, &reference)
        .await
        .map_err(err);
    core.docs_changed(&id).await;
    out
}

#[tauri::command]
pub async fn docs_documents(
    core: CoreState<'_>,
    id: String,
    folder: Option<String>,
) -> Res<Vec<crate::docs::DocInfo>> {
    core.docs_space(&id, None)
        .await
        .and_then(|s| s.documents(folder.as_deref().unwrap_or_default()))
        .map_err(err)
}

#[tauri::command]
pub async fn docs_document_text(
    core: CoreState<'_>,
    id: String,
    path: String,
    from: Option<usize>,
) -> Res<crate::docs::DocText> {
    core.docs_space(&id, None)
        .await
        .and_then(|s| s.document_text(&path, from.unwrap_or(0), crate::docs::TEXT_CHUNK))
        .map_err(err)
}

/// A file dropped or chosen (`source`, a file of this machine), put in the drive: in `folder` when
/// one is given, else filed by itself (`docsdrive::place`). Over 50 MB without an LFS rule that
/// takes it, it asks to be `confirmed`.
#[tauri::command]
pub async fn docs_document_add(
    core: CoreState<'_>,
    id: String,
    folder: Option<String>,
    source: String,
    confirmed: Option<bool>,
) -> Res<crate::docsdrive::Dropped> {
    core.docs_drop(
        &id,
        folder.as_deref(),
        std::path::Path::new(&source),
        confirmed.unwrap_or(false),
    )
    .await
    .map_err(err)
}

/// The drive: its folders, its files with who added them, how many times the agents read them and
/// how many pages and decisions cite them.
#[tauri::command]
pub async fn docs_drive(core: CoreState<'_>, id: String) -> Res<crate::docsdrive::Drive> {
    core.docs_drive(&id).await.map_err(err)
}

#[derive(serde::Deserialize)]
pub struct NamedSize {
    name: String,
    size: u64,
}

/// What the window must know of files before it drops them: LFS takes them, or they are large.
#[tauri::command]
pub async fn docs_document_check(
    core: CoreState<'_>,
    id: String,
    folder: Option<String>,
    files: Vec<NamedSize>,
) -> Res<Vec<crate::docsdrive::Check>> {
    core.docs_check(
        &id,
        folder.as_deref().unwrap_or_default(),
        files.into_iter().map(|f| (f.name, f.size)).collect(),
    )
    .await
    .map_err(err)
}

/// A document moved to another folder (the root when empty).
#[tauri::command]
pub async fn docs_document_move(
    core: CoreState<'_>,
    id: String,
    path: String,
    folder: String,
) -> Res<crate::docs::DocInfo> {
    core.docs_move(&id, &path, &folder).await.map_err(err)
}

/// « Nouveau dossier ».
#[tauri::command]
pub async fn docs_folder_add(core: CoreState<'_>, id: String, folder: String) -> Res<String> {
    core.docs_folder_add(&id, &folder).await.map_err(err)
}

/// « Rechercher dans les documents ».
#[tauri::command]
pub async fn docs_documents_search(
    core: CoreState<'_>,
    id: String,
    query: String,
) -> Res<Vec<crate::docsdrive::DocHit>> {
    let space = core.docs_space(&id, None).await.map_err(err)?;
    core.docs_search_in(&id, space, &query, 30)
        .await
        .map(|(hits, _)| hits)
        .map_err(err)
}

/// What the project's docs have to share: the files, their number, the conflict being handled.
#[tauri::command]
pub async fn docs_pending(core: CoreState<'_>, id: String) -> Res<crate::docsshare::Pending> {
    core.docs_pending(&id).await.map_err(err)
}

/// « Partager »: fetches, commits the changes of the docs and only them, pushes. Stops on a
/// conflict, which is kept for `docs_share_resolve` / `docs_share_cancel`.
#[tauri::command]
pub async fn docs_share(
    core: CoreState<'_>,
    id: String,
    message: Option<String>,
    with_code: Option<bool>,
) -> Res<crate::docsshare::Shared> {
    core.docs_share(&id, message.as_deref(), with_code.unwrap_or(false))
        .await
        .map_err(err)
}

/// The synchronisation with none of the docs left to share (ESC-534): fetches and brings in what
/// the others pushed on the docs branch, without committing nor pushing. Stops on a conflict as
/// `docs_share` does.
#[tauri::command]
pub async fn docs_pull(core: CoreState<'_>, id: String) -> Res<crate::docsshare::Shared> {
    core.docs_pull(&id).await.map_err(err)
}

/// « L'agent résout » a conflict of the docs.
#[tauri::command]
pub async fn docs_share_resolve(
    core: CoreState<'_>,
    id: String,
) -> Res<crate::docsshare::Conflict> {
    core.docs_share_resolve(&id).await.map_err(err)
}

/// « Annuler » a conflict of the docs.
#[tauri::command]
pub async fn docs_share_cancel(core: CoreState<'_>, id: String) -> Res<()> {
    core.docs_share_cancel(&id).await.map_err(err)
}

/// How the spec found at `path` (one of `docs_detect_spec`'s) would be cut into pages: nothing written.
#[tauri::command]
pub async fn docs_import_plan(
    core: CoreState<'_>,
    id: String,
    path: String,
) -> Res<Vec<crate::docsimport::Draft>> {
    core.docs_space(&id, None)
        .await
        .and_then(|s| s.import_plan(&path))
        .map_err(err)
}

/// « Importer », once the user agreed to the cut: the pages of `only` (all when left out), none overwritten.
#[tauri::command]
pub async fn docs_import(
    core: CoreState<'_>,
    id: String,
    path: String,
    only: Option<Vec<String>>,
) -> Res<crate::docsimport::Imported> {
    let out = core
        .docs_space(&id, None)
        .await
        .and_then(|s| s.import(&path, only.as_deref()))
        .map_err(err);
    core.docs_changed(&id).await;
    out
}

/// « Ce qui a bougé »: the last committed changes of the spec.
#[tauri::command]
pub async fn docs_recent(core: CoreState<'_>, id: String) -> Res<Vec<crate::docsshare::Moved>> {
    core.docs_recent(&id).await.map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_not_opened_is_marked_for_the_window_to_offer_its_folder() {
        let e = crate::sentfiles::not_run(Lang::Fr, "a.msc", crate::sentfiles::Why::Program);
        let text = err(e);
        assert!(
            text.starts_with("not-opened|« a.msc » est un programme"),
            "{text}"
        );
        assert_eq!(err(anyhow::anyhow!("autre")), "autre");
    }

    #[test]
    fn the_refusals_of_a_terminal_read_in_english() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(shell_not_found(En, "zsh"), "shell “zsh” not found");
        assert_eq!(shell_not_found(Fr, "zsh"), "shell « zsh » introuvable");
    }

    #[test]
    fn the_commands_of_the_background_tasks_are_in_the_handler_the_window_calls() {
        // A command that is written and not listed there is never reachable from the window: the single list of
        // the commands (`remoteapi::list`, ESC-478) makes the IPC's handler.
        for name in ["stop_bg_task", "task_output"] {
            assert_eq!(
                crate::remoteapi::list::IPC
                    .iter()
                    .filter(|n| **n == name)
                    .count(),
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

/// The project's shared memory (ESC-255), as « Mémoire partagée » of its settings lists it.
#[tauri::command(async)]
pub fn memory_list(core: CoreState, project_id: String) -> Res<Vec<crate::memory::Memory>> {
    core.memories(&project_id).map_err(err)
}

/// « Oublier » of a memory, in the project's settings.
#[tauri::command]
pub async fn memory_forget(core: CoreState<'_>, project_id: String, name: String) -> Res<()> {
    core.forget_memory(&project_id, &name).await.map_err(err)
}

/// What Claude Code remembers of the project, offered once: none once offered.
#[tauri::command(async)]
pub fn claude_memory_offer(
    core: CoreState,
    project_id: String,
) -> Res<Vec<crate::memory::Offered>> {
    core.claude_memory_offer(&project_id).map_err(err)
}

/// « Reprendre » the memories ticked (none: « Pas maintenant »); the offer is not made again.
#[tauri::command]
pub async fn claude_memory_take(
    core: CoreState<'_>,
    project_id: String,
    names: Vec<String>,
) -> Res<Project> {
    core.claude_memory_take(&project_id, &names)
        .await
        .map_err(err)
}

/// The shared memory of a project of a team on its server (TS-10), as Réglages › Projets › Équipe › Mémoire partagée shows it:
/// what this member's agents read with its provenance, the proposals to keep or set aside, what was withdrawn, the
/// quarantines.
#[tauri::command]
pub async fn team_memory_view(
    core: CoreState<'_>,
    project_id: String,
) -> Res<crate::teammemory::TeamMemoryView> {
    core.team_memory_view(&project_id).await.map_err(err)
}

/// « Garder » (`keep`) or « Écarter » a proposed memory of the team: its author or an administrator, in the window
/// alone.
#[tauri::command]
pub async fn team_memory_decide(
    core: CoreState<'_>,
    project_id: String,
    id: String,
    keep: bool,
) -> Res<crate::teammemory::TeamMemoryView> {
    core.team_memory_decide(&project_id, &id, keep)
        .await
        .map_err(err)
}

/// A quarantine of the team's memory (an administrator's): its preview (`apply` false), or the rule written.
#[tauri::command]
pub async fn team_memory_quarantine(
    core: CoreState<'_>,
    project_id: String,
    rule: crate::teammemory::Rule,
    apply: bool,
) -> Res<crate::teammemory::QuarantinePreview> {
    core.team_memory_quarantine(&project_id, rule, apply)
        .await
        .map_err(err)
}

/// « Lever la quarantaine » (an administrator's).
#[tauri::command]
pub async fn team_memory_lift(
    core: CoreState<'_>,
    project_id: String,
    rule_id: String,
) -> Res<crate::teammemory::TeamMemoryView> {
    core.team_memory_lift(&project_id, &rule_id)
        .await
        .map_err(err)
}

/// The memories a ticket's agent proposed (its key), to keep or set aside at its validation.
#[tauri::command]
pub async fn team_memory_of_ticket(
    core: CoreState<'_>,
    project_id: String,
    key: String,
) -> Res<Vec<crate::teammemory::Entry>> {
    core.team_memory_of_ticket(&project_id, &key)
        .await
        .map_err(err)
}

/// « Valider les souvenirs des agents » (C14-B), unless the team imposes it.
#[tauri::command(async)]
pub fn team_memory_set_review(core: CoreState, project_id: String, on: bool) -> Res<Project> {
    core.team_memory_set_review(&project_id, on).map_err(err)
}

/// « Commencer » of « Nouvelle discussion » (ESC-261): a discussion or a brainstorm of the Docs
/// tab, its agent in the folder of the docs branch, sent its subject.
#[tauri::command]
pub async fn discussion_start(
    core: CoreState<'_>,
    id: String,
    start: crate::discussion::Start,
) -> Res<AgentView> {
    let agent = core.discussion_start(&id, start).await.map_err(err)?;
    core.telemetry
        .track(Event::AgentCreated { worktree: false });
    Ok(agent)
}

/// « Verser dans la spec »: what the discussion `agent` proposes, written on the docs branch.
#[tauri::command]
pub async fn discussion_pour(core: CoreState<'_>, agent: String) -> Res<crate::discussion::Poured> {
    core.discussion_pour(&agent).await.map_err(err)
}

/// « Planifier » (ESC-262): the changes of the discussion `agent` poured, or every change of the
/// spec without tickets when None, made into tickets « À faire » by the planner.
#[tauri::command]
pub async fn docs_plan(
    core: CoreState<'_>,
    id: String,
    agent: Option<String>,
) -> Res<crate::docsplan::Plan> {
    core.inner()
        .docs_plan(&id, agent.as_deref())
        .await
        .map_err(err)
}

/// « Annuler » of a planning: its tickets not started yet removed, its changes to plan again.
#[tauri::command]
pub async fn docs_plan_cancel(
    core: CoreState<'_>,
    id: String,
    plan: String,
) -> Res<crate::docsplan::Cancelled> {
    core.inner().docs_plan_cancel(&id, &plan).await.map_err(err)
}

/// Takes a proposal out of « Ce qui ira dans la spec »: a page, or a decision (from 1).
#[tauri::command]
pub fn discussion_drop(
    core: CoreState,
    agent: String,
    page: Option<String>,
    decision: Option<usize>,
) -> Res<()> {
    core.discussion_drop(&agent, page.as_deref(), decision)
        .map_err(err)
}

/// The links `escouade://` not yet taken (STORE-11): what they ask, for the window, once.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn deep_links_take(inbox: State<'_, crate::deeplink::Inbox>) -> Vec<crate::deeplink::DeepLink> {
    inbox.take()
}

// ---------- The team server (TS-3, `teams`) ----------

type TeamsState<'a> = State<'a, Arc<crate::teams::Teams>>;

/// The teams of this data folder, as the window shows them.
#[tauri::command]
pub async fn team_list(teams: TeamsState<'_>) -> Res<Vec<crate::teams::TeamView>> {
    Ok(teams.list().await)
}

/// Creates a team: the server's address, the token of `escouade-team-server init`, a name, and the organisation's
/// escrow key if any (the team's half of it, `escouade-escrow-team:2:`, and who holds it). Gives the 24 words of the
/// recovery key, shown once.
#[tauri::command]
pub async fn team_create(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    server: String,
    token: String,
    name: String,
    escrow_public: Option<String>,
    escrow_holder: Option<String>,
) -> Res<crate::teams::Created> {
    let escrow = match (escrow_public, escrow_holder) {
        (Some(p), Some(h)) if !p.trim().is_empty() => Some((p, h)),
        (Some(p), None) if !p.trim().is_empty() => {
            return Err("Dis qui détient la clé de séquestre.".into())
        }
        _ => None,
    };
    let created = teams.create(&server, &token, &name, escrow).await?;
    // A team of one's own: the discovery « Ton équipe » is done (ESC-506).
    core.discover_done(crate::model::Discovery::Team);
    Ok(created)
}

/// A new escrow key pair for the organisation: the team's half (its public key and the anchor of its wraps), and its
/// secret to give to whoever holds it.
#[tauri::command]
pub fn team_escrow_new(teams: TeamsState<'_>, holder: String) -> Res<crate::teams::EscrowPair> {
    teams.escrow_new(&holder)
}

/// Three of the 24 words typed back (their places from 0): true when they are right.
#[tauri::command]
pub fn team_confirm_words(
    teams: TeamsState<'_>,
    team: String,
    typed: Vec<(usize, String)>,
) -> Res<bool> {
    teams.confirm_words(&team, &typed)
}

/// A new recovery key for the team (an administrator's): its 24 words, shown once.
#[tauri::command]
pub async fn team_new_recovery(teams: TeamsState<'_>, team: String) -> Res<Vec<String>> {
    teams.new_recovery(&team).await
}

/// Invites a new member: `member` or `admin`, for `hours` (72 by default), one use.
#[tauri::command]
pub async fn team_invite(
    teams: TeamsState<'_>,
    team: String,
    role: String,
    hours: Option<u64>,
) -> Res<crate::teams::InviteView> {
    teams.invite(&team, &role, hours).await
}

/// The QR code of another device of one's own (an hour).
#[tauri::command]
pub async fn team_own_device(teams: TeamsState<'_>, team: String) -> Res<crate::teams::InviteView> {
    teams.own_device(&team).await
}

/// What an invitation link says, before joining: its server, the team's six words.
#[tauri::command]
pub fn team_join_preview(teams: TeamsState<'_>, link: String) -> Res<crate::teams::JoinPreview> {
    teams.join_preview(&link)
}

#[tauri::command]
pub async fn team_join(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    link: String,
    name: String,
) -> Res<crate::teams::TeamView> {
    let team = teams.join(&link, &name).await?;
    core.discover_done(crate::model::Discovery::Team);
    Ok(team)
}

/// Shows a short code for another device of one's own.
#[tauri::command]
pub async fn team_pair_start(teams: TeamsState<'_>, team: String) -> Res<crate::teams::PairView> {
    teams.pair_start(&team).await
}

#[tauri::command]
pub async fn team_pair_step(teams: TeamsState<'_>, team: String) -> Res<crate::teams::PairView> {
    teams.pair_step(&team).await
}

#[tauri::command]
pub fn team_pair_cancel(teams: TeamsState<'_>, team: String) {
    teams.pair_cancel(&team)
}

/// The new device, by the short code shown on the other.
#[tauri::command]
pub async fn team_pair_join(
    teams: TeamsState<'_>,
    server: String,
    code: String,
    name: String,
) -> Res<crate::teams::TeamView> {
    teams.pair_join(&server, &code, &name).await
}

/// Brings a team back by its recovery key: the server's address, the operator's token, the 24 words and the six words
/// of the team (both from the rescue sheet).
#[tauri::command]
pub async fn team_recover(
    teams: TeamsState<'_>,
    server: String,
    token: String,
    words: String,
    team_words: String,
    name: String,
) -> Res<crate::teams::Created> {
    teams
        .recover(&server, &token, &words, &team_words, &name)
        .await
}

/// Reads the team's stream now (« Réessayer » under an alarm).
#[tauri::command]
pub async fn team_sync(teams: TeamsState<'_>, team: String) -> Res<crate::teams::TeamView> {
    teams.sync(&team).await
}

/// Whether the team's server answers, and how many changes of this device wait to go (TS-5, §2.5): « Hors ligne · 3
/// changements en attente ». Shown by TS-4 and TS-6.
#[tauri::command]
pub async fn team_sync_status(
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::teams::SyncStatusView> {
    teams.status(&team).await
}

/// What the window shows of a project's docs on the team server (TS-9): the team's choice, the conflicts, the
/// repository's files left to remove, what waits.
#[tauri::command]
pub async fn team_docs_status(
    core: CoreState<'_>,
    id: String,
) -> Res<crate::teamdocs::TeamDocsView> {
    core.team_docs_view(&id).await.map_err(err)
}

/// « Les docs sur le serveur d'équipe » (`server`) or « Les docs restent dans le dépôt » (TS-9, C4-B); `journal`: a
/// teammate's journal to follow, for a project that has none yet.
#[tauri::command]
pub async fn team_docs_choose(
    core: CoreState<'_>,
    id: String,
    server: bool,
    journal: Option<String>,
) -> Res<crate::teamdocs::TeamDocsView> {
    let out = core
        .team_docs_choose(&id, server, journal)
        .await
        .map_err(err);
    core.docs_changed(&id).await;
    out
}

/// A page's conflict on the team server settled: `mine`, `theirs` or `agent` (TS-9).
#[tauri::command]
pub async fn team_docs_settle(
    core: CoreState<'_>,
    id: String,
    path: String,
    choice: String,
) -> Res<crate::teamdocs::TeamDocsView> {
    core.team_docs_settle(&id, &path, &choice)
        .await
        .map_err(err)
}

/// « Compris » on a decision's field that lost to a teammate's (TS-9).
#[tauri::command]
pub async fn team_docs_dismiss(
    core: CoreState<'_>,
    id: String,
    stamp: String,
) -> Res<crate::teamdocs::TeamDocsView> {
    core.team_docs_dismiss(&id, &stamp).await.map_err(err)
}

/// « Retirer du dépôt » (TS-9): the repository's docs imported to the server, removed from its folder; the user
/// commits.
#[tauri::command]
pub async fn team_docs_remove_repo_files(
    core: CoreState<'_>,
    id: String,
) -> Res<crate::teamdocs::TeamDocsView> {
    core.team_docs_remove_repo_files(&id).await.map_err(err)
}

/// Removes a project's snapshot found unreadable from the team's server (an administrator's, SR2-01).
#[tauri::command]
pub async fn team_remove_snapshot(
    teams: TeamsState<'_>,
    team: String,
    project: String,
    upto: u64,
) -> Res<crate::teams::TeamView> {
    teams.remove_snapshot(&team, &project, upto).await
}

/// Hides the alert of a project's snapshot put aside on this device (SR2-01).
#[tauri::command]
pub async fn team_dismiss_snapshot(
    teams: TeamsState<'_>,
    team: String,
    project: String,
    upto: u64,
) -> Res<crate::teams::TeamView> {
    teams.dismiss_snapshot(&team, &project, upto).await
}

/// Forgets the team on this device.
#[tauri::command]
pub async fn team_forget(teams: TeamsState<'_>, team: String) -> Res<()> {
    teams.forget(&team).await
}

// ---------- The administrator's circuit breaker (TS-15) ----------

/// A team's breakers with what its revocation list withdraws (TS-14), which the circuit breaker reads.
fn with_withdrawn(
    core: &CoreState<'_>,
    view: Res<crate::teams::BreakersView>,
) -> Res<crate::teams::BreakersView> {
    let mut view = view?;
    view.withdrawn = core
        .team_withdrawn(&view.team)
        .into_iter()
        .map(|(id, reason)| crate::teams::WithdrawnView { id, reason })
        .collect();
    Ok(view)
}

/// A team's circuit breakers as this device read them: those in force, those lifted, what was refused, and what the
/// team's revocation list withdraws.
#[tauri::command]
pub async fn team_breakers(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::teams::BreakersView> {
    with_withdrawn(&core, teams.breakers(&team).await)
}

/// The other devices of the team online now: what the confirmation of a breaker says will stop.
#[tauri::command]
pub async fn team_breaker_online(teams: TeamsState<'_>, team: String) -> Res<usize> {
    teams.breaker_online(&team).await
}

/// An administrator sets a circuit breaker (its scope, what it takes out, its reason), signed by this device. No tool of
/// the MCP server does it.
#[tauri::command]
pub async fn team_breaker_set(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    ask: crate::teams::BreakerAsk,
) -> Res<crate::teams::BreakersView> {
    with_withdrawn(&core, teams.breaker_set(&team, ask).await)
}

/// An administrator lifts a circuit breaker in force. No tool of the MCP server does it.
#[tauri::command]
pub async fn team_breaker_lift(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    breaker: String,
    by: String,
) -> Res<crate::teams::BreakersView> {
    with_withdrawn(&core, teams.breaker_lift(&team, &breaker, &by).await)
}

/// « Réessayer » after an alarm of a team's breakers.
#[tauri::command]
pub async fn team_breakers_retry(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::teams::BreakersView> {
    with_withdrawn(&core, teams.breakers_retry(&team).await)
}

// ---------- The costs and the quotas of the team (TS-16) ----------

/// The table of a team's costs: every device's totals on an administrator's device, this device's own on a member's,
/// with the rules of the team and its budgets weighed.
#[tauri::command]
pub async fn team_costs(core: CoreState<'_>, team: String) -> Res<crate::teamcosts::TeamCostsView> {
    core.team_costs_view(&team).await.map_err(err)
}

/// The member saw that the team sees its totals by day, « par membre » (C15): they carry its name from now on.
#[tauri::command]
pub fn team_costs_ack(core: CoreState<'_>, team: String) {
    core.team_costs_ack(&team);
}

/// The teams « par membre » whose screen this device's member has not seen yet: `[id, name]`.
#[tauri::command]
pub fn team_costs_untold(core: CoreState<'_>) -> Vec<(String, String)> {
    core.team_costs_untold()
}

/// What a start in a project is told of the budgets of its team (80 % or more of one), none otherwise.
#[tauri::command]
pub fn team_cost_notice(
    core: CoreState<'_>,
    project: String,
) -> Option<crate::teamcosts::CostNotice> {
    core.team_cost_notice(&project)
}

/// Sends this device's totals of every team now, then weighs the budgets (« Envoyer maintenant »).
#[tauri::command]
pub async fn team_costs_send(core: CoreState<'_>) -> Res<()> {
    core.inner().clone().team_costs_pass(true).await;
    Ok(())
}

// ---------- The team's signed .escouade.yml (TS-13) ----------

/// The team's settings as this device read them: the version in force, the history, what waits for this device.
#[tauri::command]
pub async fn team_settings(
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::teams::TeamSettingsView> {
    teams.settings(&team).await
}

/// A version written in the window, read before it is signed: its file, what refuses it, the signatures it needs.
#[tauri::command]
pub fn team_settings_check(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    yaml: String,
) -> crate::teams::SettingsDraft {
    let mut draft = teams.settings_check(&team, &yaml);
    // A plugin's setting (ESC-532), judged against the manifests of this device's plugins.
    if let Some(file) = &draft.file {
        draft.warnings.extend(core.team_plugin_warnings(file));
    }
    draft
}

/// An administrator signs a version, chained to the one in force.
#[tauri::command]
pub async fn team_settings_propose(
    teams: TeamsState<'_>,
    team: String,
    yaml: String,
    note: String,
    by: String,
) -> Res<crate::teams::TeamSettingsView> {
    teams.settings_propose(&team, &yaml, &note, &by).await
}

/// An administrator cosigns another's version (the quorum's second signature).
#[tauri::command]
pub async fn team_settings_cosign(
    teams: TeamsState<'_>,
    team: String,
    digest: String,
) -> Res<crate::teams::TeamSettingsView> {
    teams.settings_cosign(&team, &digest).await
}

/// This device accepts a sensitive key or a command proposed, at the fingerprint the member read.
#[tauri::command]
pub async fn team_settings_consent(
    teams: TeamsState<'_>,
    team: String,
    part: String,
    key: String,
    fingerprint: String,
) -> Res<crate::teams::TeamSettingsView> {
    teams
        .settings_consent(&team, &part, &key, &fingerprint)
        .await
}

/// This device takes its consent back.
#[tauri::command]
pub async fn team_settings_withdraw(
    teams: TeamsState<'_>,
    team: String,
    part: String,
    key: String,
) -> Res<crate::teams::TeamSettingsView> {
    teams.settings_withdraw(&team, &part, &key).await
}

/// « Voir » on the band: the member saw the version `number`.
#[tauri::command]
pub fn team_settings_seen(teams: TeamsState<'_>, team: String, number: u64) {
    teams.settings_seen(&team, number)
}

/// « Réessayer » after an alarm of the settings.
#[tauri::command]
pub async fn team_settings_retry(
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::teams::TeamSettingsView> {
    teams.settings_retry(&team).await
}

/// « Projet de l'équipe », « Rattacher à l'équipe »: what the team imposes and recommends holds for this project (or no
/// longer); attached, it joins the Kanban the team's list names and the key `team` of its `.escouade.yml` is written
/// (TS-7).
#[tauri::command]
pub async fn project_set_team(
    core: CoreState<'_>,
    project: String,
    team: Option<String>,
) -> Res<crate::model::Project> {
    match team.filter(|t| !t.trim().is_empty()) {
        Some(team) => core.team_attach(&project, &team).await,
        None => core.team_detach(&project),
    }
    .map_err(|e| format!("{e:#}"))
}

// ---------- The team's projects (TS-8) ----------

async fn team_projects_of(
    core: &Core,
    team: &str,
    of: crate::teams::CatalogOf,
) -> crate::team_projects::TeamProjectsView {
    let projects = core.projects.read().clone();
    let locals = crate::team_projects::locals(projects).await;
    crate::team_projects::combine(of, team, &locals)
}

/// The team's private store (TS-14), read from the server: its key, its namespace, its plugins, the proposals.
#[tauri::command]
pub async fn team_plugins(
    core: CoreState<'_>,
    team: String,
) -> Res<crate::teamplugins::TeamPluginsView> {
    core.team_plugins(&team).await
}

/// The stores of every team of this device, as last read: what the page Plugins shows of them.
#[tauri::command]
pub async fn team_plugins_all(
    core: CoreState<'_>,
) -> Res<Vec<crate::teamplugins::TeamPluginsView>> {
    Ok(core.team_plugins_all().await)
}

/// What the key's creation answers: the store, and where the backup key was written.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamKeyMade {
    view: crate::teamplugins::TeamPluginsView,
    backup: String,
}

/// An administrator makes the team's key and its backup on this machine (never the server) and declares them with the
/// namespace; the backup key's file goes to `backup_dir`, to keep elsewhere.
#[tauri::command]
pub async fn team_plugin_key_create(
    core: CoreState<'_>,
    team: String,
    namespace: String,
    password: String,
    backup_dir: String,
) -> Res<TeamKeyMade> {
    let (view, backup) = core
        .team_plugin_key_create(
            &team,
            &namespace,
            &password,
            std::path::Path::new(&backup_dir),
        )
        .await?;
    Ok(TeamKeyMade { view, backup })
}

/// Another administrator's machine takes the team's sealed key (or its backup) from a file.
#[tauri::command]
pub async fn team_plugin_key_import(
    core: CoreState<'_>,
    team: String,
    file: String,
    password: String,
) -> Res<crate::teamplugins::TeamPluginsView> {
    core.team_plugin_key_import(&team, std::path::Path::new(&file), &password)
        .await
}

/// A member proposes the plugin of a folder: the checks of the store played here, the compilation twice.
#[tauri::command]
pub async fn team_plugin_submit(
    core: CoreState<'_>,
    team: String,
    folder: String,
    tag: String,
) -> Res<crate::teamplugins::Submitted> {
    core.team_plugin_submit(&team, std::path::Path::new(&folder), &tag)
        .await
}

/// What the signature answers: this machine's checks, and the store.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSigned {
    report: escouade_plugin_cli::team::Report,
    view: crate::teamplugins::TeamPluginsView,
}

/// An administrator plays the checks again and signs a proposal by the team's key when the digests are the same.
#[tauri::command]
pub async fn team_plugin_sign(
    core: CoreState<'_>,
    team: String,
    proposal: String,
    password: String,
) -> Res<TeamSigned> {
    let (report, view) = core.team_plugin_sign(&team, &proposal, &password).await?;
    Ok(TeamSigned { report, view })
}

/// An administrator withdraws versions of a team's plugin: the team's revocation list, signed, its serial one further.
#[tauri::command]
pub async fn team_plugin_revoke(
    core: CoreState<'_>,
    team: String,
    plugin: String,
    versions: Option<Vec<String>>,
    reason: String,
    password: String,
    revoke_key: Option<String>,
) -> Res<crate::teamplugins::TeamPluginsView> {
    core.team_plugin_revoke(&team, &plugin, versions, &reason, &password, revoke_key)
        .await
}

/// « Réessayer » after an alarm of the team's store.
#[tauri::command]
pub async fn team_plugins_retry(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::teamplugins::TeamPluginsView> {
    teams.plugins_retry(&team).await?;
    core.team_plugins(&team).await
}

/// A proposal set aside.
#[tauri::command]
pub async fn team_plugin_dismiss(
    core: CoreState<'_>,
    team: String,
    proposal: String,
) -> Res<crate::teamplugins::TeamPluginsView> {
    core.team_plugin_dismiss(&team, &proposal).await
}

/// What installing a team's plugin would do: checked again, set aside for the window of its permissions
/// (`plugin_install` installs it, the member's gesture).
#[tauri::command]
pub async fn team_plugin_inspect(
    core: CoreState<'_>,
    team: String,
    id: String,
    version: Option<String>,
) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
    core.team_plugin_inspect(&team, &id, version.as_deref())
        .await
}

/// « Projets de l'équipe »: the team's list, read from the server, next to the projects of this device.
#[tauri::command]
pub async fn team_projects(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::team_projects::TeamProjectsView> {
    let of = teams.catalog(&team).await?;
    Ok(team_projects_of(&core, &team, of).await)
}

/// An administrator adds a project to the team's list (`project` null) or changes it, whole.
#[tauri::command]
pub async fn team_project_put(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    project: Option<String>,
    listed: escouade_team::catalog::Listed,
) -> Res<crate::team_projects::TeamProjectsView> {
    let (_, of) = teams.catalog_put(&team, project.as_deref(), listed).await?;
    Ok(team_projects_of(&core, &team, of).await)
}

/// An administrator's « Inscrire le Kanban »: the Kanban a member proposed for a project of the list (TS-7).
#[tauri::command]
pub async fn team_project_accept_board(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    project: String,
) -> Res<crate::team_projects::TeamProjectsView> {
    core.team_accept_board(&team, &project)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let of = teams.catalog(&team).await?;
    Ok(team_projects_of(&core, &team, of).await)
}

/// An administrator takes a project out of the team's list (its clones stay).
#[tauri::command]
pub async fn team_project_remove(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    project: String,
) -> Res<crate::team_projects::TeamProjectsView> {
    let of = teams.catalog_remove(&team, &project).await?;
    Ok(team_projects_of(&core, &team, of).await)
}

/// « Réessayer » after an alarm of the team's list.
#[tauri::command]
pub async fn team_projects_retry(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
) -> Res<crate::team_projects::TeamProjectsView> {
    let of = teams.catalog_retry(&team).await?;
    Ok(team_projects_of(&core, &team, of).await)
}

/// « Cloner » in the wizard: the repositories of the project `project` of the team's list (read from the list, never
/// from the window) into `parent/folder`, by the member's own accounts on the host of code.
#[tauri::command]
pub async fn team_project_clone(
    teams: TeamsState<'_>,
    team: String,
    project: String,
    parent: String,
    folder: String,
) -> Res<crate::team_projects::CloneView> {
    let of = teams.catalog(&team).await?;
    let item = of.catalog.get(&project).ok_or_else(|| {
        tr!(
            "Ce projet n’est plus dans la liste de l’équipe.",
            "This project is no longer in the team’s list."
        )
    })?;
    crate::team_projects::clone_listed(
        &item.project,
        std::path::Path::new(parent.trim()),
        &folder,
        &[],
    )
    .await
}

/// The logo of a project of the team's list as a file, for the wizard to take it as « Choisir une image… » does.
#[tauri::command]
pub async fn team_project_logo(
    core: CoreState<'_>,
    teams: TeamsState<'_>,
    team: String,
    project: String,
) -> Res<Option<String>> {
    let of = teams.catalog(&team).await?;
    let Some(logo) = of
        .catalog
        .get(&project)
        .and_then(|i| i.project.logo.clone())
    else {
        return Ok(None);
    };
    let dir = core.data.root().join("teams").join("logos");
    Ok(crate::team_projects::logo_file(&dir, &project, &logo)
        .map(|p| p.to_string_lossy().to_string()))
}

// ---------- The launches' pages (ESC-311) ----------

/// Whether an address is Escouade's own (`navigation::destination`), never loaded in a page.
#[cfg(feature = "desktop")]
fn own_of(app: &tauri::AppHandle) -> crate::devpage::Own {
    use ::tauri::Manager;
    let dev = app
        .try_state::<crate::navigation::Home>()
        .and_then(|h| h.dev.clone());
    Arc::new(move |u: &tauri::Url| {
        crate::navigation::destination(u, dev.as_ref()) == crate::navigation::Destination::App
    })
}

/// What happens to the pages, to the window.
fn tell_of(core: &Arc<Core>) -> crate::devpage::Tell {
    let hub = Arc::downgrade(core);
    Arc::new(move |page| {
        if let Some(core) = hub.upgrade() {
            core.hub.emit(UiEvent::DevPage { page });
        }
    })
}

/// Opens the page of agent `agent_id`'s launch at `url` (or loads it in the page already open):
/// in the window at `bounds` (ESC-657: a browser's row; the « Pages » window is gone).
#[cfg(feature = "desktop")]
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn devpage_open(
    app: tauri::AppHandle,
    core: CoreState,
    pages: State<Arc<crate::devpage::Pages>>,
    agent_id: String,
    title: String,
    url: String,
    bounds: Option<crate::devpage::Bounds>,
    project_id: Option<String>,
) -> Res<crate::devpage::PageView> {
    // A page without an agent (ESC-657), in the window: a browser the user opened alone
    // (`browser:<id>`), or the page of a launch command run in its project
    // (`run:<project>:<command>`, ESC-560).
    let project_id = match project_id {
        Some(p) => {
            crate::devpage::agentless_key(&agent_id, &p)?;
            core.project(&p).map_err(err)?;
            p
        }
        None => core
            .agent(&agent_id)
            .map_err(err)?
            .lock()
            .meta
            .project_id
            .clone(),
    };
    // Its project's launch off (ESC-658): its pages do not open.
    core.part_check(&project_id, crate::parts::Part::Launch)?;
    let own = own_of(&app);
    crate::devpage::open(
        &app,
        &pages,
        crate::devpage::Open {
            agent_id: &agent_id,
            project_id: &project_id,
            title: &title,
            url: &url,
            bounds,
            data_root: core.data.root(),
            visible: bounds.is_some(),
        },
        own,
        tell_of(&core),
    )?;
    pages.get(&agent_id).ok_or_else(|| "no page".into())
}

/// The page of agent `agent_id` in its view: placed at `bounds`, or hidden (none).
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn devpage_place(
    app: tauri::AppHandle,
    pages: State<Arc<crate::devpage::Pages>>,
    agent_id: String,
    bounds: Option<crate::devpage::Bounds>,
) -> Res<()> {
    crate::devpage::place(&app, &pages, &agent_id, bounds)
}

/// Back, forward, reload, or `to` (an address typed in its bar).
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn devpage_go(
    app: tauri::AppHandle,
    agent_id: String,
    action: String,
    to: Option<String>,
) -> Res<()> {
    crate::devpage::go(&app, &agent_id, &action, to.as_deref())
}

/// The devtools of the page of agent `agent_id`.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn devpage_devtools(app: tauri::AppHandle, agent_id: String) -> Res<()> {
    crate::devpage::devtools(&app, &agent_id)
}

/// Shows the page of agent `agent_id` at a width: `full`, `tablet` or `mobile`.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn devpage_mode(
    pages: State<Arc<crate::devpage::Pages>>,
    agent_id: String,
    mode: crate::devpage::PageMode,
) -> Res<()> {
    crate::devpage::set_mode(&pages, &agent_id, mode)
}

/// Closes the page of agent `agent_id`.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn devpage_close(
    app: tauri::AppHandle,
    pages: State<Arc<crate::devpage::Pages>>,
    agent_id: String,
) -> Res<()> {
    crate::devpage::close(&app, &pages, &agent_id)
}

// ---------- scheduled tasks (ESC-448) ----------

#[tauri::command]
pub fn scheduled_create(
    core: CoreState,
    project_id: String,
    draft: crate::scheduled::ScheduledDraft,
) -> Res<crate::scheduled::ScheduledView> {
    core.scheduled_create(&project_id, draft).map_err(err)
}

#[tauri::command]
pub fn scheduled_update(
    core: CoreState,
    id: String,
    draft: crate::scheduled::ScheduledDraft,
) -> Res<crate::scheduled::ScheduledView> {
    core.scheduled_update(&id, draft).map_err(err)
}

/// The switch of a task's row: paused, or resumed.
#[tauri::command]
pub fn scheduled_pause(
    core: CoreState,
    id: String,
    paused: bool,
) -> Res<crate::scheduled::ScheduledView> {
    core.scheduled_set_paused(&id, paused).map_err(err)
}

#[tauri::command]
pub fn scheduled_delete(core: CoreState, id: String) -> Res<()> {
    core.scheduled_delete(&id).map_err(err)
}

/// « Prochaines » of the form: the next due times of a rhythm, or why it is refused.
#[tauri::command]
pub fn scheduled_preview(core: CoreState, rhythm: crate::model::Rhythm) -> Res<Vec<i64>> {
    core.scheduled_preview(&rhythm).map_err(err)
}

// ---------- the editor's language servers (ESC-446) ----------

type Servers<'a> = State<'a, Arc<crate::lsp::Manager>>;

/// The catalog's servers for a project (or an agent's worktree), or for none (the machine's alone): found or not, where,
/// turned on, trusted.
#[tauri::command]
pub async fn lsp_servers(
    core: CoreState<'_>,
    project_id: Option<String>,
    agent_id: Option<String>,
) -> Res<Vec<crate::lsp::Status>> {
    let inputs = match project_id {
        Some(p) => core.lsp_inputs(&p, agent_id).await.map_err(err)?,
        None => core.lsp_machine_inputs(),
    };
    Ok(inputs.statuses())
}

/// What `lsp_start` did: the server running for the view, or why not (`refused`, said in `message`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspStart {
    info: Option<crate::lsp::Info>,
    refused: Option<crate::lsp::Refused>,
    message: Option<String>,
}

/// The server of `server` (a server's id or a language's) in a project, for `view`: the one running there, or one
/// started. Its messages come as `lsp-message`, its states as `lsp-state`.
#[tauri::command]
pub async fn lsp_start(
    core: CoreState<'_>,
    servers: Servers<'_>,
    view: String,
    project_id: String,
    agent_id: Option<String>,
    server: String,
) -> Res<LspStart> {
    // The feature Éditeur off (F-1): no language server starts.
    core.feature_check(crate::features::Feature::Editor)?;
    let inputs = core.lsp_inputs(&project_id, agent_id).await.map_err(err)?;
    let refused = |r: crate::lsp::Refused| LspStart {
        info: None,
        message: Some(r.to_string()),
        refused: Some(r),
    };
    let def = match crate::lsp::server_named(&server) {
        Ok(d) => d,
        Err(r) => return Ok(refused(r)),
    };
    match inputs.launch(def) {
        Ok(launch) => {
            let root = inputs.root.display().to_string();
            let info = servers.acquire(&view, &project_id, def.id, &root, launch);
            Ok(LspStart {
                info: Some(info),
                refused: None,
                message: None,
            })
        }
        Err(r) => Ok(refused(r)),
    }
}

#[tauri::command]
pub async fn lsp_send(servers: Servers<'_>, id: u32, msg: String) -> Res<()> {
    servers.send(id, &msg).await.map_err(err)
}

/// `view` no longer needs the server `id`: it stops a while after the last view lets go of it.
#[tauri::command]
pub async fn lsp_release(servers: Servers<'_>, view: String, id: u32) -> Res<()> {
    servers.release(&view, id);
    Ok(())
}

/// `view` is gone: every server it held is let go of.
#[tauri::command]
pub async fn lsp_release_view(servers: Servers<'_>, view: String) -> Res<()> {
    servers.release_view(&view);
    Ok(())
}

/// Starts the server `id` again, one that failed too.
#[tauri::command]
pub async fn lsp_restart(servers: Servers<'_>, id: u32) -> Res<()> {
    servers.restart(id).map_err(err)
}

#[tauri::command]
pub async fn lsp_running(servers: Servers<'_>) -> Res<Vec<crate::lsp::Info>> {
    Ok(servers.list())
}

/// The log of the server `id`: its stderr, and what the core did with it.
#[tauri::command]
pub async fn lsp_log(servers: Servers<'_>, id: u32) -> Res<Vec<crate::lsp::manager::LogLine>> {
    servers.log(id).map_err(err)
}

/// Trusts the project with its own language servers (a `node_modules/.bin` of its, the path and arguments of its
/// `.escouade.yml`), or no longer: those running of it stop when no longer trusted.
#[tauri::command]
pub async fn lsp_trust(
    core: CoreState<'_>,
    servers: Servers<'_>,
    project_id: String,
    trusted: bool,
) -> Res<()> {
    core.lsp_trust(&project_id, trusted).map_err(err)?;
    if !trusted {
        servers.stop_of_project(&project_id);
    }
    Ok(())
}

// ---------- the Overseer (ESC-462) ----------

/// What the window reads of the Overseer: whether it runs, its instructions, its journal.
#[tauri::command]
pub fn overseer_view(core: CoreState) -> Res<crate::overseer_core::OverseerView> {
    Ok(core.overseer_view())
}

/// A message of the user to the Overseer (its discussion), typed or said: answers its id.
#[tauri::command]
pub async fn overseer_send(core: CoreState<'_>, text: String, said: bool) -> Res<String> {
    core.overseer_send(&text, said).await.map_err(err)
}

/// « Annuler » of one of its actions.
#[tauri::command]
pub async fn overseer_undo(core: CoreState<'_>, id: String) -> Res<()> {
    core.overseer_undo(&id).await.map_err(err)
}

/// « Retirer » of one of its instructions.
#[tauri::command]
pub fn overseer_revoke(core: CoreState, id: String) -> Res<()> {
    core.overseer_revoke(&id).map(|_| ()).map_err(err)
}

// ---------- the box « À toi » (ESC-509): the commands its screen (IN-2) reads and acts by ----------

/// The box: the requests of a project, or of all (`project_id` left out); nothing while the MCP
/// server is off.
#[tauri::command(async)]
pub fn inbox_list(core: CoreState, project_id: Option<String>) -> crate::asks::InboxView {
    core.inbox_list(project_id.as_deref())
}

/// A request whole.
#[tauri::command(async)]
pub fn inbox_get(core: CoreState, id: String) -> Res<crate::asks::WindowAsk> {
    core.inbox_get(&id).map_err(err)
}

/// A part of a request's file (4 MB at most), in base64: its images, its PDF.
#[tauri::command(async)]
pub fn inbox_file(
    core: CoreState,
    id: String,
    file: String,
    offset: Option<u64>,
    length: Option<u64>,
) -> Res<crate::asks::InboxChunk> {
    core.inbox_file(&id, &file, offset.unwrap_or(0), length)
        .map_err(err)
}

/// A request's file whole, in raw bytes: binary frames on the remote API (ESC-495), parted beyond
/// 16 MB, never base64.
#[tauri::command]
pub async fn inbox_file_bytes(
    core: CoreState<'_>,
    id: String,
    file: String,
) -> Res<tauri::ipc::Response> {
    let core = core.inner().clone();
    let bytes = tokio::task::spawn_blocking(move || core.inbox_file_bytes(&id, &file))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// « Ouvrir » or « Afficher dans le dossier » of a request's file.
#[tauri::command(async)]
pub fn inbox_open_file(core: CoreState, id: String, file: String, reveal: bool) -> Res<()> {
    core.inbox_open_file(&id, &file, reveal).map_err(err)
}

/// The user's answer: it goes to its author 10 seconds later (`inbox_reopen` takes it back).
#[tauri::command(async)]
pub fn inbox_answer(
    core: CoreState,
    id: String,
    answer: crate::asks::AnswerIn,
    files: Option<Vec<AttachedFile>>,
) -> Res<crate::asks::WindowAsk> {
    let a = core
        .inbox_answer(&id, answer, files.unwrap_or_default())
        .map_err(err)?;
    if let Some(event) = crate::asks::answered_event(&a.ask) {
        core.telemetry.track(event);
    }
    Ok(a)
}

/// The box closed with the server: what its commands say.
fn inbox_open(core: &Core) -> Res<()> {
    if core.inbox_enabled() {
        Ok(())
    } else {
        Err(tr!(
            "La boîte « À toi » est fermée avec le serveur MCP : allume-le dans Réglages › Agents › Piloter Escouade.",
            "The “Your turn” box is closed with the MCP server: turn it on in Settings › Agents › Drive Escouade."
        ))
    }
}

/// « Plus tard » until `until` (ms since 1970); left out, it comes back now.
#[tauri::command(async)]
pub fn inbox_snooze(
    core: CoreState,
    id: String,
    until: Option<i64>,
) -> Res<crate::asks::WindowAsk> {
    inbox_open(&core)?;
    let a = core.ask_snooze(&id, until).map_err(err)?;
    if until.is_some() {
        core.telemetry.track(Event::AskSnoozed);
    }
    core.inbox_get(&a.id).map_err(err)
}

/// « Laisser <agent> trancher » (`agent`), or taken back (left out).
#[tauri::command(async)]
pub fn inbox_delegate(
    core: CoreState,
    id: String,
    agent: Option<String>,
) -> Res<crate::asks::WindowAsk> {
    inbox_open(&core)?;
    let a = core.ask_delegate(&id, agent.as_deref()).map_err(err)?;
    core.inbox_get(&a.id).map_err(err)
}

/// « Revenir sur ma réponse »: before its author read it.
#[tauri::command(async)]
pub fn inbox_reopen(core: CoreState, id: String) -> Res<crate::asks::WindowAsk> {
    inbox_open(&core)?;
    let a = core.ask_reopen(&id).map_err(err)?;
    core.telemetry.track(Event::AskReopened);
    core.inbox_get(&a.id).map_err(err)
}

/// What the window alone sees of the box (ESC-510): opened (`from`: tile, shortcut, settings; `scope`: all, project),
/// « Comparer en grand », a file looked at (`kind`). Only these values are counted: anything else is dropped.
#[tauri::command(async)]
pub fn inbox_track(
    core: CoreState,
    event: String,
    from: Option<String>,
    scope: Option<String>,
    kind: Option<String>,
) {
    let pick = |v: Option<String>, of: &[&'static str]| -> Option<&'static str> {
        v.and_then(|v| of.iter().copied().find(|x| *x == v))
    };
    let tracked = match event.as_str() {
        "opened" => pick(from, &["tile", "shortcut", "settings", "notification"]).map(|from| {
            Event::InboxOpened {
                from,
                scope: pick(scope, &["all", "project"]).unwrap_or("all"),
            }
        }),
        "compare" => Some(Event::InboxCompareOpened),
        "attachment" => pick(kind, &["image", "pdf", "markdown", "word", "csv", "other"])
            .map(|kind| Event::InboxAttachmentOpened { kind }),
        _ => None,
    };
    if let Some(e) = tracked {
        core.telemetry.track(e);
    }
}

/// The user saw the request.
#[tauri::command(async)]
pub fn inbox_seen(core: CoreState, id: String) -> Res<()> {
    inbox_open(&core)?;
    core.ask_seen(&id).map_err(err)
}

/// What the question before turning the MCP server off lists: the requests that wait.
#[tauri::command(async)]
pub fn inbox_closing(core: CoreState) -> crate::asks::InboxClosing {
    core.inbox_closing()
}

/// What the box's files weigh, in bytes (shown with the space the app takes, IN-2).
#[tauri::command(async)]
pub fn inbox_weight(core: CoreState) -> u64 {
    core.asks_weight()
}

/// « Vider les fichiers des demandes closes »: what it freed, in bytes.
#[tauri::command(async)]
pub fn inbox_erase_closed_files(core: CoreState) -> u64 {
    core.asks_erase_closed_files()
}

// ---------- « Se connecter à un serveur » (ESC-537) ----------
//
// The servers this computer knows, their tunnel and their pairing: always this computer's (`desk`), whichever core the
// window speaks to.

#[cfg(feature = "desktop")]
type ServersState<'a> = State<'a, Arc<crate::servers::Servers>>;

/// The servers this computer knows, and the one the window is on.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn servers_view(servers: ServersState, core: CoreState) -> crate::servers::View {
    let mut view = servers.view();
    view.locks = Some(core.locks.read().clone());
    view
}

/// The hosts of `~/.ssh/config`, for the dialog.
#[cfg(feature = "desktop")]
#[tauri::command(async)]
pub fn servers_ssh_hosts(servers: ServersState) -> Vec<crate::servers::sshconfig::SshHost> {
    servers.ssh_hosts()
}

/// Where a window connects, read before each of its tries: the main window to the server it is on (none on this
/// computer); a space's own window (ESC-539, `space`: a server's id, or `here` for this computer) to that space.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_link(servers: ServersState, space: Option<String>) -> Option<crate::servers::Link> {
    match space.as_deref() {
        None => servers.link(),
        Some(crate::servers::window::HERE) => None,
        Some(id) => servers.link_of(id),
    }
}

/// Where to reach every server known (ESC-539): the piles of the rail and « Partout » open a connection of their own
/// to each space the window is not on.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_links(servers: ServersState) -> Vec<crate::servers::Link> {
    servers.links()
}

/// A space in its own window (ESC-539, S14): `space`, a server's id or `here`; one window per space, a second demand
/// bringing back the first (the main window when it is on that space). `existing`: only brought back when there is
/// one, never made. Whether a window of that space is in front now.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn space_window(
    app: tauri::AppHandle,
    servers: ServersState,
    space: String,
    existing: Option<bool>,
) -> Res<bool> {
    use tauri::Manager;
    if existing == Some(true) {
        return Ok(
            // A window being closed may still be listed a moment: only one on screen counts.
            match app
                .get_webview_window(&crate::servers::window::label(&space))
                .filter(|w| w.is_visible().unwrap_or(false))
            {
                Some(w) => {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                    true
                }
                None => false,
            },
        );
    }
    let here = space == crate::servers::window::HERE;
    let title = if here {
        tr!("Ce poste", "This computer")
    } else {
        servers
            .view()
            .servers
            .into_iter()
            .find(|s| s.known.id == space)
            .map(|s| s.known.name)
            .ok_or_else(|| {
                tr!(
                    "Ce serveur n’est plus connu de ce poste.",
                    "This server is no longer known to this computer."
                )
            })?
    };
    let main_on = match servers.active() {
        Some(active) => active == space,
        None => here,
    };
    crate::servers::window::open(&app, &space, &title, main_on).map(|()| true)
}

/// A notification of another space (ESC-539): what a pile's glance found waiting there, named by its space in its
/// title (« Travail · facturation · relances-impayes »). This computer's settings say whether it shows (its sound, its
/// toast); a click brings the main window back and tells it (`space-focus`) to go to that space and that agent.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn space_notify(
    app: tauri::AppHandle,
    core: CoreState,
    space: String,
    title: String,
    body: String,
    agent_id: Option<String>,
    project_id: Option<String>,
) {
    use tauri::Emitter;
    let settings = core.settings.read().clone();
    if !settings
        .notify_for
        .allows(crate::agent::NotifyKind::Question)
    {
        return;
    }
    if settings.sound {
        crate::notify::play_chime();
    }
    if crate::notify::window_attended(&app) {
        return;
    }
    crate::notify::flash(&app);
    if settings.os_notifications {
        let handle = app.clone();
        crate::notify::toast(&app, &title, &body, move || {
            crate::notify::show_main(&handle);
            let _ = handle.emit_to(
                "main",
                "space-focus",
                serde_json::json!({ "space": space, "agentId": agent_id, "projectId": project_id }),
            );
        });
    }
}

/// The dialog's step « Connexion »: the policy, the tunnel to `host`, the handshake without a token.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn server_attempt(
    servers: ServersState<'_>,
    host: String,
    remote_port: Option<u16>,
    existing: Option<String>,
) -> Res<crate::servers::Outcome<crate::servers::Reached>> {
    Ok(servers.attempt(&host, remote_port, existing).await)
}

/// « Le lancer d'ici par SSH »: the code `escouade-server pair` printed on the host.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn server_code_over_ssh(
    servers: ServersState<'_>,
    attempt: String,
) -> Res<crate::servers::Outcome<String>> {
    Ok(servers.code_over_ssh(&attempt).await)
}

/// The dialog's step « Appairage »: the server known, its token in this computer's keychain; its id.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn server_pair(
    servers: ServersState<'_>,
    attempt: String,
    code: String,
    name: String,
    device: String,
) -> Res<crate::servers::Outcome<String>> {
    Ok(servers.pair(&attempt, &code, &name, &device).await)
}

/// The dialog closed before its end: its tunnel stopped.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_cancel(servers: ServersState, attempt: String) {
    servers.cancel(&attempt);
}

/// The window to go on the server `id` (it loads again once this answers « ok »).
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn server_open(
    servers: ServersState<'_>,
    id: String,
) -> Res<crate::servers::Outcome<()>> {
    Ok(servers.inner().open(&id).await)
}

/// Back on this computer (the window loads again).
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_leave(servers: ServersState) {
    servers.leave();
}

/// « Réessayer »: the tunnel started again now: the one of the server `id` (a pile, « Partout », a space's window), or
/// of the server the window is on.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_retry(servers: ServersState, id: Option<String>) {
    match id {
        Some(id) => servers.inner().retry_of(&id),
        None => servers.inner().retry(),
    }
}

/// « Retirer » (S15): the server asked to revoke this computer when it answers, then forgotten here. Whether it
/// revoked it.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn server_forget(servers: ServersState<'_>, id: String) -> Res<bool> {
    Ok(servers.inner().forget(&id).await)
}

/// What stopped the window's tries on the server `id`, noted for Réglages › Serveurs.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_note(servers: ServersState, id: String, problem: crate::servers::Problem) {
    servers.note(&id, problem);
}

/// A server's name, the user's.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_rename(servers: ServersState, id: String, name: String) {
    servers.rename(&id, &name);
}

/// What Escouade puts in the box of its own (ESC-513): the views on what already waits for the user
/// elsewhere, of a project or of all, with the counts of every project. Nothing while the MCP
/// server is off.
#[tauri::command]
pub async fn inbox_views(
    core: CoreState<'_>,
    project_id: Option<String>,
) -> Res<crate::inboxviews::InboxViews> {
    Ok(core.inbox_views(project_id.as_deref()).await)
}

/// A gesture on a view of the box: the one of its source, on the view the user read.
#[tauri::command]
pub async fn inbox_view_act(
    core: CoreState<'_>,
    key: String,
    act: crate::inboxviews::ViewAct,
) -> Res<()> {
    core.inbox_view_act(&key, act).await.map_err(err)
}

/// « Plus tard » on a view (`until`, epoch ms; none brings it back): its source is not touched.
#[tauri::command]
pub async fn inbox_view_snooze(core: CoreState<'_>, key: String, until: Option<i64>) -> Res<()> {
    core.inbox_view_snooze(&key, until).await.map_err(err)
}

/// « Remettre dans la boîte » (ESC-557): a view the squad lead took in hand or ticked as settled waits for the user
/// again; its source is not touched.
#[tauri::command]
pub async fn inbox_view_reopen(core: CoreState<'_>, key: String) -> Res<()> {
    core.inbox_view_reopen(&key).await.map_err(err)
}

/// « Enregistrer sous… » of a request's file (ESC-510): its copy written where the user picks in the system's dialog,
/// which the core opens; the window never gives a path. False when the user cancels.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn inbox_save_file(
    app: tauri::AppHandle,
    core: CoreState<'_>,
    id: String,
    file: String,
) -> Res<bool> {
    use tauri_plugin_dialog::DialogExt;
    let (f, from) = core.inbox_file_copy(&id, &file).map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<bool> {
        let Some(path) = app
            .dialog()
            .file()
            .set_file_name(&f.name)
            .blocking_save_file()
        else {
            return Ok(false);
        };
        std::fs::copy(from, path.into_path()?)?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

// ---------- « Partout » and the space's devices (ESC-539) ----------
//
// Served: the window asks them of each space through its own connection (this computer's core over the IPC).

/// What waits in this core at a glance: the piles of the rail and « Partout » read it of a space the window is not on.
/// `counts`: the numbers alone (the window's policy denies the aggregate view for this space).
#[tauri::command(async)]
pub fn space_glance(core: CoreState, counts: Option<bool>) -> crate::glance::Glance {
    core.glance(counts.unwrap_or(false))
}

/// The devices paired to this core's remote API (Réglages de l'espace › Appareils).
#[tauri::command(async)]
pub fn remote_devices(core: CoreState) -> Vec<crate::remoteapi::devices::Device> {
    crate::remoteapi::devices::Devices::load(&core.data, core.secrets.clone()).list()
}

/// Who asks a revocation, as its trace keeps it (SR1-02): the device of the connection that called, by its name, or
/// the window; never a name the caller gives.
fn revoker(devices: &crate::remoteapi::devices::Devices) -> String {
    match crate::remoteapi::current_device() {
        Some(id) => devices.name_of(&id).unwrap_or(id),
        None => tr!("la fenêtre d’Escouade", "Escouade’s window"),
    }
}

/// The device `id` revoked, and the devices paired from it with it (SR1-02): the API closes their connections within
/// two seconds. Who asked is the caller (`revoker`). False when there was none.
#[tauri::command(async)]
pub fn remote_device_revoke(core: CoreState, id: String) -> Res<bool> {
    let devices = crate::remoteapi::devices::Devices::load(&core.data, core.secrets.clone());
    let by = revoker(&devices);
    devices.revoke_by(&id, Some(&by)).map_err(err)
}

/// A pairing code (S5): ten minutes, one device; it takes the place of the one that waited. Asked by the window alone:
/// a device paired to the API is refused, so that a leaked token cannot make devices of its own (SR1-02).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairCode {
    pub code: String,
    pub expires_at: i64,
}

#[tauri::command(async)]
pub fn remote_pair_code(core: CoreState) -> Res<PairCode> {
    if crate::remoteapi::current() != crate::remoteapi::Client::Window {
        return Err(tr!(
            "Un code d’appairage ne se crée pas depuis un appareil appairé : lance « escouade-server pair » sur le serveur, ou crée-le dans la fenêtre d’Escouade de cette machine.",
            "A pairing code is not created from a paired device: run “escouade-server pair” on the server, or create it in Escouade’s window on that machine."
        ));
    }
    let code = crate::remoteapi::pairing::create_by(
        &core.data,
        crate::model::now_ms(),
        crate::remoteapi::current_device().as_deref(),
    )
    .map_err(err)?;
    Ok(PairCode {
        code: code.text,
        expires_at: code.expires_at,
    })
}

// ---------- Réglages › Téléphone (ESC-567) ----------
//
// Served: a server's window asks them of its core (Réglages de l'espace › Téléphone), the desktop's of its own.

/// The phone's access as Réglages › Téléphone shows it: on or off, the port, the tunnel, the box, the public address,
/// Tailscale, the phones.
#[tauri::command]
pub async fn phone_status(core: CoreState<'_>) -> Res<crate::phone::Status> {
    let core = core.inner().clone();
    Ok(core.phone.status(&core).await)
}

/// The phone's access changed: on or off, its UDP port, Tailscale on or off; applied at once. Refused when a policy
/// of the device forbids it (`servers.remoteApi`, `servers.phone`), or when the port does not open (said as such).
#[tauri::command]
pub async fn phone_set(
    core: CoreState<'_>,
    enabled: Option<bool>,
    port: Option<u16>,
    tailscale: Option<bool>,
) -> Res<crate::phone::Status> {
    let core = core.inner().clone();
    core.phone
        .set(&core, enabled, port, tailscale)
        .await
        .map_err(err)?;
    Ok(core.phone.status(&core).await)
}

/// The network looked at again now: the box, the public address, Tailscale.
#[tauri::command]
pub async fn phone_check(core: CoreState<'_>) -> Res<crate::phone::Status> {
    let core = core.inner().clone();
    core.phone.check().await;
    Ok(core.phone.status(&core).await)
}

/// The tunnel's key changed: every phone must be paired again.
#[tauri::command]
pub async fn phone_rotate_key(core: CoreState<'_>) -> Res<crate::phone::Status> {
    let core = core.inner().clone();
    core.phone.rotate_key().map_err(err)?;
    Ok(core.phone.status(&core).await)
}

/// A phone removed (Réglages › Téléphone): what was done.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneRemoved {
    /// Its peer was one, and is gone from the tunnel.
    pub removed: bool,
    /// Its peer was paired as a device (`peer.device`): that device's token was revoked with it.
    pub revoked: bool,
    /// Its peer was never paired as a device: no token was tied to it, none was revoked (said so).
    pub never_linked: bool,
}

/// A phone removed from the tunnel, its sessions and connections cut, and the device it was paired as revoked (SR1-03),
/// with the devices paired from it: its token no longer passes by any way (Tailscale, the loopback, another peer).
#[tauri::command(async)]
pub fn phone_remove(core: CoreState, id: String) -> Res<PhoneRemoved> {
    let Some(peer) = core.phone.take_peer(&id).map_err(err)? else {
        return Ok(PhoneRemoved {
            removed: false,
            revoked: false,
            never_linked: false,
        });
    };
    let Some(device) = peer.device else {
        return Ok(PhoneRemoved {
            removed: true,
            revoked: false,
            never_linked: true,
        });
    };
    let devices = crate::remoteapi::devices::Devices::load(&core.data, core.secrets.clone());
    let by = revoker(&devices);
    let revoked = devices.revoke_by(&device, Some(&by)).map_err(err)?;
    Ok(PhoneRemoved {
        removed: true,
        revoked,
        never_linked: false,
    })
}

// ---------- What changes machines on a server (ESC-538) ----------
//
// This computer's side of what a server's window does here: the files of its agents kept in the downloads, its ports
// relayed, its notifications shown. Always this computer's (`desk`).

/// A file of a server's agent received on this computer (S12): its bytes, raw, kept in `Téléchargements/Escouade/<espace>`
/// (headers `x-space`, `x-name`), then opened (`x-open: 1`) when it is an image or a PDF (ESC-629), else shown in its
/// folder (as with `x-reveal: 1`). What it became.
#[cfg(feature = "desktop")]
#[tauri::command(async)]
pub fn received_save(
    core: CoreState,
    request: crate::remoteapi::Body,
) -> Res<crate::servers::received::Received> {
    let Some(bytes) = request.raw() else {
        return Err(tr!(
            "Les octets d’un fichier reçu viennent bruts, pas en JSON.",
            "The bytes of a received file come raw, not as JSON."
        ));
    };
    let space = header(&request, "x-space").unwrap_or_default();
    let name = header(&request, "x-name").unwrap_or_else(|| "fichier".into());
    let downloads = crate::servers::received::downloads();
    let mut got = crate::servers::received::save(&downloads, &space, &name, bytes).map_err(err)?;
    let path = std::path::PathBuf::from(&got.path);
    // Come from another computer: Gatekeeper judges it as any download (macOS).
    crate::quarantine::mark(&path);
    // Opened only when it is on the list of what Escouade opens by the system (ESC-629): an image or a PDF.
    if request.header("x-open") == Some("1")
        && got.opens == crate::servers::received::Opening::System.as_str()
    {
        core.host
            .open(
                crate::host::Opened::Path(path),
                std::slice::from_ref(&downloads),
            )
            .map_err(err)?;
        got.opened = true;
    } else if request.header("x-reveal") == Some("1") || request.header("x-open") == Some("1") {
        core.host
            .open(crate::host::Opened::Reveal(path), &[downloads])
            .map_err(err)?;
    }
    Ok(got)
}

/// A copy received before, opened again or shown in its folder: only one of Escouade's downloads, only an image or a PDF
/// opened (ESC-629).
#[cfg(feature = "desktop")]
#[tauri::command(async)]
pub fn received_show(core: CoreState, path: String, reveal: bool) -> Res<()> {
    let downloads = crate::servers::received::downloads();
    let file = crate::servers::received::shown(&downloads, std::path::Path::new(&path), reveal)
        .map_err(err)?;
    core.host
        .open(
            if reveal {
                crate::host::Opened::Reveal(file)
            } else {
                crate::host::Opened::Path(file)
            },
            &[downloads],
        )
        .map_err(err)
}

/// The port `port` of the server the window is on relayed to this computer's 127.0.0.1 (a test server; `same`: the
/// return of a sign-in on `localhost`, only on the same port). The ports, or why not.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn server_relay(
    servers: ServersState<'_>,
    port: u16,
    same: Option<bool>,
) -> Res<crate::servers::Outcome<crate::servers::Relayed>> {
    Ok(match servers.relay(port, same.unwrap_or(false)).await {
        Ok(value) => crate::servers::Outcome::Ok { value },
        Err(problem) => crate::servers::Outcome::Problem { problem },
    })
}

/// The ports relayed now.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_relays(servers: ServersState) -> Vec<crate::servers::Relayed> {
    servers.relays()
}

/// What a server's host published, shown on this computer (ESC-538): `chime`, `flash`, `notify` (`title` already named
/// by its space, `body`; `clicked` told when the user clicks it), `waiting` (the tray's count). The taskbar and the
/// notification only when the window is not looked at, as the desktop does. Whether it was shown.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn server_alert(
    core: CoreState,
    kind: String,
    title: Option<String>,
    body: Option<String>,
    count: Option<usize>,
    clicked: Channel<bool>,
) -> bool {
    // The e2e read what would be shown, nothing shown for real.
    if cfg!(debug_assertions) {
        if let Some(log) = std::env::var_os("ESCOUADE_ALERT_LOG").filter(|v| !v.is_empty()) {
            use std::io::Write;
            let line =
                serde_json::json!({ "kind": kind, "title": title, "body": body, "count": count });
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
            {
                let _ = writeln!(f, "{line}");
            }
            if kind == "notify" {
                let _ = clicked.send(true);
            }
            return true;
        }
    }
    match kind.as_str() {
        "chime" => core.host.chime(),
        "waiting" => core
            .host
            .show_waiting(count.unwrap_or(0), crate::i18n::ui()),
        "flash" if !core.host.attended() => core.host.flash(),
        "notify" if !core.host.attended() => {
            core.host.notify(
                &title.unwrap_or_default(),
                &body.unwrap_or_default(),
                Box::new(move || {
                    let _ = clicked.send(true);
                }),
            );
        }
        _ => return false,
    }
    true
}

/// The voice's settings of this computer (ESC-538): on a server, the window shows and saves these, not the server's —
/// the microphone, the transcription and the keys of the voice are this computer's.
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn local_voice_get(core: CoreState) -> crate::model::VoiceSettings {
    core.settings.read().voice.clone()
}

/// The voice's settings of this computer saved (ESC-538), the others as they are.
#[cfg(feature = "desktop")]
#[tauri::command(async)]
pub fn local_voice_set(
    core: CoreState,
    voice_state: Maybe<Arc<crate::voice::Voice>>,
    voice: crate::model::VoiceSettings,
) -> Res<()> {
    let mut settings = core.settings.read().clone();
    settings.voice = voice;
    core.save_window_settings(settings).map_err(err)?;
    if let Some(v) = voice_state.get() {
        v.conversation_tune(&conv_choice(&core.settings.read(), &*core.secrets, &v.catalog).0);
    }
    Ok(())
}
