//! The app's native menus, in the interface's language: the tray icon's (Windows and macOS) and,
//! on macOS, the app menu in the menu bar. Built at startup (`run`), and again when the language
//! changes (`relabel`).

use crate::i18n::Lang;
use tauri::menu::{
    AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID, WINDOW_SUBMENU_ID,
};
use tauri::{AppHandle, Runtime};

/// The tray icon's id.
pub const TRAY_ID: &str = "main";
/// The id of the app menu `app_menu` makes: another one (Tauri's default, in tests) is left as it
/// is by `relabel`.
pub const APP_MENU_ID: &str = "escouade-app-menu";

/// The entries of the tray icon's menu.
#[derive(Debug, PartialEq, Eq)]
pub struct TrayLabels {
    pub show: String,
    pub quit: String,
}

pub fn tray_labels(lang: Lang) -> TrayLabels {
    TrayLabels {
        show: tr_in!(lang, "Afficher", "Show"),
        quit: tr_in!(lang, "Quitter", "Quit"),
    }
}

/// The tray icon's tooltip, with the agents waiting for an answer.
pub fn tray_tooltip(lang: Lang, waiting: usize) -> String {
    match waiting {
        0 => "Escouade".into(),
        1 => tr_in!(
            lang,
            "Escouade — 1 agent en attente",
            "Escouade — 1 agent waiting"
        ),
        n => tr_in!(
            lang,
            "Escouade — {n} agents en attente",
            "Escouade — {n} agents waiting"
        ),
    }
}

