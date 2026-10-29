// First: its macros (`tr!`, `tr_in!`) are then in scope in every module below.
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
mod attach;
mod attachfiles;
#[cfg(test)]
mod attachfiles_tests;
mod board;
#[cfg(test)]
mod branches_tests;
mod clipboard;
mod closed;
#[cfg(test)]
mod closed_tests;
mod codeindex;
#[cfg(test)]
mod codeindex_tests;
#[cfg(test)]
mod codex_tests;
#[cfg(test)]
mod codex_tui_tests;
mod commands;
mod conv;
mod convsearch;
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
mod desktop;
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
mod fence;
#[cfg(test)]
mod fence_tests;
mod fileorigin;
#[cfg(test)]
mod fileorigin_tests;
#[cfg(test)]
mod first_message_tests;
mod forge;
mod fsedit;
#[cfg(test)]
mod gemini_tui_tests;
mod git;
mod handover;
mod hub;
mod integrations;
#[cfg(test)]
mod integrations_tests;
mod isola;
mod job;
mod leftovers;
mod mcp;
mod memory;
mod menus;
mod model;
#[cfg(test)]
mod multi_repo_tests;
mod multiticket;
mod multiwt;
mod navigation;
mod notify;
#[cfg(test)]
mod opencode_tests;
mod outside;
mod pagecrash;
mod paths;
mod plan;
#[cfg(test)]
mod plan_later_tests;
#[cfg(test)]
mod plan_tests;
mod planfiles;
#[cfg(test)]
mod planfiles_tests;
mod plugins;
#[cfg(test)]
mod plugins_bench_tests;
#[cfg(test)]
mod plugins_runtime_tests;
#[cfg(test)]
mod plugins_tests;
mod pricing;
#[cfg(test)]
mod private_tests;
mod proc;
#[cfg(test)]
mod process_tests;
mod projectfile;
#[cfg(test)]
mod projectfile_tests;
mod projectlogo;
mod pty;
mod repos;
#[cfg(test)]
mod repos_tests;
mod resources;
mod search;
mod selfupdate;
mod sentfiles;
#[cfg(test)]
mod sentfiles_tests;
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
mod telemetry;
#[cfg(test)]
mod terminal_tests;
#[cfg(test)]
mod test_tmp;
mod testlaunch;
mod text;
mod ticketfiles;
mod ticketimages;
mod tickets;
#[cfg(test)]
mod tickets_tests;
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
mod which;
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
mod wsl;

use crate::core::Core;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_window_state::StateFlags;

struct FileLogger {
    file: parking_lot::Mutex<std::fs::File>,
}

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
fn window_theme(choice: model::ThemeChoice) -> Option<tauri::Theme> {
    match choice {
        model::ThemeChoice::System => None,
        model::ThemeChoice::Dark => Some(tauri::Theme::Dark),
        model::ThemeChoice::Light => Some(tauri::Theme::Light),
    }
}

/// The main window, made from the config's (`create: false` there: the guard of `navigation` can
/// only be put on it as it is made), hidden until `setup` (in `run`) shows it.
fn build_window(app: &tauri::App) -> tauri::Result<()> {
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
    let main = navigation::guard(
        WebviewWindowBuilder::from_config(app.handle(), &window)?,
        dev,
    )
    .build()?;
    // The paths of the files the page hands the window (a reload keeps the WebView, and the listener).
    let origins = fileorigin::FileOrigins::default();
    app.manage(origins.clone());
    fileorigin::install(&main, &origins);
    // A page whose process ends is loaded again, without what it showed (ESC-290).
    pagecrash::install(&main);
    Ok(())
}

/// `escouade-plugin` (src/bin): the conformance bench, the manifest's check and the fake Escouade,
/// for the plugins' authors (PLG-3). Gives the exit code.
pub fn plugin_cli(args: Vec<String>) -> i32 {
    plugins::bench::cli::main(args)
}

