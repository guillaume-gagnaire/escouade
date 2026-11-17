//! The desktop app's own window on the remote API (ESC-495, `docs/REMOTE.md`, « La fondation 2b »): in a build of
//! development only, when `ESCOUADE_REMOTE_API` gives a port and `ESCOUADE_REMOTE_API_TOKEN` a token (the e2e on the
//! remote API), the window speaks to the core by the API's WebSocket instead of Tauri's IPC.
//!
//! - The API's socket is bound before the window is made ([`bind`]): the page is told its address from its first line
//!   (`window.__ESCOUADE_REMOTE__`, an initialization script of the main window alone, and of Escouade's own page alone:
//!   never a launch's page, which is another webview), and the API serves on it once the core is up.
//! - The window's content security policy lets it reach the loopback's WebSockets ([`open_csp`]): in that case alone.
//! - Nothing of it in a release: a variable opens nothing there.

use std::net::{Ipv4Addr, TcpListener};

/// The link of the window to the API: its socket, bound, and what the page is told.
pub struct WindowLink {
    pub listener: TcpListener,
    pub url: String,
    token: String,
}

/// The variables ask for it.
pub fn wanted() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    let set = |k: &str| std::env::var(k).is_ok_and(|v| !v.trim().is_empty());
    set(super::server::PORT_VAR) && set(super::server::TEST_TOKEN_VAR)
}

/// The API's socket bound on 127.0.0.1 at the port of `ESCOUADE_REMOTE_API` (0: a free one), when both variables say
/// the window is to speak by it.
pub fn bind() -> Option<WindowLink> {
    if !wanted() {
        return None;
    }
    let port = super::server::port_asked()?;
    let token = std::env::var(super::server::TEST_TOKEN_VAR).ok()?;
    match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => {
            let addr = listener.local_addr().ok()?;
            log::info!("remote API: the window speaks to the core through {addr}");
            Some(WindowLink {
                url: format!("ws://{addr}{}", super::proto::PATH),
                listener,
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
            listener: TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap(),
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
