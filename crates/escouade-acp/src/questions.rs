//! The questions of an agent that has no tool to ask in ACP (Gemini CLI, Goose): the first message tells it to end its
//! turn on a block `<escouade_question>{"questions": […]}</escouade_question>`, which the client takes out of the text
//! and shows as a question's card; the answers go back as the next message. Shared with the core's Gemini CLI and
//! Antigravity CLI.

use serde_json::{json, Value};

/// What the first message tells the model of its questions.
pub const QUESTIONS: &str = "<escouade>\nYou have no tool to ask the user a question here. When you need the user to choose or answer before going on, end your turn with one block, and nothing after it:\n<escouade_question>{\"questions\": [{\"question\": \"…\", \"options\": [\"…\", \"…\"], \"multiSelect\": false}]}</escouade_question>\nEscouade shows it to the user and sends the answers as the next message.\n</escouade>";

/// The tags of a question's block.
pub const OPEN: &str = "<escouade_question>";
pub const CLOSE: &str = "</escouade_question>";

/// `text` split at its question's block: what comes before, and the block (its tags included).
pub fn split(text: &str) -> (String, Option<String>) {
    match text.find(OPEN) {
        Some(at) => (
            text[..at].trim_end().to_string(),
            Some(text[at..].to_string()),
        ),
        None => (text.to_string(), None),
    }
}

/// The questions of a block as Cursor's `cursor/ask_question` gives them, under the call `call`; None for a block that
/// is not questions.
pub fn question_params(call: &str, block: &str) -> Option<Value> {
    let inner = block.trim().strip_prefix(OPEN)?.split(CLOSE).next()?.trim();
    let inner = inner
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed: Value = serde_json::from_str(inner).ok()?;
    let list = match &parsed {
        Value::Array(_) => &parsed,
        _ => &parsed["questions"],
    };
    let questions: Vec<Value> = list
        .as_array()?
        .iter()
        .enumerate()
        .filter_map(|(i, q)| {
            let text = q["question"].as_str()?.trim();
            if text.is_empty() {
                return None;
            }
            let options: Vec<Value> = q["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|o| o.as_str().or(o["label"].as_str()))
                .map(|l| json!({ "id": l, "label": l }))
                .collect();
            Some(json!({
                "id": format!("q{i}"), "prompt": text, "options": options,
                "allowMultiple": q["multiSelect"] == true,
            }))
        })
        .collect();
    (!questions.is_empty()).then(|| json!({ "toolCallId": call, "questions": questions }))
}

/// The message that gives the model the user's answers to the questions `params` (a card's `answers`, by question).
pub fn answers_message(params: &Value, answers: &Value) -> String {
    let lines: Vec<String> = params["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| {
            let text = q["prompt"].as_str().unwrap_or_default();
            let a = answers[text].as_str().unwrap_or_default().trim();
            format!("- {text} → {a}")
        })
        .collect();
    format!("The user answered your questions:\n{}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_ends_the_text_and_reads_as_questions() {
        let (before, block) = split("Il me faut ton choix.\n<escouade_question>{\"questions\": [{\"question\": \"Rouge ou bleu ?\", \"options\": [\"rouge\", \"bleu\"]}]}</escouade_question>");
        assert_eq!(before, "Il me faut ton choix.");
        let p = question_params("c", &block.unwrap()).unwrap();
        assert_eq!(p["questions"][0]["prompt"], "Rouge ou bleu ?");
        assert_eq!(
            answers_message(&p, &json!({ "Rouge ou bleu ?": "rouge" })),
            "The user answered your questions:\n- Rouge ou bleu ? → rouge"
        );
        assert!(
            question_params("c", "<escouade_question>pas du json</escouade_question>").is_none()
        );
    }
}
