//! What the plugin does: it says when a ticket is ready to test, and refuses force pushes. The same source runs as a module
//! in Escouade and under `cargo test` on your machine.

use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::serve::{self, Cx, Plugin, Refusal};
use serde_json::{json, Value};

struct Template;

impl Plugin for Template {
    // Every method of Escouade comes here but `initialize` and `shutdown`, which the kit answers. A notification too: what
    // it returns goes nowhere. Nothing may block here: an `.await` gives the turn back to Escouade.
    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(async move {
            match method {
                "events/fact" => {
                    fact(cx, &params);
                    Ok(Value::Null)
                }
                "permissions/decide" => Ok(decide(&params)),
                other => Err(Refusal::not_found(other)),
            }
        })
    }
}

/// A fact of `permissions.events`: a ticket changed columns (its data says from and to).
fn fact(cx: &Cx, fact: &Value) {
    if fact["name"] == "ticket.column" && fact["data"]["to"] == "review" {
        let key = fact["ticket"]["key"].as_str().unwrap_or_default();
        let project = fact["project"]["name"].as_str().unwrap_or_default();
        cx.log("info", &format!("{key} is ready to test ({project})"));
    }
}

/// A permission an agent asks for, before the user is asked (`permissions.decide`). Answer within 500 ms.
fn decide(params: &Value) -> Value {
    let request = &params["request"];
    let command = request["input"]["command"].as_str().unwrap_or_default();
    if request["kind"] == "command" && is_force_push(command) {
        return json!({
            "decision": "deny",
            "message": "No force push in this team: push a new commit instead.",
        });
    }
    // Leaves it to the user, as without the plugin.
    json!({ "decision": "ask" })
}

fn is_force_push(command: &str) -> bool {
    let mut words = command.split_whitespace();
    words.by_ref().any(|w| w == "git")
        && words.next() == Some("push")
        && words.any(|w| w == "--force" || w == "-f")
}

/// What the module runs at its first turn: the plugin, kept for the turns to come. Natively, the loop on stdin and stdout.
pub fn run() -> i32 {
    serve::run("plugin", Template)
}

// The one line that makes this crate a module of Escouade. Natively, it is nothing.
escouade_plugin_kit::protocol::module!(run);

#[cfg(all(test, not(target_family = "wasm")))]
mod tests;
