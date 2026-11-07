//! The plugin Cursor CLI (`dev.escouade.cursor-cli`, INT-10.1, docs/PLUGINS.md §4): Cursor CLI as a harness, over ACP.
//! The harness its manifest declares (`contributes.harnesses[0]`, its `acp` object with the extension `cursor`) is
//! served by the client of `escouade_acp`, as the generic ACP plugin serves those other plugins describe: the core
//! finds `agent`, checks the launch this plugin describes and starts it; what Cursor writes comes here
//! (`agent/frames`) and goes back as the items of the conversation and the lines to write to it.

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

pub fn run() -> i32 {
    let own = escouade_acp::plugin::harnesses_of(MANIFEST)
        .into_iter()
        .next();
    escouade_acp::plugin::serve_as("cursor", own)
}

escouade_plugin_protocol::module!(run);

#[cfg(test)]
mod tests {
    #[test]
    fn its_manifest_declares_cursor_on_the_client_with_its_extension() {
        let h = escouade_acp::plugin::harnesses_of(super::MANIFEST);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].id, "cursor");
        let acp = h[0].acp.as_ref().expect("its acp object");
        assert_eq!(acp.args, ["acp"]);
        assert_eq!(acp.extensions, ["cursor"]);
    }
}
