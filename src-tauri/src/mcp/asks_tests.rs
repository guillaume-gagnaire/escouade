//! The tools of the box « À toi » (ESC-509) through a real MCP client (rmcp's): `ask_user` with files on
//! the request and on its options, by path and by content; `get_ask`, `get_ask_file`, `list_asks`,
//! `wait_ask`, `update_ask`, `withdraw_ask`; `answer_ask` refused but by delegation; a private project's
//! requests not found by the others; the log without a request's text.

use super::asks::AGENT_DATA_NOTICE;
use super::private_tests::make_private;
use super::tools_tests::{
    as_agent, call, external, other_project, project, put, put_agent, read, refused, ticket, Client,
};
use crate::asks::{AnswerIn, AskState};
use crate::core_tests::harness;
use crate::model::*;
use base64::Engine;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Value};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
const PDF: &[u8] = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

/// The blocks a tool answers, as JSON.
async fn blocks(c: &Client, tool: &'static str, args: Value) -> Vec<Value> {
    let mut params = CallToolRequestParams::new(tool);
    if let Value::Object(args) = args {
        params = params.with_arguments(args);
    }
    let answer = serde_json::to_value(c.call_tool(params).await.unwrap()).unwrap();
    assert!(answer["isError"] != true, "{tool}: {answer}");
    answer["content"].as_array().unwrap().clone()
}

/// A request of three options, each with its render, and a PDF of its own: by path (relative to the
/// working folder) and by content.
fn three_options() -> Value {
    json!({
        "kind": "choice",
        "title": "Choisis la direction du tableau de bord",
        "body": "J’ai rendu trois directions.",
        "options": [
            { "label": "Colonnes de chiffres", "files": [{ "path": "design/a.png" }] },
            { "label": "Cartes empilées", "files": [{ "path": "design/b.png" }] },
            { "label": "Tableau dense", "files": [{ "name": "c.png", "data": b64(PNG) }] }
        ],
        "recommend": { "option": "b", "why": "Les adhérents arrivent du mobile." },
        "unblocks": "DEM-13 démarre sur la direction choisie",
        "files": [{ "name": "etude.pdf", "data": b64(PDF) }]
    })
}

fn renders(dir: &str) {
    let d = std::path::Path::new(dir).join("design");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("a.png"), PNG).unwrap();
    std::fs::write(d.join("b.png"), PNG).unwrap();
}

