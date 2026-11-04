//! `crates/escouade-plugin-kit` for a module (ESC-330): its files as they are, but `serve`.

#[path = "generated/errors.rs"]
pub mod errors;
#[path = "generated/forge.rs"]
pub mod forge;
#[path = "generated/hosts.rs"]
pub mod hosts;
#[path = "generated/http.rs"]
pub mod http;
#[path = "generated/i18n.rs"]
pub mod i18n;
pub mod serve;
#[path = "generated/text.rs"]
pub mod text;

pub use escouade_plugin_protocol as protocol;
