// The core alone (`escouade-server`, no feature `desktop`) leaves out the window and what serves it alone: what the
// desktop's modules lend the core is built, its other parts are unused there, which says nothing.
#![cfg_attr(not(feature = "desktop"), allow(dead_code, unused_imports))]
// First: its macros (`tr!`, `tr_in!`) are then in scope in every module below.
#[macro_use]
extern crate escouade_plugin_cli;
// The core alone (`escouade-server`): what it takes of Tauri, without Tauri, under Tauri's name (docs/CRATES.md, § 14).
#[cfg(not(feature = "desktop"))]
extern crate escouade_tauri_shim as tauri;
#[macro_use]
mod i18n;
mod accounts;
mod agent;
mod agentbranch;
#[cfg(test)]
mod agentbranch_tests;
mod agentprs;
#[cfg(test)]
mod agentprs_tests;
#[cfg(test)]
mod agy_tests;
#[cfg(test)]
mod agy_tui_tests;
mod asks;
#[cfg(test)]
mod asks_tests;
mod attach;
mod attachfiles;
#[cfg(test)]
mod attachfiles_tests;
mod board;
mod branchci;
#[cfg(test)]
mod branches_tests;
mod bypass;
#[cfg(feature = "desktop")]
mod clipboard;
mod closed;
#[cfg(test)]
mod closed_tests;
mod codesearch;
#[cfg(test)]
mod codex_tests;
#[cfg(test)]
mod codex_tui_tests;
mod commands;
#[cfg(test)]
mod compat22_tests;
mod conv;
mod convsearch;
mod copies;
#[cfg(test)]
mod copilot_tests;
#[cfg(test)]
mod copilot_tui_tests;
mod core;
#[cfg(test)]
mod core_tests;
#[cfg(test)]
mod cursor_tests;
#[cfg(test)]
mod cursor_tui_tests;
#[cfg(feature = "desktop")]
mod deeplink;
#[cfg(test)]
mod deeplink_tests;
mod delegation;
mod deleteask;
#[cfg(test)]
mod deleteask_tests;
mod demo;
mod deploys;
#[cfg(feature = "desktop")]
mod desktop;
mod devpage;
#[cfg(feature = "desktop")]
mod devpage_probe;
#[cfg(test)]
mod devpage_tests;
mod discover;
mod discussion;
#[cfg(test)]
mod discussion_tests;
mod docs;
#[cfg(test)]
mod docs_tests;
mod docsbranch;
mod docscompare;
#[cfg(test)]
mod docscompare_tests;
mod docsdrive;
#[cfg(test)]
mod docsdrive_tests;
mod docsextract;
#[cfg(test)]
mod docsextract_tests;
#[cfg(test)]
mod docsfollow_tests;
mod docsimport;
#[cfg(test)]
mod docsimport_tests;
mod docsplan;
#[cfg(test)]
mod docsplan_tests;
mod docsshare;
#[cfg(test)]
mod docsshare_tests;
mod docsticket;
#[cfg(test)]
mod docsticket_tests;
mod engine;
#[cfg(test)]
mod engines_tests;
mod facts;
#[cfg(test)]
mod facts_tests;
mod features;
mod fence;
#[cfg(test)]
mod fence_tests;
#[cfg(test)]
mod fetch_tests;
mod fileorigin;
#[cfg(test)]
mod fileorigin_tests;
#[cfg(test)]
mod first_message_tests;
#[cfg(test)]
mod fluxbench;
mod folders;
#[cfg(test)]
mod folders_tests;
mod forge;
#[cfg(test)]
mod forgotten_tests;
mod fsedit;
#[cfg(test)]
mod gemini_tui_tests;
mod git;
mod glance;
#[cfg(test)]
mod glance_tests;
mod handover;
mod harness_scope;
#[cfg(test)]
mod harness_scope_tests;
/// The core without a window (ESC-451): what `escouade-server` will run.
pub mod headless;
#[cfg(test)]
mod headless_tests;
pub mod host;
mod hotkey;
#[cfg(test)]
mod hotkey_tests;
mod hub;
mod inboxnotify;
#[cfg(test)]
mod inboxnotify_tests;
mod inboxviews;
#[cfg(test)]
mod inboxviews_tests;
mod integrations;
#[cfg(test)]
mod integrations_tests;
mod isola;
mod launchenv;
mod leftovers;
mod locks;
mod lsp;
mod mcp;
mod memory;
#[cfg(feature = "desktop")]
mod menus;
mod model;
mod modes;
#[cfg(test)]
mod multi_repo_tests;
mod multiticket;
mod multiwt;
mod myreview;
#[cfg(test)]
mod myreview_tests;
#[cfg(feature = "desktop")]
mod navigation;
mod notify;
mod offers;
mod onboarding;
#[cfg(test)]
mod opencode_tests;
mod opening;
#[cfg(test)]
mod opening_tests;
mod outside;
mod overseer;
mod overseer_core;
mod overseer_reads;
#[cfg(test)]
mod overseer_reads_tests;
#[cfg(test)]
mod overseer_tests;
mod pace;
mod pagecrash;
mod parts;
mod paths;
pub mod phone;
#[cfg(test)]
mod phone_tests;
#[cfg(test)]
mod recovery_tests;
#[cfg(test)]
mod remote_tests;
pub mod remoteapi;
#[cfg(test)]
mod remoteapi_tests;
#[cfg(test)]
mod save_tests;
// What an agent is getting through, and the plan files it follows: the crate `escouade-plan`.
use escouade_plan::{plan, planfiles};
// The data and what it names, the processes and WSL: the crate `escouade-model` (`model` and `paths` re-export theirs).
use escouade_model::{agentgroups, heat, job, pricing, proc, wsl};
/// A plugin's profiles and its settings of each project, as the window sets them (ESC-530).
mod plugin_profiles;
#[cfg(test)]
mod plugin_profiles_tests;
mod plugin_secrets;
#[cfg(test)]
mod plugin_secrets_tests;
mod plugins;
#[cfg(test)]
mod plugins_accounts_tests;
#[cfg(test)]
mod plugins_acp_tests;
#[cfg(test)]
mod plugins_builtin_tests;
#[cfg(test)]
mod plugins_claude_acp_parity_tests;
#[cfg(test)]
mod plugins_contract_tests;
#[cfg(test)]
use escouade_forge::plugins_clickup_tests;
#[cfg(test)]
mod plugins_decide_tests;
#[cfg(test)]
mod plugins_harness_parity_tests;
#[cfg(test)]
mod plugins_harness_tests;
#[cfg(test)]
mod plugins_isola_tests;
#[cfg(test)]
mod plugins_loans_tests;
#[cfg(test)]
mod plugins_net_tests;
#[cfg(test)]
use escouade_plugins::plugins_protocol_tests;
#[cfg(test)]
use escouade_plugins::plugins_reactor_tests;
#[cfg(test)]
mod plugins_reader_tests;
#[cfg(test)]
use escouade_plugins::plugins_runtime_tests;
#[cfg(test)]
use escouade_plugins::plugins_seed_tests;
#[cfg(test)]
mod plugins_sort_tests;
#[cfg(test)]
use escouade_plugins::plugins_store_tests;
#[cfg(test)]
mod plugins_tests;
#[cfg(test)]
use escouade_forge::plugins_trello_tests;
#[cfg(test)]
mod plugins_view_parity_tests;
#[cfg(test)]
mod plugins_wasm_hosts_parity_tests;
#[cfg(test)]
mod plugins_wasm_parity_tests;
#[cfg(test)]
use escouade_plugins::plugins_wasm_tests;
mod offline;
mod pragent;
#[cfg(test)]
mod pragent_tests;
#[cfg(test)]
mod private_tests;
mod prnews;
#[cfg(test)]
mod process_tests;
mod projectfile;
#[cfg(test)]
mod projectfile_tests;
mod projectlogo;
mod proxysecret;
#[cfg(test)]
mod proxysecret_tests;
mod prread;
#[cfg(test)]
mod prread_tests;
mod prreview;
mod prview;
#[cfg(test)]
mod prview_tests;
mod prwrite;
#[cfg(test)]
mod prwrite_tests;
mod pty;
mod quarantine;
mod repos;
#[cfg(test)]
mod repos_tests;
mod resources;
mod scheduled;
#[cfg(test)]
mod scheduled_tests;
mod search;
mod selfupdate;
mod sentfiles;
#[cfg(test)]
mod sentfiles_tests;
pub mod server;
// What changes machines in server mode (ESC-538): the folders of the server, for « Nouveau projet ».
#[cfg(test)]
mod server_moves_tests;
#[cfg(test)]
mod server_tests;
mod serverfolders;
// « Se connecter à un serveur » (ESC-537): the client of the server mode, its SSH tunnel and its pairing.
mod parallel;
#[cfg(test)]
mod parallel_tests;
#[cfg(feature = "desktop")]
mod servers;
#[cfg(test)]
mod servers_tests;
mod sharing;
#[cfg(test)]
mod sharing_tests;
#[cfg(unix)]
mod shellenv;
mod signin;
#[cfg(test)]
mod signin_tests;
mod stats;
mod switching;
#[cfg(test)]
mod switching_tests;
mod syspath;
mod taskout;
mod team_breakers;
mod team_projects;
mod team_settings;
mod teamboard;
mod teamcosts;
#[cfg(test)]
mod teamcosts_tests;
mod teamdocs;
#[cfg(test)]
mod teamdocs_tests;
mod teamhandoff;
mod teamjoin;
mod teammemory;
mod teamplugins;
#[cfg(test)]
mod teamplugins_tests;
mod teampour;
mod teams;
#[cfg(test)]
mod teams_tests;
mod telemetry;
#[cfg(test)]
mod terminal_tests;
mod termlog;
#[cfg(test)]
mod termlog_tests;
#[cfg(test)]
mod test_tmp;
#[cfg(test)]
mod test_vars_tests;
mod testlaunch;
mod text;
mod third_party;
#[cfg(test)]
mod ticket_accounts_tests;
#[cfg(test)]
mod ticket_target_tests;
mod ticketfiles;
mod ticketimages;
mod ticketprs;
mod tickets;
#[cfg(test)]
mod tickets_tests;
mod tls;
mod tui;
#[cfg(test)]
mod tui_tests;
mod updates;
mod usage;
mod vectors;
#[cfg(test)]
mod vibe_tests;
#[cfg(test)]
mod vibe_tui_tests;
mod voice;
mod vsx;
use escouade_engines::which;
mod windowthread;
#[cfg(test)]
mod windowthread_tests;
mod workflow;
#[cfg(test)]
mod workflow_tests;
mod workflow_track;
#[cfg(test)]
mod workflow_track_tests;
mod workflowfiles;
#[cfg(test)]
mod workflowfiles_tests;
mod workflowfix;
#[cfg(test)]
mod workflowfix_tests;
mod workflowrefine;
#[cfg(test)]
mod workflowrefine_tests;
mod worktrees;

