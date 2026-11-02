//! The plugin Mistral Vibe (`dev.escouade.mistral-vibe`, INT-10.6, docs/PLUGINS.md §4): Mistral Vibe as a harness,
//! over ACP. The harness its manifest declares (`contributes.harnesses[0]`, its `acp` object with the extension `vibe`)
//! is served by the client of `escouade_acp`, as the generic ACP plugin serves those other plugins describe: the core
//! finds `vibe-acp`, writes the trust of the agent's folder and the refusals of a private project in its `VIBE_HOME`,
//! checks the launch this plugin describes and starts it; what Vibe writes comes here (`agent/frames`) and goes back as
//! the items of the conversation and the lines to write to it.

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

pub fn run() -> i32 {
    let own = escouade_plugin_acp::harnesses_of(MANIFEST)
        .into_iter()
        .next();
    escouade_plugin_acp::serve_as("vibe", own)
}

#[cfg(test)]
mod tests {
    #[test]
    fn its_manifest_declares_vibe_on_the_client_with_its_extension() {
        let h = escouade_plugin_acp::harnesses_of(super::MANIFEST);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].id, "vibe");
        assert_eq!(h[0].account_var.as_deref(), Some("VIBE_HOME"));
        assert_eq!(h[0].key_var.as_deref(), Some("MISTRAL_API_KEY"));
        let acp = h[0].acp.as_ref().expect("its acp object");
        assert!(acp.args.is_empty() && !acp.fs && acp.fence_on_permissions);
        assert_eq!(acp.extensions, ["vibe"]);
        // Escouade's mode Plan is played by the client: Vibe's session stays in `ask`.
        assert_eq!(acp.modes["plan"], "ask");
        assert!(
            acp.read_only_mode.is_none(),
            "a discussion never goes to it"
        );
    }
}