/// The tray icon's menu: « Afficher » and « Quitter », which its `on_menu_event` handles by id.
pub fn tray_menu<R: Runtime>(app: &AppHandle<R>, lang: Lang) -> tauri::Result<Menu<R>> {
    let labels = tray_labels(lang);
    let show = MenuItem::with_id(app, "show", labels.show, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

/// The entries of macOS's app menu, written as macOS writes them in each language (its own apps'
/// menus sit beside).
#[derive(Debug, PartialEq, Eq)]
pub struct AppMenuLabels {
    pub about: String,
    pub services: String,
    pub hide: String,
    pub hide_others: String,
    pub show_all: String,
    pub quit: String,
    pub edit: String,
    pub undo: String,
    pub redo: String,
    pub cut: String,
    pub copy: String,
    pub paste: String,
    pub select_all: String,
    pub view: String,
    pub fullscreen: String,
    pub window: String,
    pub minimize: String,
    pub zoom: String,
    pub close: String,
    pub help: String,
}

pub fn app_menu_labels(lang: Lang) -> AppMenuLabels {
    AppMenuLabels {
        about: tr_in!(lang, "À propos d’Escouade", "About Escouade"),
        services: tr_in!(lang, "Services", "Services"),
        hide: tr_in!(lang, "Masquer Escouade", "Hide Escouade"),
        hide_others: tr_in!(lang, "Masquer les autres", "Hide Others"),
        show_all: tr_in!(lang, "Tout afficher", "Show All"),
        quit: tr_in!(lang, "Quitter Escouade", "Quit Escouade"),
        edit: tr_in!(lang, "Édition", "Edit"),
        undo: tr_in!(lang, "Annuler", "Undo"),
        redo: tr_in!(lang, "Rétablir", "Redo"),
        cut: tr_in!(lang, "Couper", "Cut"),
        copy: tr_in!(lang, "Copier", "Copy"),
        paste: tr_in!(lang, "Coller", "Paste"),
        select_all: tr_in!(lang, "Tout sélectionner", "Select All"),
        view: tr_in!(lang, "Présentation", "View"),
        // macOS writes it again as it toggles (« Quitter le mode plein écran »).
        fullscreen: tr_in!(lang, "Passer en mode plein écran", "Enter Full Screen"),
        window: tr_in!(lang, "Fenêtre", "Window"),
        minimize: tr_in!(lang, "Placer dans le Dock", "Minimize"),
        zoom: tr_in!(lang, "Réduire/Agrandir", "Zoom"),
        close: tr_in!(lang, "Fermer", "Close"),
        help: tr_in!(lang, "Aide", "Help"),
    }
}

/// macOS's app menu, set at startup in place of Tauri's default (written in English), with the
/// same menus: the app's, Edit (without it, the WebView's fields would neither copy nor paste),
/// View, Window and Help. Its entries are macOS's own (`PredefinedMenuItem`): « Quitter Escouade »
/// ends the app as Cmd+Q does, through `RunEvent::Exit` (see `run`). No File menu: Window's
/// « Fermer » keeps Cmd+W. Only macOS sets it; it builds everywhere.
///
/// What macOS adds to it by itself (« Démarrer la dictée », « Emoji et symboles », the list of the
/// windows, the about panel's texts) is in the system's language among those the bundle declares
/// (`CFBundleLocalizations` in `src-tauri/Info.plist`, which Tauri merges): it follows the system,
/// not the app's setting.
pub fn app_menu<R: Runtime>(app: &AppHandle<R>, lang: Lang) -> tauri::Result<Menu<R>> {
    let l = app_menu_labels(lang);
    let package = app.package_info();
    let bundle = &app.config().bundle;
    // What Tauri's default menu shows in the about panel.
    let about = AboutMetadata {
        name: Some(package.name.clone()),
        version: Some(package.version.to_string()),
        copyright: bundle.copyright.clone(),
        authors: bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };
    let own = Submenu::with_items(
        app,
        &package.name,
        true,
        &[
            &PredefinedMenuItem::about(app, Some(l.about.as_str()), Some(about))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, Some(l.services.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(l.hide.as_str()))?,
            &PredefinedMenuItem::hide_others(app, Some(l.hide_others.as_str()))?,
            &PredefinedMenuItem::show_all(app, Some(l.show_all.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(l.quit.as_str()))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        &l.edit,
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(l.undo.as_str()))?,
            &PredefinedMenuItem::redo(app, Some(l.redo.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(l.cut.as_str()))?,
            &PredefinedMenuItem::copy(app, Some(l.copy.as_str()))?,
            &PredefinedMenuItem::paste(app, Some(l.paste.as_str()))?,
            &PredefinedMenuItem::select_all(app, Some(l.select_all.as_str()))?,
        ],
    )?;
    // Its id makes it macOS's Window menu, which lists the windows open.
    let window = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        &l.window,
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(l.minimize.as_str()))?,
            &PredefinedMenuItem::maximize(app, Some(l.zoom.as_str()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(l.close.as_str()))?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        &l.view,
        true,
        &[&PredefinedMenuItem::fullscreen(
            app,
            Some(l.fullscreen.as_str()),
        )?],
    )?;
    // Its id makes it macOS's Help menu, which holds the search through the menus.
    let help = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, &l.help, true, &[])?;
    Menu::with_id_and_items(app, APP_MENU_ID, &[&own, &edit, &view, &window, &help])
}

/// The native menus written again in `lang` after a change of language: the tray icon's menu and
/// tooltip (`waiting` agents waiting for an answer), and the app menu when it is the app's own
/// (macOS). Only those the app set: none in tests.
pub fn relabel<R: Runtime>(app: &AppHandle<R>, lang: Lang, waiting: usize) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        // Same ids: the handler the tray icon was built with handles the new entries.
        if let Err(e) = tray_menu(app, lang).and_then(|menu| tray.set_menu(Some(menu))) {
            log::warn!("tray menu not written again: {e}");
        }
        if let Err(e) = tray.set_tooltip(Some(tray_tooltip(lang, waiting))) {
            log::warn!("tray tooltip not written again: {e}");
        }
    }
    if app.menu().is_some_and(|m| m.id().as_ref() == APP_MENU_ID) {
        if let Err(e) = app_menu(app, lang).and_then(|menu| app.set_menu(menu)) {
            log::warn!("app menu not written again: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Lang::{En, Fr};

    #[test]
    fn the_tray_menu_and_tooltip_are_written_in_the_language_of_the_interface() {
        let labels = |l: TrayLabels| (l.show, l.quit);
        assert_eq!(
            labels(tray_labels(Fr)),
            ("Afficher".into(), "Quitter".into())
        );
        assert_eq!(labels(tray_labels(En)), ("Show".into(), "Quit".into()));
        for lang in [Fr, En] {
            assert_eq!(tray_tooltip(lang, 0), "Escouade");
        }
        assert_eq!(tray_tooltip(Fr, 1), "Escouade — 1 agent en attente");
        assert_eq!(tray_tooltip(Fr, 3), "Escouade — 3 agents en attente");
        assert_eq!(tray_tooltip(En, 1), "Escouade — 1 agent waiting");
        assert_eq!(tray_tooltip(En, 3), "Escouade — 3 agents waiting");
    }

    #[test]
    fn the_app_menu_of_macos_reads_as_macos_writes_it_in_each_language() {
        let all = |l: AppMenuLabels| {
            [
                l.about,
                l.services,
                l.hide,
                l.hide_others,
                l.show_all,
                l.quit,
                l.edit,
                l.undo,
                l.redo,
                l.cut,
                l.copy,
                l.paste,
                l.select_all,
                l.view,
                l.fullscreen,
                l.window,
                l.minimize,
                l.zoom,
                l.close,
                l.help,
            ]
        };
        assert_eq!(
            all(app_menu_labels(Fr)),
            [
                "À propos d’Escouade",
                "Services",
                "Masquer Escouade",
                "Masquer les autres",
                "Tout afficher",
                "Quitter Escouade",
                "Édition",
                "Annuler",
                "Rétablir",
                "Couper",
                "Copier",
                "Coller",
                "Tout sélectionner",
                "Présentation",
                "Passer en mode plein écran",
                "Fenêtre",
                "Placer dans le Dock",
                "Réduire/Agrandir",
                "Fermer",
                "Aide",
            ]
        );
        assert_eq!(
            all(app_menu_labels(En)),
            [
                "About Escouade",
                "Services",
                "Hide Escouade",
                "Hide Others",
                "Show All",
                "Quit Escouade",
                "Edit",
                "Undo",
                "Redo",
                "Cut",
                "Copy",
                "Paste",
                "Select All",
                "View",
                "Enter Full Screen",
                "Window",
                "Minimize",
                "Zoom",
                "Close",
                "Help",
            ]
        );
    }

    #[test]
    fn a_menu_is_made_again_only_when_the_app_set_it() {
        // The mock sets neither a tray icon nor an app menu: nothing is made.
        let app = tauri::test::mock_app();
        relabel(app.handle(), En, 2);
        assert!(app.menu().is_none());
        assert!(app.tray_by_id(TRAY_ID).is_none());
    }

    // muda makes macOS's menus on the main thread only, and tests run on others: the menus are made
    // and read here on the other systems, from the same code.

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_app_menu_is_made_of_the_entries_of_macos_in_the_language_asked_for() {
        let app = tauri::test::mock_app();
        let menu = app_menu(app.handle(), Fr).unwrap();
        assert_eq!(menu.id().as_ref(), APP_MENU_ID);
        let texts = |name: &str, entries: &[&str]| {
            (
                name.to_string(),
                entries.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            submenus(&menu),
            [
                texts(
                    &app.package_info().name,
                    &[
                        "À propos d’Escouade",
                        "",
                        "Services",
                        "",
                        "Masquer Escouade",
                        "Masquer les autres",
                        "Tout afficher",
                        "",
                        "Quitter Escouade",
                    ]
                ),
                texts(
                    "Édition",
                    &[
                        "Annuler",
                        "Rétablir",
                        "",
                        "Couper",
                        "Copier",
                        "Coller",
                        "Tout sélectionner",
                    ]
                ),
                texts("Présentation", &["Passer en mode plein écran"]),
                texts(
                    "Fenêtre",
                    &["Placer dans le Dock", "Réduire/Agrandir", "", "Fermer"]
                ),
                // Empty: macOS puts its search field in it.
                texts("Aide", &[]),
            ]
        );
        // macOS's Window menu, which lists the windows open, and its Help menu.
        let ids: Vec<String> = menu
            .items()
            .unwrap()
            .iter()
            .map(|item| item.id().as_ref().to_string())
            .collect();
        assert_eq!(
            (ids[3].as_str(), ids[4].as_str()),
            (WINDOW_SUBMENU_ID, HELP_SUBMENU_ID)
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_app_menu_is_made_again_in_the_new_language_and_another_is_left_alone() {
        let app = tauri::test::mock_app();
        app.set_menu(app_menu(app.handle(), Fr).unwrap()).unwrap();
        relabel(app.handle(), En, 0);
        let menu = app.menu().unwrap();
        assert_eq!(menu.id().as_ref(), APP_MENU_ID);
        let names: Vec<String> = submenus(&menu).into_iter().map(|(name, _)| name).collect();
        assert_eq!(names[1..], ["Edit", "View", "Window", "Help"]);
        assert_eq!(submenus(&menu)[1].1[0], "Undo");
        // One the app did not make stays as it is.
        let other = Menu::with_items(
            app.handle(),
            &[&Submenu::new(app.handle(), "Autre", true).unwrap()],
        )
        .unwrap();
        let id = other.id().clone();
        app.set_menu(other).unwrap();
        relabel(app.handle(), Fr, 0);
        assert_eq!(app.menu().unwrap().id(), &id);
    }

    /// Each submenu of `menu`, with the texts of its entries: macOS's own, a separator reading as
    /// an empty one.
    #[cfg(not(target_os = "macos"))]
    fn submenus(menu: &Menu<tauri::test::MockRuntime>) -> Vec<(String, Vec<String>)> {
        let entry = |item: &tauri::menu::MenuItemKind<tauri::test::MockRuntime>| {
            item.as_predefined_menuitem()
                .expect("an entry of macOS's own")
                .text()
                .unwrap()
        };
        menu.items()
            .unwrap()
            .iter()
            .map(|item| {
                let sub = item.as_submenu().expect("a submenu");
                (
                    sub.text().unwrap(),
                    sub.items().unwrap().iter().map(entry).collect(),
                )
            })
            .collect()
    }
}
