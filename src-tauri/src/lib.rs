// First: its macros (`tr!`, `tr_claude!`, `tr_in!`) are then in scope in every module below.
#[macro_use]
mod i18n;
mod accounts;
mod agent;
mod board;
mod claude;
mod commands;
mod conv;
mod convsearch;
mod core;
#[cfg(test)]
mod core_tests;
mod fsedit;
mod git;
mod hub;
mod integrations;
#[cfg(test)]
mod integrations_tests;
mod isola;
mod job;
mod menus;
mod model;
mod notify;
mod paths;
mod pricing;
#[cfg(test)]
mod process_tests;
mod pty;
mod resources;
mod search;
#[cfg(unix)]
mod shellenv;
mod stats;
mod testlaunch;
mod tickets;
#[cfg(test)]
mod tickets_tests;
mod updates;
mod usage;
mod which;
mod worktrees;

use crate::core::Core;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::StateFlags;

struct FileLogger {
    file: parking_lot::Mutex<std::fs::File>,
}

impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
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

pub fn run() {
    #[cfg(unix)]
    shellenv::adopt_login_path();
    paths::migrate_app_folders();
    // What the last stop for an update left: the window comes back as it was then.
    let after_update = updates::take_note(
        &paths::DataDir::new(paths::default_data_dir()),
        env!("CARGO_PKG_VERSION"),
    );
    let window = updates::AfterUpdate::window(&after_update);
    let mut builder = tauri::Builder::default();
    // Sandboxed runs (end-to-end tests, demos) must not hand over to an instance the user
    // already has open.
    if paths::sandbox_dir().is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            notify::show_main(app)
        }));
    }
    let builder = builder
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
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
            app.manage(updates::Updates::started(after_update));
            build_tray(app)?;
            // In place of Tauri's default, in English. Should it fail, the default stays: its Edit
            // menu keeps copy and paste working in the WebView.
            #[cfg(target_os = "macos")]
            if let Err(e) = menus::app_menu(app.handle(), i18n::ui()).and_then(|m| app.set_menu(m))
            {
                log::error!("app menu not set: {e}");
            }
            core.start(git_rx);
            updates::watch(app.handle().clone());
            if let Some(w) = app.get_webview_window("main") {
                match window {
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
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let core = window.state::<Arc<Core>>();
                if !core.quitting.load(Ordering::Acquire) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe,
            commands::set_ui,
            commands::save_settings,
            commands::inspect_folder,
            commands::create_project,
            commands::update_project,
            commands::reorder_projects,
            commands::remove_project,
            commands::create_agent,
            commands::warm_agent,
            commands::get_conversation,
            commands::search_conversations,
            commands::send_message,
            commands::interrupt,
            commands::answer_question,
            commands::answer_permission,
            commands::set_agent_options,
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
            commands::fs_mkdir,
            commands::fs_base,
            commands::code_search,
            commands::term_spawn,
            commands::run_start,
            commands::term_write,
            commands::term_resize,
            commands::term_kill,
            commands::play_chime,
            commands::quit_app,
            commands::set_unsaved,
            commands::ticket_create,
            commands::ticket_update,
            commands::ticket_delete,
            commands::ticket_prioritize,
            commands::ticket_start,
            commands::ticket_resume,
            commands::ticket_approve,
            commands::ticket_reject,
            commands::ticket_resolve_conflict,
            commands::ticket_dismiss,
            commands::board_set,
            commands::autopilot_resume,
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
        ])
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
