//! The harness side of the plugin Mistral (`dev.escouade.mistral`, INT-10.6 bis, docs/PLUGINS.md §4.13): Mistral Vibe
//! as a harness, over ACP. The same process serves the voice service (`voice/*`, `escouade_lib::voice_plugin_with`):
//! what is not `voice/*` comes to `handler`. The harness its manifest declares (`contributes.harnesses[0]`, its `acp`
//! object with the extension `vibe`) is served by the client of `escouade_acp`, as the generic ACP plugin serves those
//! other plugins describe: the core finds `vibe-acp`, writes the trust of the agent's folder and the refusals of a
//! private project in its `VIBE_HOME`, checks the launch this plugin describes and starts it; what Vibe writes comes
//! here (`agent/frames`) and goes back as the items of the conversation and the lines to write to it.

use escouade_plugin_protocol::Refusal;
use serde_json::{json, Value};

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

/// What this side says it serves at `initialize`.
pub fn capabilities() -> Value {
    json!({ "harnesses": ["vibe"] })
}

/// The answer of the harness to a request that is not `voice/*`.
pub fn handler() -> impl FnMut(&str, Value) -> Result<Value, Refusal> {
    let own = escouade_acp::plugin::harnesses_of(MANIFEST)
        .into_iter()
        .next();
    escouade_acp::plugin::handler(own)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn its_manifest_declares_vibe_on_the_client_with_its_extension() {
        let h = escouade_acp::plugin::harnesses_of(super::MANIFEST);
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

    #[test]
    fn what_it_does_not_know_is_refused_and_not_lost() {
        let mut handle = super::handler();
        let e = handle("nothing/here", json!({})).unwrap_err();
        assert_eq!(e.code, escouade_plugin_protocol::code::METHOD_NOT_FOUND);
    }
}