use crate::core::Core;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(feature = "desktop")]
use tauri::{Manager, WebviewWindowBuilder, WindowEvent};
#[cfg(feature = "desktop")]
use tauri_plugin_window_state::StateFlags;

#[cfg(feature = "desktop")]
struct FileLogger {
    file: parking_lot::Mutex<std::fs::File>,
}

#[cfg(feature = "desktop")]
impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        // whisper.cpp tells every step of a model's loading: only its warnings and errors.
        if m.target().starts_with("whisper_rs") {
            return m.level() <= log::Level::Warn;
        }
        m.level() <= log::Level::Info
            || (cfg!(debug_assertions) && m.target().starts_with("escouade"))
    }

    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let line = format!(
                "{} {:<5} {}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                r.level(),
                r.args()
            );
            let _ = self.file.lock().write_all(line.as_bytes());
            #[cfg(debug_assertions)]
            eprint!("{line}");
        }
    }

    fn flush(&self) {}
}

#[cfg(feature = "desktop")]
fn init_logging() {
    let data = paths::DataDir::new(paths::default_data_dir());
    let _ = data.ensure();
    let path = data.log_file();
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 5_000_000) {
        let _ = std::fs::rename(&path, path.with_extension("old.log"));
    }
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let logger = Box::leak(Box::new(FileLogger {
            file: parking_lot::Mutex::new(file),
        }));
        if log::set_logger(logger).is_ok() {
            log::set_max_level(log::LevelFilter::Debug);
        }
    }
}

