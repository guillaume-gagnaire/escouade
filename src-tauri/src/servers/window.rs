//! A space in its own window (ESC-539, `docs/SERVER-UX.md` § 5, S14): the plate's menu opens it; one window per space
//! at most, a second demand bringing back the first. Its page is the app's, told its space (`?space=<id>`, `here` for
//! this computer): its transport asks the link of that space ([`super::Servers::link_of`]), and its title bar says its
//! machine. Closing it closes it (the main window alone goes to the tray).

use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

/// The labels of the spaces' windows: `space-<id>`, `space-here`.
pub const WINDOW_PREFIX: &str = "space-";

/// The space of this computer, as a window's page names it.
pub const HERE: &str = "here";

/// The label of the window of `space`: its letters and digits alone.
pub fn label(space: &str) -> String {
    let clean: String = space
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    format!("{WINDOW_PREFIX}{clean}")
}

/// Whether `label` is a space's window.
pub fn is_space(label: &str) -> bool {
    label.starts_with(WINDOW_PREFIX)
}

/// The window of `space` brought to the front, made when there is none; `title`, the space's name. When the main
/// window is on that space (`main_on`), it is the one brought back.
pub fn open<R: Runtime>(
    app: &AppHandle<R>,
    space: &str,
    title: &str,
    main_on: bool,
) -> Result<(), String> {
    let front = |w: tauri::WebviewWindow<R>| {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    };
    if main_on {
        if let Some(w) = app.get_webview_window("main") {
            front(w);
        }
        return Ok(());
    }
    let label = label(space);
    if let Some(w) = app.get_webview_window(&label) {
        front(w);
        return Ok(());
    }
    let url = format!("index.html?space={}", urlencode(space));
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title(format!("Escouade · {title}"))
        .inner_size(1280.0, 820.0)
        .min_inner_size(1000.0, 600.0)
        .decorations(false)
        .shadow(true)
        .center()
        .background_color(tauri::window::Color(8, 9, 10, 255))
        .disable_drag_drop_handler()
        .devtools(cfg!(debug_assertions));
    // The WebView2 folder of the main window's (an end-to-end test's): one environment for the app's windows.
    if let Some(dir) = crate::paths::webview_dir() {
        builder = builder.data_directory(dir);
    }
    let dev = app
        .try_state::<crate::navigation::Home>()
        .and_then(|h| h.dev.clone());
    crate::navigation::guard(builder, dev)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_space_window_is_named_by_its_space_alone() {
        assert_eq!(label("here"), "space-here");
        assert_eq!(label("a1b2-c3"), "space-a1b2-c3");
        assert_eq!(label("../x y"), "space-___x_y");
        assert!(is_space(&label("abc")));
        assert!(!is_space("main"));
        assert!(!is_space("pages-p1"));
        assert_eq!(urlencode("a b/é"), "a%20b%2F%C3%A9");
    }
}
