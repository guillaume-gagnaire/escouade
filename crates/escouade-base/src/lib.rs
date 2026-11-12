//! What every part of Escouade stands on, cut out of the app's crate for it to compile faster (docs/CRATES.md): the
//! languages written in now (`i18n`), the guards of the paths (`paths`) and of a line of text (`text`). The app
//! re-exports each where it was (`i18n::ui`, `paths::contained`, `mcp::visible_line`, `taskout::safe_origin`), and the
//! crates cut out of it read them here.

#[macro_use]
extern crate escouade_plugin_cli;

pub mod i18n;
pub mod paths;
pub mod text;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
