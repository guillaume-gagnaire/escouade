//! What `escouade-plugin` does without the app's host (ESC-328): read a plugin's manifest (`manifest`), compile a
//! `run.js` plugin to WASM (`wasm`), a `run.wasm` plugin (`rust`, `canon`), and the command itself (`cli`). No Tauri, GTK, WebKit or wasmtime here: the store's
//! image builds this crate alone, and the app takes these from it.

#[macro_use]
pub mod i18n;
pub mod canon;
pub mod cli;
pub mod manifest;
pub mod paths;
pub mod rust;
pub mod wasm;

/// This crate's version: the app's (scripts/version.mjs keeps them together).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