/// The tray icon, its menu in the interface's language (written again when it changes:
/// `menus::relabel`).
#[cfg(feature = "desktop")]
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let lang = i18n::ui();
    let menu = menus::tray_menu(app.handle(), lang)?;
    let mut builder = TrayIconBuilder::with_id(menus::TRAY_ID)
        .tooltip(menus::tray_tooltip(lang, 0))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => notify::show_main(app),
            "reload" => navigation::reload(app),
            "quit" => {
                let core = app.state::<Arc<Core>>();
                if core.ask_before_quit() {
                    notify::show_main(app);
                } else {
                    updates::quit(app);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                notify::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// The window's theme for « Thème »: none is the system's.
#[cfg(feature = "desktop")]
fn window_theme(choice: model::ThemeChoice) -> Option<tauri::Theme> {
    match choice {
        model::ThemeChoice::System => None,
        model::ThemeChoice::Dark => Some(tauri::Theme::Dark),
        model::ThemeChoice::Light => Some(tauri::Theme::Light),
    }
}

/// The main window, made from the config's (`create: false` there: the guard of `navigation` can
/// only be put on it as it is made), hidden until `setup` (in `run`) shows it. `remote`: in a build of development, the
/// link of the window to the remote API (`remoteapi::window`), which its page then speaks to the core by.
#[cfg(feature = "desktop")]
fn build_window(
    app: &tauri::App,
    remote: Option<&remoteapi::window::WindowLink>,
) -> tauri::Result<()> {
    let config = app.config();
    let window = config
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .ok_or(tauri::Error::WindowNotFound)?;
    // Out of sight (`paths::hidden_window`), its WebView does not take the focus as it is made:
    // that would take it from the user's app, the window still hidden.
    let mut window = window.clone();
    window.focus &= !paths::hidden_window();
    // The size an end-to-end test gives it (a CI runner's screen): the zoom is held to the window's (ESC-440).
    if let Some((width, height)) = paths::window_size() {
        window.width = width;
        window.height = height;
    }
    // Built with Tauri’s `devtools` for the launches’ pages (ESC-311): the window’s own stay a
    // debug build’s.
    window.devtools = Some(cfg!(debug_assertions));
    // Only the dev server's address is the app's besides its own.
    let dev = if tauri::is_dev() {
        config.build.dev_url.clone()
    } else {
        None
    };
    app.manage(navigation::Home {
        url: navigation::home(dev.as_ref(), window.use_https_scheme),
        dev: dev.clone(),
    });
    let mut builder = WebviewWindowBuilder::from_config(app.handle(), &window)?;
    if let Some(link) = remote {
        let version = app.package_info().version.to_string();
        let origin = dev.as_ref().map(|u| u.origin().ascii_serialization());
        builder = builder.initialization_script(link.script(&version, origin.as_deref()));
    }
    // The WebView2 folder an end-to-end test gives it: by the API, which a CI runner (elevated) follows (ESC-496).
    if let Some(dir) = paths::webview_dir() {
        builder = builder.data_directory(dir);
    }
    let main = navigation::guard(builder, dev).build()?;
    // The paths of the files the page hands the window (a reload keeps the WebView, and the listener).
    let origins = fileorigin::FileOrigins::default();
    app.manage(origins.clone());
    fileorigin::install(&main, &origins);
    // A page whose process ends is loaded again, without what it showed (ESC-290).
    pagecrash::install(&main);
    Ok(())
}

/// `escouade-server` (src/bin): the core without a window, its remote API, its pairing (ESC-514). Gives the exit code.
pub fn server_main(args: Vec<String>) -> i32 {
    server::main(args)
}

/// `escouade-plugin` (src/bin): the conformance bench, the manifest's check and the fake Escouade,
/// for the plugins' authors (PLG-3). Gives the exit code.
pub fn plugin_cli(args: Vec<String>) -> i32 {
    plugins::bench::cli::main(args)
}

/// The app's context (its config and its pages), made once: the window's and the probes'.
#[cfg(feature = "desktop")]
fn context() -> tauri::Context {
    tauri::generate_context!()
}

/// `devpage_probe` (examples): a launch's page in a real webview of this system (ESC-311), what it
/// could reach of Escouade, its storage, its devtools and its widths. Gives the exit code.
#[cfg(feature = "desktop")]
pub fn devpage_probe_cli(args: Vec<String>) -> i32 {
    devpage_probe::cli(args, context())
}

#[cfg(feature = "desktop")]
pub fn run() {
    // A debug build told `--sandbox-keychain …` takes back the keychain's entries of sandboxed runs, then stops: the
    // end-to-end fixture's, at its start and its end (ESC-584). Before anything, which would touch the user's folders.
    #[cfg(debug_assertions)]
    if let Some((out, code)) = escouade_forge::integrations::sandbox_keychain::command(
        &std::env::args().skip(1).collect::<Vec<_>>(),
    ) {
        println!("{out}");
        std::process::exit(code);
    }
    // Before GTK starts: the window's app_id and WM_CLASS, which find its icon (ESC-239).
    #[cfg(target_os = "linux")]
    desktop::name_the_program();
    #[cfg(unix)]
    shellenv::adopt_login_path();
    // Started by a process whose PATH is older than the user's (ESC-186): today's, before any thread.
    #[cfg(windows)]
    syspath::refresh();
    paths::migrate_app_folders();
    // What the last stop for an update left: the window comes back as it was then.
    let after_update = updates::take_note(
        &paths::DataDir::new(paths::default_data_dir()),
        env!("CARGO_PKG_VERSION"),
    );
    let window = updates::AfterUpdate::window(&after_update);
    // The commands, away from the window's thread; what runs on their lanes, for its watch (ESC-290).
    let running = Arc::new(windowthread::Running::default());
    let lanes = windowthread::Lanes::new(running.clone());
    let watched = running.clone();
    let focused = running.clone();
    let mut builder = tauri::Builder::default();
    // Sandboxed runs (end-to-end tests, demos) must not hand over to an instance the user
    // already has open.
    // Nor take the size and place of the user's window, or save their own over it (the file is the
    // app's, not the data folder's): a sandboxed window opens at the size of tauri.conf.json, as
    // big as the screen allows, whatever window the user last had.
    if paths::sandbox_dir().is_none() {
        builder = builder
            // A second launch hands its link over to this one (`escouade://plugin/<id>`, STORE-11).
            .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
                notify::show_main(app);
                deeplink::deliver(app, deeplink::links_in(args));
            }))
            .plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                    .build(),
            );
    }
    let builder = builder
        // `escouade://`, registered by the installers (STORE-11); macOS hands a link over by an
        // Apple event, which this plugin receives.
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_plugin_notification::init());
    // Restarted by itself behind another app, it does not take the focus from it.
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(window == updates::Window::Front);
    // The files of the editor's extensions (ESC-481), read-only: what `vsx` installed and checked, nothing else.
    let builder =
        builder.register_uri_scheme_protocol(escouade_vsx::serve::SCHEME, |ctx, request| {
            let ext = ctx.app_handle().try_state::<Arc<vsx::Extensions>>();
            vsx::protocol(ext.as_deref().map(|e| e.as_ref()), &request)
        });
    builder
        .setup(move |app| {
            init_logging();
            windowthread::watch_app(app.handle().clone(), watched.clone());
            // The window on the remote API (ESC-495), in a build of development whose variables ask for it: its socket
            // bound before the window is made, which is told where it is.
            let window_link = remoteapi::window::bind();
            // First: nothing below may find the window missing.
            build_window(app, window_link.as_ref())?;
            for note in paths::take_migration_notes() {
                log::info!("migration: {note}");
            }
            log::info!("Escouade {} starting", app.package_info().version);
            if let Some(updates::AfterUpdate::Failed(version)) = &after_update {
                log::warn!("Escouade {version} did not install: the app still runs this version");
            }
            app.manage(deeplink::Inbox::default());
            // The pages of the launches (ESC-311).
            app.manage(Arc::new(devpage::Pages::default()));
            // The process groups of agents a killed launch left behind (macOS, Linux: ESC-417),
            // stopped before any agent starts; this launch's are noted from now on.
            agentgroups::start(
                paths::DataDir::new(paths::default_data_dir())
                    .root()
                    .join("agent-groups.json"),
            );
            let data = paths::DataDir::new(paths::default_data_dir());
            // The data folder held as a process without a window holds it (`headless`), which then
            // refuses to start on it; single-instance keeps a second desktop app away already.
            match headless::InstanceLock::acquire(data.root()) {
                Ok(lock) => {
                    app.manage(lock);
                }
                Err(e) => log::warn!("data folder not locked: {e:#}"),
            }
            let (core, git_rx) = Core::load(app.handle().clone(), data);
            app.manage(core.clone());
            // The teams of the team server (TS-3): their keys in the app's keychain, the network of the settings, the
            // data's `teams` folder; read every few seconds, for the invitees an online administrator lets in.
            let teams = {
                let settings = Arc::downgrade(&core);
                let hub = Arc::downgrade(&core);
                Arc::new(teams::Teams::open(
                    core.data.root().join("teams"),
                    core.secrets.clone(),
                    Arc::new(move || {
                        settings
                            .upgrade()
                            .map(|c| c.settings.read().clone())
                            .unwrap_or_default()
                    }),
                    Arc::new(move || {
                        if let Some(core) = hub.upgrade() {
                            core.hub.emit(model::UiEvent::Teams {});
                        }
                    }),
                ))
            };
            app.manage(teams.clone());
            // The Kanban of the teams' projects (TS-6): their journals, their claims.
            core.set_teams(teams.clone());
            {
                let core = Arc::downgrade(&core);
                teams.set_docs_sink(Arc::new(move |id| {
                    if let Some(core) = core.upgrade() {
                        let id = id.to_string();
                        tauri::async_runtime::spawn(async move {
                            core.team_docs_after_read(&id).await;
                        });
                    }
                }));
            }
            // The team's signed settings (TS-13): the version in force of each team to the core's locks and projects.
            {
                let core = Arc::downgrade(&core);
                teams.set_settings_sink(Arc::new(move |id, name, view| {
                    if let Some(core) = core.upgrade() {
                        core.apply_team_settings(id, name, view);
                    }
                }));
                let started = teams.clone();
                tauri::async_runtime::spawn(async move { started.start_settings().await });
            }
            tauri::async_runtime::spawn(teams.watch(teams::watch_every()));
            // The editor's language servers (ESC-446): their messages and states to the window.
            app.manage(lsp::for_app(app.handle().clone()));
            // The editor's extensions (ESC-481): Open VSX by the core, checked, served.
            app.manage(vsx::Extensions::new(core.data.root()));
            // The dictation: its events to the window, as the core's.
            let hub = std::sync::Arc::downgrade(&core);
            app.manage(Arc::new(voice::Voice::for_app(
                core.data.root().to_path_buf(),
                Arc::new(move |e| {
                    if let Some(core) = hub.upgrade() {
                        core.hub.emit(model::UiEvent::Voice { voice: e });
                    }
                }),
            )));
            app.manage(updates::Updates::started(after_update));
            // The servers this computer knows (ESC-537): the window goes back on the one it was on, its tunnel started.
            let servers = {
                let locks = Arc::downgrade(&core);
                servers::Servers::load(
                    core.data.root(),
                    core.secrets.clone(),
                    servers::tunnel::Ssh::system(),
                    app.package_info().version.to_string(),
                    Arc::new(move |host: &str| match locks.upgrade() {
                        Some(core) => core.locks.read().check_ssh_server(host),
                        None => Ok(()),
                    }),
                    servers::sshconfig::user_config(),
                )
            };
            servers.resume();
            app.manage(servers);
            if !paths::hidden_window() {
                build_tray(app)?;
            }
            // In place of Tauri's default, in English. Should it fail, the default stays: its Edit
            // menu keeps copy and paste working in the WebView.
            #[cfg(target_os = "macos")]
            if let Err(e) = menus::app_menu(app.handle(), i18n::ui()).and_then(|m| app.set_menu(m))
            {
                log::error!("app menu not set: {e}");
            }
            // The Overseer's global shortcut (ESC-468): the system's, through its plugin; recorded in a sandbox (the
            // end-to-end tests press it by `overseer_hotkey_press`); none where an app has none (Wayland), as the
            // settings say. A plugin that cannot start leaves the app without it, nothing else.
            if paths::sandbox_dir().is_some() {
                core.overseer
                    .hotkey
                    .set_keys(Arc::new(hotkey::Recorded::default()));
            } else if !hotkey::wayland() {
                let plugin = tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(|app, _shortcut, event| {
                        if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                            hotkey::pressed(app);
                        }
                    })
                    .build();
                match app.handle().plugin(plugin) {
                    Ok(()) => core
                        .overseer
                        .hotkey
                        .set_keys(Arc::new(hotkey::PluginKeys(app.handle().clone()))),
                    Err(e) => log::warn!("overseer: no global shortcut here: {e}"),
                }
            }
            core.start(git_rx);
            // The remote API (ESC-478): off, unless `ESCOUADE_REMOTE_API` gives its port, in a build of development
            // only (the tests, until the server mode has its setting: a variable opens nothing in a release, as
            // SEC-13 of the audit asks of the policies); the window is a client like any other of the same core.
            let listen = match window_link {
                // The window on another core's API (`escouade-server`, ESC-514): this one serves nothing.
                Some(link) => link.listener.map(remoteapi::server::Listen::Bound),
                None => remoteapi::server::port_asked().map(remoteapi::server::Listen::Port),
            };
            // The phone's tunnel (ESC-567): the API it serves, made when the access goes on, without a socket of its
            // own (served in the tunnel alone); the one of a build of development when there is one.
            {
                let factory_core = core.clone();
                let factory_app = app.handle().clone();
                core.phone.set_factory(Box::new(move || {
                    let services = Arc::new(remoteapi::Services::of_app(
                        factory_core.clone(),
                        factory_app.clone(),
                    ));
                    Box::pin(async move {
                        remoteapi::server::RemoteApi::start_with(
                            services,
                            remoteapi::server::Listen::Nothing,
                            remoteapi::server::Limits::default(),
                        )
                        .await
                    })
                }));
            }
            let has_listen = listen.is_some();
            if let Some(listen) = listen {
                let services = Arc::new(remoteapi::Services::of_app(
                    core.clone(),
                    app.handle().clone(),
                ));
                let handle = app.handle().clone();
                let phone_core = core.clone();
                tauri::async_runtime::spawn(async move {
                    let limits = remoteapi::server::Limits::default();
                    match remoteapi::server::RemoteApi::start_with(services, listen, limits).await {
                        Ok(api) => {
                            phone_core.phone.attach(&api);
                            handle.manage(api);
                        }
                        Err(e) => log::warn!("remote API not served: {e:#}"),
                    }
                    let _ = phone_core.phone.apply(&phone_core).await;
                });
            }
            if !has_listen {
                let phone_core = core.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = phone_core.phone.apply(&phone_core).await;
                });
            }
            // The Principals of settings saved before 1.8.1 kept or dropped, once (ESC-128).
            let migrating = core.clone();
            tauri::async_runtime::spawn(async move { migrating.migrate_accounts().await });
            // Finding a `claude` of WSL may start its distribution: done now and off the main
            // thread, which neither the window nor the board then waits for (`wsl::find` keeps it).
            let warm = core.clone();
            tauri::async_runtime::spawn_blocking(move || warm.warm_claude());
            // What the harness of each account lists of its models, without a turn (ESC-124, ESC-186).
            core.read_all_account_models();
            // Claude Code updated by Escouade, apart from its agents (ESC-133).
            selfupdate::watch(core.clone());
            updates::watch(app.handle().clone());
            if let Some(w) = app.get_webview_window("main") {
                // « Zoom de l’interface », before the window shows: never drawn at 100 % first, nor
                // at a zoom it is too small for (the window holds it to its size from then on).
                let saved = core.settings.read().ui_zoom;
                let scale = w.scale_factor().unwrap_or(1.0);
                let zoom = match w.inner_size() {
                    Ok(s) if s.width > 0 && s.height > 0 => model::fit_zoom(
                        saved,
                        f64::from(s.width) / scale,
                        f64::from(s.height) / scale,
                    ),
                    _ => saved,
                };
                if let Err(e) = w.set_zoom(f64::from(zoom) / 100.0) {
                    log::warn!("zoom not set: {e}");
                }
                // « Thème », before the window shows: born in it (the title bar of macOS, the
                // corners and shadow of Windows 11, what shows before the page); the page follows
                // its changes (lib/appearance.ts).
                if let Err(e) = w.set_theme(window_theme(core.settings.read().theme)) {
                    log::warn!("theme not set: {e}");
                }
                if w.theme().is_ok_and(|t| t == tauri::Theme::Light) {
                    // The light frame, where the config has the dark one.
                    let _ =
                        w.set_background_color(Some(tauri::window::Color(0xf0, 0xf1, 0xf3, 0xff)));
                }
                match window {
                    _ if paths::hidden_window() => notify::show_out_of_sight(&w),
                    updates::Window::Front => {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                    updates::Window::Behind => {
                        let _ = w.show();
                    }
                    // Minimized when the app restarted for an update: minimized again, its button
                    // in the taskbar (hidden, it would only be found in the tray).
                    updates::Window::Minimized => {
                        let _ = w.show();
                        let _ = w.minimize();
                    }
                    // Closed to the tray when the app restarted for an update: it stays there.
                    updates::Window::Hidden => {}
                }
            }
            deeplink::at_start(app.handle());
            Ok(())
        })
        .on_window_event(move |window, event| {
            // Back to the window: a harness installed meanwhile (in a terminal) gets Escouade's server.
            if let WindowEvent::Focused(true) = event {
                if let Some(core) = window.try_state::<Arc<Core>>() {
                    let _running = focused.enter("window focused");
                    core.declare_on_focus();
                    // A repository added or removed in a project's folder meanwhile (ESC-169).
                    core.refresh_all_repos();
                    core.refresh_all_workflows();
                    // The pull requests of the tickets « En revue », read again at once (ESC-59).
                    core.window_front(true);
                }
            }
            if let WindowEvent::Focused(false) = event {
                if let Some(core) = window.try_state::<Arc<Core>>() {
                    core.window_front(false);
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closed to the tray: no one sees the dictation any more, it stops.
                if let Some(v) = window.try_state::<Arc<voice::Voice>>() {
                    v.cancel();
                }
                let core = window.state::<Arc<Core>>();
                // A space's own window (ESC-539) closes: the main window alone goes to the tray.
                if !core.quitting.load(Ordering::Acquire)
                    && !servers::window::is_space(window.label())
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(windowthread::off_the_window(
            lanes,
            // The single list of the commands, for the IPC and the remote API (`remoteapi::list`).
            remoteapi::list::ipc(),
        ))
        .build({
            let mut context = context();
            // The window on the remote API reaches the WebSockets of the loopback (ESC-495): in that case alone.
            if remoteapi::window::wanted() {
                remoteapi::window::open_csp(&mut context);
            }
            context
        })
        .expect("error while building the application")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => {
                // The tunnel to a server ends with the app (ESC-537).
                if let Some(servers) = app.try_state::<Arc<servers::Servers>>() {
                    servers.shutdown();
                }
                // The language servers end with the app, with what they started (their Jobs).
                if let Some(servers) = app.try_state::<Arc<lsp::Manager>>() {
                    servers.shutdown();
                }
                if let Some(core) = app.try_state::<Arc<Core>>() {
                    if !core.quitting.load(Ordering::Acquire) {
                        // macOS: Cmd+Q, the app menu's « Quitter Escouade » (`menus::app_menu`)
                        // and the Dock's « Quitter » come here straight, with no exit request
                        // first: the update ready installs now.
                        #[cfg(target_os = "macos")]
                        {
                            if updates::install_at_exit(app) {
                                return;
                            }
                        }
                        core.shutdown();
                    }
                }
            }
            // A click on the Dock icon brings back the window closed to the menu bar.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => notify::show_main(app),
            _ => {}
        });
}
