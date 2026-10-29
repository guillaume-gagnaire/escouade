//! The page's process that ends abnormally (ESC-290): under Linux, the WebKitWebProcess of an
//! AppImage seen aborting each time one project was opened, the window frozen while its core was
//! written. Its end is written in the log, the page is loaded again on a safe view (no project
//! open), and what it showed is kept for the page, which offers to open it again: never at once,
//! for a page that crashes on it would crash again at each start.

use crate::core::Core;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Manager, Runtime, WebviewWindow};

/// What the page showed when its process ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageCrash {
    pub project_id: Option<String>,
    pub agent_id: Option<String>,
}

impl<R: Runtime> Core<R> {
    /// The page's process ended for `reason`: what it showed is kept and closed (the next page
    /// opens on no project), and the line for the log returned. Ids only, never a name nor a path.
    // macOS: WKWebView tells the end of its process to a delegate wry keeps, nothing calls this there.
    #[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
    pub fn page_crashed(&self, reason: &str) -> String {
        let shown = {
            let mut ui = self.ui.write();
            let project = (ui.view == "project")
                .then(|| ui.active_project.clone())
                .flatten();
            let agent = project
                .as_ref()
                .and_then(|p| ui.selected_agent.get(p).cloned());
            ui.view = "project".into();
            ui.active_project = None;
            PageCrash {
                project_id: project,
                agent_id: agent,
            }
        };
        self.request_save();
        let line = format!(
            "page process ended ({reason}), showing project {} agent {}: loaded again on no project",
            shown.project_id.as_deref().unwrap_or("-"),
            shown.agent_id.as_deref().unwrap_or("-"),
        );
        *self.page_crash.lock() = Some(shown);
        line
    }

    /// What the page showed at its last crash, told once to the page that follows.
    pub fn take_page_crash(&self) -> Option<PageCrash> {
        self.page_crash.lock().take()
    }
}

/// The page's process ended for `reason`: written, and the page loaded again.
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
pub fn crashed<R: Runtime>(app: &tauri::AppHandle<R>, reason: &str) {
    let Some(core) = app.try_state::<Arc<Core<R>>>() else {
        return;
    };
    log::error!("{}", core.page_crashed(reason));
    crate::navigation::load_home(app);
}

/// Listens to the end of the page's process of `window` (WebKitGTK, WebView2). macOS's WKWebView
/// tells it to a delegate wry keeps: nothing there.
pub fn install<R: Runtime>(window: &WebviewWindow<R>) {
    let app = window.app_handle().clone();
    let asked = window.with_webview(move |webview| listen(webview, app));
    if let Err(e) = asked {
        log::error!("the end of the page's process is not followed: {e}");
    }
}

#[cfg(target_os = "linux")]
fn listen<R: Runtime>(webview: tauri::webview::PlatformWebview, app: tauri::AppHandle<R>) {
    use webkit2gtk::{WebProcessTerminationReason, WebViewExt};
    webview
        .inner()
        .connect_web_process_terminated(move |_, reason| {
            let reason = match reason {
                WebProcessTerminationReason::Crashed => "crashed",
                WebProcessTerminationReason::ExceededMemoryLimit => "out of memory",
                // Ended by the app itself: nothing went wrong.
                WebProcessTerminationReason::TerminatedByApi => return,
                _ => "ended",
            };
            let app = app.clone();
            tauri::async_runtime::spawn(async move { crashed(&app, reason) });
        });
}

#[cfg(windows)]
fn listen<R: Runtime>(webview: tauri::webview::PlatformWebview, app: tauri::AppHandle<R>) {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::ProcessFailedEventHandler;
    let handler = ProcessFailedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else { return Ok(()) };
        let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
        // SAFETY: `args` is the event's, alive for the call.
        unsafe { args.ProcessFailedKind(&mut kind)? };
        let reason = match kind {
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => "crashed",
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => "unresponsive",
            COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED => "browser process ended",
            // A GPU or utility process: WebView2 starts it again by itself, the page goes on.
            _ => return Ok(()),
        };
        let app = app.clone();
        tauri::async_runtime::spawn(async move { crashed(&app, reason) });
        Ok(())
    }));
    // SAFETY: the controller is the live one of this WebView, used on its thread (`with_webview`).
    let added = unsafe {
        webview.controller().CoreWebView2().and_then(|core| {
            // The registration lasts as long as the WebView: it is never removed.
            let mut token = 0;
            core.add_ProcessFailed(&handler, &mut token)
        })
    };
    if let Err(e) = added {
        log::error!("the end of the page's process is not followed: {e}");
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn listen<R: Runtime>(_webview: tauri::webview::PlatformWebview, _app: tauri::AppHandle<R>) {}

#[cfg(test)]
mod tests {
    use crate::core_tests::harness;

    /// The page crashed on a project: its next load opens none, and is told which one it was,
    /// once, to offer to open it again; the log has ids only.
    #[test]
    fn a_page_that_crashed_comes_back_on_no_project_and_is_told_what_it_showed() {
        let h = harness("page-crash");
        {
            let mut ui = h.core.ui.write();
            ui.view = "project".into();
            ui.active_project = Some("p1".into());
            ui.selected_agent.insert("p1".into(), "a1".into());
        }
        let line = h.core.page_crashed("crashed");
        assert_eq!(
            line,
            "page process ended (crashed), showing project p1 agent a1: loaded again on no project"
        );
        assert_eq!(h.core.ui.read().active_project, None);
        assert_eq!(h.core.ui.read().view, "project");
        let told = h.core.take_page_crash().unwrap();
        assert_eq!(told.project_id.as_deref(), Some("p1"));
        assert_eq!(told.agent_id.as_deref(), Some("a1"));
        assert_eq!(h.core.take_page_crash(), None);
        // On the statistics, no project was shown.
        h.core.ui.write().view = "stats".into();
        h.core.page_crashed("out of memory");
        assert_eq!(h.core.take_page_crash().unwrap().project_id, None);
    }
}
