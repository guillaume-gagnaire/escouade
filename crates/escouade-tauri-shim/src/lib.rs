//! What the core of Escouade takes of Tauri, without Tauri (docs/CRATES.md, § 14; docs/REMOTE.md, « Le mode
//! serveur »).
//!
//! The app's crate builds two ways: with its feature `desktop` (the default), the desktop app, on Tauri; without it,
//! the core alone, for `escouade-server`, which links neither WebKit nor GTK (Tauri links GTK under Linux whatever its
//! features). There this crate stands in Tauri's place, under Tauri's name (`extern crate escouade_tauri_shim as
//! tauri;` at the root of the app's crate): the modules of the core keep writing `tauri::async_runtime::spawn`,
//! `tauri::ipc::Channel`, `tauri::http::HeaderMap`, unchanged. It holds those pieces alone, with Tauri's signatures
//! and behavior; what needs a window (`AppHandle`, `Manager`, the WebViews) is not here: the modules of the desktop are
//! not built without `desktop`.
//!
//! With `desktop`, the crate is only `async_runtime`, Tauri's own: the crates of the workspace that start tasks
//! (`escouade-plugins`) take it here, and run on the same runtime as the app.

#[cfg(feature = "desktop")]
pub use tauri::async_runtime;

#[cfg(not(feature = "desktop"))]
pub mod async_runtime;
#[cfg(not(feature = "desktop"))]
pub mod ipc;
#[cfg(not(feature = "desktop"))]
pub use http;
#[cfg(not(feature = "desktop"))]
pub use url::Url;

#[cfg(not(feature = "desktop"))]
mod error;
#[cfg(not(feature = "desktop"))]
pub use error::{Error, Result};
