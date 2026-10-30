//! The name Linux desktops know Escouade's window by (ESC-239).
//!
//! A Wayland window has no icon of its own: WSLg, GNOME and KDE find it in the `.desktop` file
//! whose name is the window's app_id, to the letter (WSLg compares them as they are). GTK takes
//! that app_id from the program's name, the binary's (`escouade`) unless set before it starts,
//! while the .deb and the AppImage install `Escouade.desktop` (`<productName>.desktop`): Tux,
//! then. In X11 the same name makes the window's WM_CLASS, which the `.desktop`'s
//! `StartupWMClass` (`linux/escouade.desktop`, the packages' template) gives back.

/// The window's app_id (Wayland) and WM_CLASS (X11): `productName` in `tauri.conf.json`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub const APP_ID: &str = "Escouade";

/// Names the program after the `.desktop` file, before GTK starts.
#[cfg(target_os = "linux")]
pub fn name_the_program() {
    gtk::glib::set_prgname(Some(APP_ID));
}

#[cfg(test)]
mod tests {
    use super::APP_ID;
    use std::path::Path;

    fn read(file: &str) -> String {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap()
    }

    fn json(file: &str) -> serde_json::Value {
        serde_json::from_str(&read(file)).unwrap()
    }

    #[test]
    fn the_window_is_named_after_the_desktop_file_of_the_packages() {
        // The bundler names the file `usr/share/applications/<productName>.desktop`, for the .deb
        // and the AppImage alike.
        assert_eq!(json("tauri.conf.json")["productName"], APP_ID);
    }

    #[test]
    fn the_desktop_file_gives_the_same_name_in_x11() {
        let linux = json("tauri.linux.conf.json");
        // The AppImage takes its `.desktop` from the .deb's data, template included.
        let template = linux["bundle"]["linux"]["deb"]["desktopTemplate"]
            .as_str()
            .expect("a .desktop template for the .deb and the AppImage");
        let desktop = read(template);
        let lines: Vec<&str> = desktop.lines().collect();
        // `{{name}}` is productName, as the file's name.
        assert!(lines.contains(&"StartupWMClass={{name}}"), "{desktop}");
        assert!(lines.contains(&"Icon={{icon}}"), "{desktop}");
        // `%u`: the link `escouade://` the desktop hands over (STORE-11).
        assert!(lines.contains(&"Exec={{exec}} %u"), "{desktop}");
        assert!(lines.contains(&"Type=Application"), "{desktop}");
        // WSLg leaves out of its list, and so without an icon, the entries it cannot show.
        assert!(!desktop.contains("NoDisplay"), "{desktop}");
        assert!(!desktop.contains("Terminal=true"), "{desktop}");
    }
}
