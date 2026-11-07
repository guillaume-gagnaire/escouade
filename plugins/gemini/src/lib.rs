//! The plugin Gemini CLI (`dev.escouade.gemini-cli`, INT-10.7, docs/PLUGINS.md §4): Gemini CLI as a harness, over ACP.
//! The harness its manifest declares (`contributes.harnesses[0]`, its `acp` object with the extension `gemini`) is served
//! by the client of `escouade_acp`, as the generic ACP plugin serves those other plugins describe: the core finds
//! `gemini`, writes the agent's settings and rules in its `GEMINI_CLI_HOME`, adds `--policy` to the launch this plugin
//! describes, checks it and starts it; what Gemini writes comes here (`agent/frames`) and goes back as the items of the
//! conversation and the lines to write to it.

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

pub fn run() -> i32 {
    let own = escouade_acp::plugin::harnesses_of(MANIFEST)
        .into_iter()
        .next();
    escouade_acp::plugin::serve_as("gemini", own)
}

escouade_plugin_protocol::module!(run);

#[cfg(test)]
mod tests {
    #[test]
    fn its_manifest_declares_gemini_on_the_client_with_its_extension() {
        let h = escouade_acp::plugin::harnesses_of(super::MANIFEST);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].id, "gemini");
        assert_eq!(h[0].account_var.as_deref(), Some("GEMINI_CLI_HOME"));
        assert_eq!(h[0].key_var.as_deref(), Some("GEMINI_API_KEY"));
        let acp = h[0].acp.as_ref().expect("its acp object");
        assert_eq!(acp.args, ["--acp"]);
        assert!(!acp.fs && acp.fence_on_permissions && acp.questions_in_text);
        assert_eq!(acp.extensions, ["gemini"]);
        assert_eq!(acp.env["GEMINI_CLI_TRUST_WORKSPACE"], "true");
        assert_eq!(acp.modes["acceptEdits"], "autoEdit");
        assert_eq!(acp.modes["bypassPermissions"], "yolo");
        // A discussion: its rules refuse its writes and commands (`--policy`, the core's).
        assert_eq!(acp.read_only_mode.as_deref(), Some("default"));
    }
}
