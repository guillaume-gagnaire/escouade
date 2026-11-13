//! What a subagent's files tell its agent (`sidechain`): how it ended, the model it runs on.

use super::*;
use sidechain::Ended;

impl AgentRt {
    /// The subagent that `call` launched ended as its files show (`how_it_ended`): its row of the
    /// plan ends as a `task_notification` would end it, with `run` written on its call. A row that
    /// ended already stays as it was.
    pub fn end_subagent(&mut self, call: &str, how: Ended, fx: &mut Effects) {
        let frame = json!({
            "type": "system", "subtype": "task_notification", "task_id": null,
            "tool_use_id": call, "status": how.status(),
        });
        self.plan_apply(fx, |plan| plan.on_system(&frame, now_ms()));
    }

    /// The subagent that `call` launched runs on `model`, as its transcript shows (`model_of`).
    pub fn subagent_model(&mut self, call: &str, model: &str, fx: &mut Effects) {
        self.child_model(call, model, fx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidechain::sidechain_items;

    /// The task of the subagent, as its entries say it.
    const TASK: &str = "a3c004cb08b483eb9";

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("escouade-sidechain-{name}-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry(kind: &str, content: Value) -> String {
        let n = new_id();
        json!({ "type": kind, "isSidechain": true, "agentId": TASK, "uuid": n,
                "timestamp": "2026-10-15T11:38:20.516Z", "message": { "content": content } })
        .to_string()
    }

    fn result_entry(call: &str, content: Value, is_error: bool, structured: Value) -> String {
        let mut e: Value = serde_json::from_str(&entry(
            "user",
            json!([{ "type": "tool_result", "tool_use_id": call, "content": content, "is_error": is_error }]),
        ))
        .unwrap();
        e["toolUseResult"] = structured;
        e.to_string()
    }

    /// The conversation items of the blocks, told as the stream tells them: the thread of `c1`.
    fn streamed(frames: &[Value]) -> Vec<Value> {
        let meta = AgentMeta {
            id: new_id(),
            cwd: "C:/p".into(),
            ..Default::default()
        };
        let mut a = AgentRt::new(meta, &dir("stream"));
        let mut fx = Effects::default();
        for f in frames {
            a.handle_frame(f, &mut fx);
        }
        a.conv
            .items()
            .into_iter()
            .filter(|i| i["parent"] == "c1")
            .collect()
    }

    #[test]
    fn the_items_have_the_forms_the_stream_gives_the_same_blocks() {
        let blocks = [
            json!({ "type": "thinking", "thinking": "Je vois." }),
            json!({ "type": "text", "text": "Je lance." }),
            json!({ "type": "tool_use", "id": "k1", "name": "Bash", "input": { "command": "ls" } }),
            json!({ "type": "tool_use", "id": "k2", "name": "Edit", "input": { "file_path": "C:/p/a.ts" } }),
        ];
        let results = [
            (
                json!({ "type": "tool_result", "tool_use_id": "k1", "content": "a\nb", "is_error": false }),
                json!({ "stdout": "a\nb", "stderr": "w" }),
            ),
            (
                json!({ "type": "tool_result", "tool_use_id": "k2", "content": "Échec", "is_error": true }),
                json!({ "filePath": "C:/p/a.ts", "structuredPatch": [{ "oldStart": 1, "newStart": 1, "lines": ["-a", "+b"] }] }),
            ),
        ];
        let mut frames = vec![json!({ "type": "assistant", "parent_tool_use_id": "c1",
            "message": { "id": "m1", "content": blocks } })];
        // A transcript has one block to an entry.
        let mut lines: Vec<String> = blocks
            .iter()
            .map(|b| entry("assistant", json!([b])))
            .collect();
        for (block, structured) in &results {
            frames.push(json!({ "type": "user", "parent_tool_use_id": "c1",
                "message": { "content": [block] }, "tool_use_result": structured }));
            let id = block["tool_use_id"].as_str().unwrap();
            lines.push(result_entry(
                id,
                block["content"].clone(),
                block["is_error"] == true,
                structured.clone(),
            ));
        }
        // What differs on purpose: the ids of the words, and when.
        let plain = |mut items: Vec<Value>| {
            for i in &mut items {
                i.as_object_mut().unwrap().remove("ts");
                if i["kind"] != "tool" {
                    i.as_object_mut().unwrap().remove("id");
                }
            }
            items
        };
        assert_eq!(
            plain(sidechain_items(&lines.join("\n"), "c1")),
            plain(streamed(&frames))
        );
    }
}
