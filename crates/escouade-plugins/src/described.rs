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

/// What else the harness `decl` says the user would not see run (SEC-30), as the installation shows it (`plugins::asks`):
/// the commands that install and update it, which the app then shows (`harnessInstall:<command>`, once if the same on every
/// system, `harnessUpdate:<command>`), and the variables a launch of its takes off its environment
/// (`harnessUnset:<variable>`, those of its description: a harness with code says its own at each launch).
pub fn told_words(decl: &proto::HarnessDecl) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(i) = &decl.install {
        for c in [&i.windows, &i.unix] {
            let word = format!("harnessInstall:{}", c.trim());
            if !c.trim().is_empty() && !out.contains(&word) {
                out.push(word);
            }
        }
    }
    if let Some(u) = decl.update_command.as_deref().map(str::trim) {
        if !u.is_empty() {
            out.push(format!("harnessUpdate:{u}"));
        }
    }
    if let Some(acp) = &decl.acp {
        out.extend(acp.remove_env.iter().map(|v| format!("harnessUnset:{v}")));
    }
    out
}