#[tokio::test]
async fn an_agent_leaves_a_choice_with_its_renders_and_reads_it_back() {
    let h = harness("mcp-ask-user");
    let p = project(&h).await;
    let me = put_agent(&h, &p, "design", 1);
    renders(&p.path);
    let c = as_agent(&h, &me.id).await;
    let left = read(&c, "ask_user", three_options()).await;
    assert_eq!(left["id"], "a-1");
    assert_eq!(left["state"], "open");
    assert_eq!(left["files"], 4);
    assert!(left["expiresAt"].as_str().unwrap().ends_with('Z'));
    // Its own request, whole, not marked.
    let got = read(&c, "get_ask", json!({ "ask": "a-1" })).await;
    assert_eq!(got["author"], "design");
    assert_eq!(got["recommend"]["option"], "b");
    let files = got["files"].as_array().unwrap();
    assert_eq!(files.len(), 4);
    assert_eq!(files[0]["type"], "pdf");
    assert!(files[0]["option"].is_null());
    assert_eq!(files[1]["option"], "a");
    assert_eq!(files[2]["type"], "image");
    // An image is read back as an image.
    let image = blocks(&c, "get_ask_file", json!({ "ask": "a-1", "index": 1 })).await;
    assert_eq!(image[0]["type"], "image");
    assert_eq!(image[0]["mimeType"], "image/png");
    let pdf = blocks(&c, "get_ask_file", json!({ "ask": "a-1", "index": 0 })).await;
    let pdf: Value = serde_json::from_str(pdf[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(pdf["mediaType"], "application/pdf");
    assert!(
        pdf.get("path").is_none(),
        "an agent is not given the copy's path"
    );
    // The log has the call, never its text.
    let last = h
        .core
        .mcp
        .activity
        .entries()
        .into_iter()
        .find(|e| e.tool == "ask_user")
        .unwrap();
    let logged = serde_json::to_string(&last).unwrap();
    assert!(
        !logged.contains("tableau de bord") && !logged.contains("a.png"),
        "{logged}"
    );
    // Its own list.
    let mine = read(&c, "list_asks", json!({})).await;
    assert_eq!(mine["asks"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_file_outside_the_working_folder_a_lying_one_or_too_many_are_refused_leaving_nothing() {
    let h = harness("mcp-ask-files-refused");
    let p = project(&h).await;
    let me = put_agent(&h, &p, "design", 1);
    let c = as_agent(&h, &me.id).await;
    let outside = h.dir.join("ailleurs.png");
    std::fs::write(&outside, PNG).unwrap();
    let mut args = three_options();
    args["files"] = json!([{ "path": outside.to_string_lossy() }]);
    args["options"][0]["files"] = json!([]);
    args["options"][1]["files"] = json!([]);
    let e = refused(&c, "ask_user", args).await;
    assert!(e.contains("dossier de travail"), "{e}");
    let mut lying = three_options();
    lying["options"] = json!([
        { "label": "A", "files": [{ "name": "a.png", "data": b64(PDF) }] },
        { "label": "B" }
    ]);
    lying["files"] = json!([]);
    let e = refused(&c, "ask_user", lying).await;
    assert!(
        e.contains("« a.png » n’est pas une image PNG, JPEG, GIF, WebP ou AVIF"),
        "{e}"
    );
    let mut many = three_options();
    many["files"] = json!((0..13)
        .map(|i| json!({ "name": format!("{i}.png"), "data": b64(PNG) }))
        .collect::<Vec<_>>());
    many["options"] = json!([{ "label": "A" }, { "label": "B" }]);
    let e = refused(&c, "ask_user", many).await;
    assert!(
        e.contains("12 fichiers au plus par demande (13 donnés)"),
        "{e}"
    );
    let e = refused(
        &c,
        "ask_user",
        json!({ "kind": "choice", "title": "Une", "options": [{ "label": "A" }] }),
    )
    .await;
    assert!(e.contains("2 à 6 options"), "{e}");
    assert!(h.core.asks_all().is_empty());
}

#[tokio::test]
async fn only_its_author_changes_or_withdraws_a_request_and_another_reads_it_as_data() {
    let h = harness("mcp-ask-author");
    let p = project(&h).await;
    let me = put_agent(&h, &p, "design", 1);
    let them = put_agent(&h, &p, "back", 2);
    renders(&p.path);
    let c = as_agent(&h, &me.id).await;
    let o = as_agent(&h, &them.id).await;
    read(&c, "ask_user", three_options()).await;
    let e = refused(&o, "update_ask", json!({ "ask": "a-1", "title": "Autre" })).await;
    assert!(e.contains("Seul l’auteur"), "{e}");
    let e = refused(&o, "withdraw_ask", json!({ "ask": "a-1", "reason": "x" })).await;
    assert!(e.contains("Seul l’auteur"), "{e}");
    // Read by another, it is marked as data.
    let seen = blocks(&o, "get_ask", json!({ "ask": "a-1" })).await;
    assert_eq!(seen[0]["text"], AGENT_DATA_NOTICE);
    assert!(seen[1]["text"]
        .as_str()
        .unwrap()
        .starts_with("<third_party_data>"));
    let all = blocks(&o, "list_asks", json!({ "mine": false })).await;
    assert_eq!(all[0]["text"], AGENT_DATA_NOTICE);
    // Its author updates it: an option goes with its file, a file is added.
    let updated = read(
        &c,
        "update_ask",
        json!({ "ask": "a-1", "options": [{ "label": "Colonnes" }, { "label": "Cartes" }], "addFiles": [{ "name": "notes.md", "data": b64(b"# Notes\n") }] }),
    )
    .await;
    assert_eq!(
        updated["files"], 4,
        "the render of c went with its option, the notes came"
    );
    let got = read(&c, "get_ask", json!({ "ask": "a-1" })).await;
    assert_eq!(got["files"][3]["type"], "markdown");
    assert!(!got["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["option"] == "c"));
    let done = read(
        &c,
        "withdraw_ask",
        json!({ "ask": "a-1", "reason": "Choisie dans la conversation." }),
    )
    .await;
    assert_eq!(done["state"], "withdrawn");
}

#[tokio::test]
async fn no_tool_answers_for_the_user_but_by_the_delegation_the_user_gave() {
    let h = harness("mcp-answer-ask");
    let p = project(&h).await;
    let me = put_agent(&h, &p, "design", 1);
    let chef = put_agent(&h, &p, "chef", 2);
    let other = put_agent(&h, &p, "autre", 3);
    h.core.settings.write().mcp_enabled = true;
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .chef = Some(chef.id.clone());
    renders(&p.path);
    let c = as_agent(&h, &me.id).await;
    read(&c, "ask_user", three_options()).await;
    let answer = json!({ "ask": "a-1", "options": ["b"] });
    let e = refused(&external(&h).await, "answer_ask", answer.clone()).await;
    assert!(e.contains("c’est l’utilisateur qui répond"), "{e}");
    let e = refused(&c, "answer_ask", answer.clone()).await;
    assert!(e.contains("sa propre demande"), "{e}");
    let ch = as_agent(&h, &chef.id).await;
    let e = refused(&ch, "answer_ask", answer.clone()).await;
    assert!(e.contains("ne t’a pas laissé cette demande"), "{e}");
    // The user leaves it to the chef, as the settings let them.
    h.core.settings.write().agents_answer = AgentsAnswer::Chef;
    h.core.ask_delegate("a-1", Some(&chef.id)).unwrap();
    let left = blocks(&ch, "list_asks", json!({ "delegated": true })).await;
    assert!(left[1]["text"].as_str().unwrap().contains("\"a-1\""));
    let e = refused(&as_agent(&h, &other.id).await, "answer_ask", answer.clone()).await;
    assert!(e.contains("ne t’a pas laissé"), "{e}");
    // The settings changed since: refused.
    h.core.settings.write().agents_answer = AgentsAnswer::Nobody;
    let e = refused(&ch, "answer_ask", answer.clone()).await;
    assert!(e.contains("Réglages › Agents › Piloter Escouade"), "{e}");
    h.core.settings.write().agents_answer = AgentsAnswer::Chef;
    let done = read(
        &ch,
        "answer_ask",
        json!({ "ask": "a-1", "options": ["b"], "note": "les cartes" }),
    )
    .await;
    assert_eq!(done["state"], "answered");
    let a = h.core.ask("a-1").unwrap();
    assert_eq!(a.answer.as_ref().unwrap().by_name.as_deref(), Some("chef"));
    // Its author reads it: marked read.
    let got = read(&c, "get_ask", json!({ "ask": "a-1" })).await;
    assert_eq!(got["answer"]["by"], "chef");
    assert_eq!(got["answer"]["via"], "delegated");
    assert_eq!(h.core.ask("a-1").unwrap().state, AskState::Read);
}

#[tokio::test]
async fn wait_ask_answers_once_the_request_changes_or_its_time_runs_out() {
    let h = harness("mcp-wait-ask");
    let p = project(&h).await;
    let ext = external(&h).await;
    let left = read(
        &ext,
        "ask_user",
        json!({ "kind": "confirm", "title": "Publier ?", "gate": true, "project": p.id }),
    )
    .await;
    assert_eq!(left["id"], "a-1");
    let still = read(
        &ext,
        "wait_ask",
        json!({ "ask": "a-1", "timeoutSeconds": 1 }),
    )
    .await;
    assert_eq!(still["changed"], false);
    let core = h.core.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        core.settings.write().mcp_enabled = true;
        core.ask_answer(
            "a-1",
            AnswerIn {
                yes: Some(true),
                ..Default::default()
            },
            None,
            vec![],
        )
        .unwrap();
    });
    let changed = read(
        &ext,
        "wait_ask",
        json!({ "ask": "a-1", "timeoutSeconds": 20 }),
    )
    .await;
    assert_eq!(changed["changed"], true);
    assert_eq!(changed["answer"]["yes"], true);
    assert_eq!(changed["answer"]["pending"], true);
    // Claude outside Escouade names the project; an agent of Escouade may leave it out.
    let e = refused(
        &ext,
        "ask_user",
        json!({ "kind": "answer", "title": "Ton nom ?" }),
    )
    .await;
    assert!(e.contains("projet"), "{e}");
}

#[tokio::test]
async fn a_private_projects_requests_are_found_by_its_own_agents_alone() {
    let h = harness("mcp-ask-private");
    let open = project(&h).await;
    let vault = other_project(&h, "Coffre").await;
    let vault = make_private(&h, &vault);
    crate::harness_scope_tests::turn_on_every_harness(&h, &vault.id);
    let inside = put_agent(&h, &vault, "Sigrid", 1);
    let outside = put_agent(&h, &open, "Alice", 2);
    let t = put(&h, ticket(&vault, "SEC-1", "Secret", Column::Todo, 3));
    let i = as_agent(&h, &inside.id).await;
    let left = read(
        &i,
        "ask_user",
        json!({ "kind": "answer", "title": "Quel nom ?", "tickets": [t.key] }),
    )
    .await;
    assert_eq!(left["id"], "a-1");
    let o = as_agent(&h, &outside.id).await;
    let unknown = refused(&o, "get_ask", json!({ "ask": "a-99" })).await;
    let private = refused(&o, "get_ask", json!({ "ask": "a-1" })).await;
    assert_eq!(
        private.replace("a-1", "a-99"),
        unknown,
        "the same words as one that does not exist"
    );
    for tool in [
        "get_ask_file",
        "wait_ask",
        "withdraw_ask",
        "answer_ask",
        "update_ask",
    ] {
        let (text, error) = call(&o, tool, json!({ "ask": "a-1" })).await;
        assert!(error, "{tool}: {text}");
        assert!(
            text.contains("Aucune demande") || text.contains("Aucun projet"),
            "{tool}: {text}"
        );
    }
    let all = blocks(&o, "list_asks", json!({ "mine": false, "state": "all" })).await;
    assert!(!all
        .iter()
        .any(|b| b["text"].as_str().unwrap_or("").contains("a-1")));
    // An agent of the private project leaves none in another project.
    let e = refused(
        &i,
        "ask_user",
        json!({ "kind": "answer", "title": "Ailleurs ?", "project": open.id }),
    )
    .await;
    assert!(e.contains("Aucun projet"), "{e}");
}

/// The server's instructions say when to leave a request in the box rather than in a report
/// (ESC-513, docs/INBOX.md § 13): given with the server only, since off there is none.
#[tokio::test]
async fn the_server_tells_its_agents_when_to_leave_a_request() {
    let h = crate::core_tests::harness("mcp-asks-guidance");
    h.core.settings.write().mcp_enabled = true;
    let c = super::tools_tests::external(&h).await;
    let said = c
        .peer_info()
        .and_then(|i| i.instructions.clone())
        .unwrap_or_default();
    for part in [
        "without being blocked",
        "with ask_user instead of only writing it in your report",
        "a go-ahead to publish, sign, delete or spend",
        "a trial only the user can make",
        "join your renders to their options (light and dark)",
        "withdraw_ask",
        "A question that blocks you stays a question",
        "Never put a secret in a request",
    ] {
        assert!(said.contains(part), "{part}: {said}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
