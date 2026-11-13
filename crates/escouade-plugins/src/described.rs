//! What a manifest says of the harnesses it declares (`contributes.harnesses`, PLG-6): the app's `plugins::harness`
//! registers and runs them.

use escouade_plugin_protocol::engine as proto;
use serde_json::Value;

/// The harnesses a manifest declares (`contributes.harnesses`), those that read.
pub fn harnesses_of(contributes: &serde_json::Map<String, Value>) -> Vec<proto::HarnessDecl> {
    contributes
        .get("harnesses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|h| match serde_json::from_value(h.clone()) {
            Ok(d) => Some(d),
            Err(e) => {
                log::warn!("a harness of a plugin's manifest left aside: {e}");
                None
            }
        })
        .collect()
}

/// What the harness `decl` describes runs, a line each, as the installation shows it (`plugins::asks`): for each name of its
/// program, its variables and its arguments, and what asks its version, unless that is the usual `--version`. Nothing for
/// a harness that is not described.
pub fn described_lines(decl: &proto::HarnessDecl) -> Vec<String> {
    let Some(acp) = &decl.acp else {
        return Vec::new();
    };
    let names = if decl.locate.names.is_empty() {
        std::slice::from_ref(&decl.id)
    } else {
        decl.locate.names.as_slice()
    };
    let usual = decl.version_args == ["--version"];
    let mut out = Vec::new();
    for name in names {
        let vars = acp.env.iter().map(|(k, v)| format!("{k}={v}"));
        let run: Vec<String> = vars
            .chain(std::iter::once(name.clone()))
            .chain(acp.args.iter().cloned())
            .collect();
        out.push(run.join(" "));
        if !usual {
            out.push(format!("{name} {}", decl.version_args.join(" ")));
        }
    }
    out
}
