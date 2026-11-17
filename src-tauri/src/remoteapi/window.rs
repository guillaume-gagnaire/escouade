//! The desktop app's own window on the remote API (ESC-495, `docs/REMOTE.md`, « La fondation 2b »): in a build of
//! development only, when `ESCOUADE_REMOTE_API` gives a port and `ESCOUADE_REMOTE_API_TOKEN` a token (the e2e on the
//! remote API), the window speaks to the core by the API's WebSocket instead of Tauri's IPC.
//!
//! - The API's socket is bound before the window is made ([`bind`]): the page is told its address from its first line
//!   (`window.__ESCOUADE_REMOTE__`, an initialization script of the main window alone, and of Escouade's own page alone:
//!   never a launch's page, which is another webview), and the API serves on it once the core is up.
//! - The window's content security policy lets it reach the loopback's WebSockets ([`open_csp`]): in that case alone.
//! - Nothing of it in a release: a variable opens nothing there.
//! - With `ESCOUADE_REMOTE_URL` in place of the port (ESC-514), the window speaks to another core's API: an
//!   `escouade-server` reached through a local forward, as a client of the server mode will (the e2e of `server.spec.ts`).
//!   The app's own core serves nothing then; the desktop's commands are still its own (the IPC).

use std::net::{Ipv4Addr, TcpListener};

/// In a build of development, the address of another core's API on the loopback (`ws://127.0.0.1:<port>/remote`), which
/// the window speaks to with the token of `ESCOUADE_REMOTE_API_TOKEN`.
pub const URL_VAR: &str = "ESCOUADE_REMOTE_URL";

/// The link of the window to the API: its socket, bound (none when the API is another process's), and what the page
/// is told.
pub struct WindowLink {
    pub listener: Option<TcpListener>,
    pub url: String,
    token: String,
}

/// The variables ask for it.
pub fn wanted() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    let set = |k: &str| std::env::var(k).is_ok_and(|v| !v.trim().is_empty());
    (set(super::server::PORT_VAR) || set(URL_VAR)) && set(super::server::TEST_TOKEN_VAR)
}

/// The address of `ESCOUADE_REMOTE_URL`, when it is a WebSocket of the loopback (what the window's policy lets it reach).
fn other_core() -> Option<String> {
    let url = std::env::var(URL_VAR).ok()?;
    let url = url.trim();
    let parsed = tauri::Url::parse(url).ok()?;
    let loopback = parsed.host_str() == Some("127.0.0.1");
    if parsed.scheme() != "ws" || !loopback || parsed.port().is_none() {
        log::warn!("remote API: {URL_VAR} is not a WebSocket of 127.0.0.1 with a port: the window keeps the IPC");
        return None;
    }
    Some(url.to_string())
}

/// The API's socket bound on 127.0.0.1 at the port of `ESCOUADE_REMOTE_API` (0: a free one), when both variables say
/// the window is to speak by it; or the address of another core's (`ESCOUADE_REMOTE_URL`), nothing bound.
pub fn bind() -> Option<WindowLink> {
    if !wanted() {
        return None;
    }
    let token = std::env::var(super::server::TEST_TOKEN_VAR).ok()?;
    if std::env::var(URL_VAR).is_ok_and(|v| !v.trim().is_empty()) {
        let url = other_core()?;
        log::info!("remote API: the window speaks to another core, through {url}");
        return Some(WindowLink {
            listener: None,
            url,
            token,
        });
    }
    let port = super::server::port_asked()?;
    match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => {
            let addr = listener.local_addr().ok()?;
            log::info!("remote API: the window speaks to the core through {addr}");
            Some(WindowLink {
                url: format!("ws://{addr}{}", super::proto::PATH),
                listener: Some(listener),
                token,
            })
        }
        Err(e) => {
            log::warn!(
                "remote API: port {port} not bound for the window ({e}): the window keeps the IPC"
            );
            None
        }
    }
}

impl WindowLink {
    /// The initialization script that tells Escouade's own page where the API is, its token and the app's version.
    /// Another origin (a page the window was sent to) is told nothing.
    pub fn script(&self, version: &str, dev: Option<&str>) -> String {
        let link = serde_json::json!({ "url": self.url, "token": self.token, "version": version });
        let mut origins = vec![
            "http://tauri.localhost".to_string(),
            "https://tauri.localhost".to_string(),
            "tauri://localhost".to_string(),
        ];
        if let Some(dev) = dev {
            origins.push(dev.trim_end_matches('/').to_string());
        }
        format!(
            "(function () {{ if ({origins}.indexOf(location.origin) < 0) return; \
             Object.defineProperty(window, '__ESCOUADE_REMOTE__', {{ value: Object.freeze({link}) }}); }})();",
            origins = serde_json::Value::from(origins),
        )
    }
}

/// The window's content security policy, which lets the page reach the WebSockets of the loopback.
pub fn open_csp<R: tauri::Runtime>(context: &mut tauri::Context<R>) {
    let security = &mut context.config_mut().app.security;
    let Some(csp) = security.csp.take() else {
        return;
    };
    let mut policy = csp.to_string();
    if policy.contains("connect-src ") {
        policy = policy.replacen("connect-src ", "connect-src ws://127.0.0.1:* ", 1);
    } else {
        policy.push_str("; connect-src 'self' ws://127.0.0.1:*");
    }
    security.csp = Some(tauri::utils::config::Csp::Policy(policy));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_of_escouade_alone_is_told_where_the_api_is_and_its_policy_lets_it_reach_the_loopback(
    ) {
        let link = WindowLink {
            listener: Some(TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap()),
            url: "ws://127.0.0.1:4210/remote".into(),
            token: "a\"b</script>".into(),
        };
        let script = link.script("2.3.0", None);
        // Its values in JSON (a token that would close a string or the script stays a value of it).
        assert!(script.contains(r#""token":"a\"b</script>""#), "{script}");
        assert!(script.contains(r#""version":"2.3.0""#));
        // Told to Escouade's own page alone.
        assert!(script.contains(
            r#"["http://tauri.localhost","https://tauri.localhost","tauri://localhost"]"#
        ));
        assert!(script.contains("indexOf(location.origin) < 0) return;"));
        let mut context: tauri::Context<tauri::test::MockRuntime> =
            tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().app.security.csp = Some(tauri::utils::config::Csp::Policy(
            "default-src 'self'; connect-src 'self' ipc:".into(),
        ));
        open_csp(&mut context);
        let csp = context
            .config()
            .app
            .security
            .csp
            .as_ref()
            .unwrap()
            .to_string();
        assert_eq!(
            csp,
            "default-src 'self'; connect-src ws://127.0.0.1:* 'self' ipc:"
        );
    }
}