pub fn run() {
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
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                notify::show_main(app)
            }))
            .plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                    .build(),
            );
    }
    let builder = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_plugin_notification::init());
    // Restarted by itself behind another app, it does not take the focus from it.
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(window == updates::Window::Front);
    builder
        .setup(move |app| {
            init_logging();
            windowthread::watch_app(app.handle().clone(), watched.clone());
            // First: nothing below may find the window missing.
            build_window(app)?;
            for note in paths::take_migration_notes() {
                log::info!("migration: {note}");
            }
            log::info!("Escouade {} starting", app.package_info().version);
            if let Some(updates::AfterUpdate::Failed(version)) = &after_update {
                log::warn!("Escouade {version} did not install: the app still runs this version");
            }
            let (core, git_rx) = Core::load(
                app.handle().clone(),
                paths::DataDir::new(paths::default_data_dir()),
            );
            app.manage(core.clone());
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
            core.start(git_rx);
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
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closed to the tray: no one sees the dictation any more, it stops.
                if let Some(v) = window.try_state::<Arc<voice::Voice>>() {
                    v.cancel();
                }
                let core = window.state::<Arc<Core>>();
                if !core.quitting.load(Ordering::Acquire) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(windowthread::off_the_window(
            lanes,
            tauri::generate_handler![
                commands::subscribe,
                commands::set_ui,
                commands::open_url,
                commands::open_path,
                commands::save_settings,
                commands::claude_locate,
                commands::claude_update_status,
                commands::data_dir,
                commands::plugins_list,
                commands::plugin_install_folder,
                commands::plugin_install_git,
                commands::plugin_load_dev,
                commands::plugin_remove,
                commands::account_sign_in_ended,
                commands::claude_found,
                commands::engines,
                commands::account_shareable,
                commands::account_create,
                commands::account_add_machine,
                commands::account_machine,
                commands::account_update,
                commands::account_reorder,
                commands::engine_order_set,
                commands::favorite_model_set,
                commands::account_remove,
                commands::account_status,
                commands::account_models_refresh,
                commands::engines_found_now,
                commands::account_key,
                commands::account_login,
                commands::set_agent_view,
                commands::agent_terminal,
                commands::tui_asks_trust,
                commands::paste_image,
                commands::inspect_folder,
                commands::refresh_repos,
                commands::docs_spec_tree,
                commands::docs_compare,
                commands::docs_detect_spec,
                commands::docs_spec_page,
                commands::docs_spec_write,
                commands::docs_spec_search,
                commands::docs_decisions,
                commands::docs_decision_propose,
                commands::docs_decision_replace,
                commands::docs_decision_accept,
                commands::docs_documents,
                commands::docs_document_text,
                commands::docs_document_add,
                commands::docs_drive,
                commands::docs_document_check,
                commands::docs_document_move,
                commands::docs_folder_add,
                commands::docs_documents_search,
                commands::docs_pending,
                commands::docs_share,
                commands::docs_share_resolve,
                commands::docs_share_cancel,
                commands::docs_import_plan,
                commands::docs_import,
                commands::docs_recent,
                commands::discussion_start,
                commands::discussion_pour,
                commands::discussion_drop,
                commands::docs_plan,
                commands::docs_plan_cancel,
                commands::get_workflows,
                commands::get_workflow_tracks,
                commands::refresh_workflows,
                commands::workflow_refine_plan,
                commands::refine_workflows,
                commands::effective_workflows,
                commands::remove_workflow_file,
                commands::agent_merge_check,
                commands::merge_agent_repos,
                commands::forget_repo,
                commands::inspect_setup,
                commands::first_commit,
                commands::create_project,
                commands::update_project,
                commands::set_project_logo,
                commands::get_code_index,
                commands::set_code_index,
                commands::get_index_savings,
                commands::vectors_model,
                commands::vectors_model_download,
                commands::vectors_model_cancel,
                commands::vectors_model_delete,
                commands::project_logo,
                commands::read_logo,
                commands::shared_trust,
                commands::memory_list,
                commands::memory_forget,
                commands::claude_memory_offer,
                commands::claude_memory_take,
                commands::shared_write,
                commands::shared_unkeep,
                commands::shared_preview,
                commands::reorder_projects,
                commands::remove_project,
                commands::closed_projects,
                commands::erase_closed_project,
                commands::create_agent,
                commands::warm_agent,
                commands::restart_agent_process,
                commands::get_conversation,
                commands::search_conversations,
                commands::send_message,
                commands::attach_open,
                commands::attach_chunk,
                commands::attach_finish,
                commands::attach_cancel,
                commands::attach_discard,
                commands::attach_origin,
                fileorigin::file_origin,
                commands::clipboard_image,
                commands::interrupt,
                commands::stop_subagent,
                commands::stop_bg_task,
                commands::task_output,
                commands::subagent_transcript,
                commands::message_subagent,
                commands::answer_question,
                commands::answer_permission,
                commands::set_agent_options,
                commands::set_agent_account,
                commands::resume_on_account,
                commands::continue_on_engine,
                commands::back_to_previous_account,
                commands::rename_agent,
                commands::duplicate_agent,
                commands::archive_agent,
                commands::delete_agent,
                commands::merge_agent,
                commands::get_commands,
                commands::file_suggestions,
                commands::git_files,
                commands::git_diff,
                commands::git_project_diff,
                commands::git_log,
                commands::git_show,
                commands::git_fetch,
                commands::git_pull,
                commands::git_push,
                commands::branch_list,
                commands::branch_check,
                commands::branch_switch,
                commands::branch_create,
                commands::branch_delete,
                commands::branches_merged,
                commands::git_diff_refs,
                commands::create_agent_on_branch,
                commands::integrate_base,
                commands::agent_pr_propose,
                commands::agent_pr_open,
                commands::agent_prs_refresh,
                commands::agent_branch_switch,
                commands::agent_git_sync,
                commands::agent_pr_fix,
                commands::set_remote_control,
                commands::stats,
                commands::refresh_usage,
                commands::cancel_resume,
                commands::git_discard,
                commands::commit_preview,
                commands::commit_propose,
                commands::commit_direct,
                commands::fs_tree,
                commands::fs_read,
                commands::fs_write,
                commands::fs_create,
                commands::fs_rename,
                commands::fs_delete,
                commands::fs_reveal,
                commands::sent_files,
                commands::sent_file_open,
                commands::agent_local_image,
                commands::remote_image,
                commands::fs_mkdir,
                commands::fs_base,
                commands::code_search,
                commands::term_spawn,
                commands::run_start,
                commands::term_write,
                commands::term_resize,
                commands::term_kill,
                commands::term_report,
                commands::play_chime,
                commands::quit_app,
                commands::set_unsaved,
                commands::ticket_create,
                commands::ticket_update,
                commands::ticket_delete,
                commands::clean_worktrees,
                commands::ticket_images,
                commands::ticket_prioritize,
                commands::ticket_share,
                commands::ticket_hide,
                commands::shared_resolve,
                commands::git_user_name,
                commands::ticket_start,
                commands::ticket_resume,
                commands::ticket_approve,
                commands::ticket_reject,
                commands::ticket_resolve_conflict,
                commands::ticket_dismiss,
                commands::board_set,
                commands::autopilot_resume,
                commands::forges,
                commands::forge_check,
                commands::forge_connect,
                commands::forge_disconnect,
                commands::forge_guess,
                commands::forge_token_page,
                commands::integration_connect,
                commands::integration_disconnect,
                commands::integration_containers,
                commands::integration_states,
                commands::integration_issues,
                commands::integration_import,
                commands::integration_resync,
                commands::git_branches,
                commands::agent_prepare_launch,
                commands::test_recipe_approve,
                commands::test_run_start,
                commands::http_ready,
                commands::update_check,
                commands::update_download,
                commands::update_restart,
                commands::update_postpone,
                commands::update_presence,
                commands::update_close,
                commands::isola_config,
                commands::isola_approve,
                commands::isola_services,
                commands::isola_down,
                commands::suggest_worktree_steps,
                commands::suggest_run_commands,
                commands::suggest_worktree_steps_at,
                commands::suggest_run_commands_at,
                commands::mcp_status,
                commands::mcp_activity,
                commands::mcp_clear_activity,
                commands::mcp_set_enabled,
                commands::telemetry_set,
                commands::mcp_declare_status,
                commands::mcp_manual_command,
                commands::voice_info,
                commands::voice_start,
                commands::voice_stop,
                commands::voice_cancel,
                commands::voice_download,
                commands::voice_download_cancel,
                commands::voice_delete_model,
                commands::voice_online_keys,
                commands::voice_online_key_set,
                commands::voice_online_key_delete,
                commands::voice_online_catalog,
                commands::speech_info,
                commands::conversation_start,
                commands::conversation_listen,
                commands::conversation_hold,
                commands::conversation_say,
                commands::conversation_stop,
                commands::conversation_send,
                commands::speech_download,
                commands::speech_download_cancel,
                commands::speech_delete,
                commands::speech_preview,
                commands::speech_preview_online,
            ],
        ))
        .build(tauri::generate_context!())
        .expect("error while building the application")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => {
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
