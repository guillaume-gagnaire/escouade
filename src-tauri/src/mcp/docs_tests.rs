//! The docs through the tools (ESC-256), called by a real MCP client (rmcp's) on a test core: the
//! spec written, read and searched, the decisions proposed and replaced but never accepted by a
//! tool, the documents added and read, the spec found in a project that has one, nothing committed,
//! and nothing of a private project for the agents of the others.

use super::private_tests::make_private;
use super::tools_tests::{
    as_agent, call, external, link, other_project, project, put, put_agent, read, refused, ticket,
};
use crate::core_tests::{git, harness};
use crate::model::Column;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn the_spec_is_written_read_searched_and_listed_by_the_tools() {
    let h = harness("mcp-docs-spec");
    let p = project(&h).await;
    let c = external(&h).await;
    let empty = read(&c, "get_spec_tree", json!({ "project": p.name })).await;
    assert_eq!(
        (empty["pages"].clone(), empty["branch"].clone()),
        (json!(0), json!("current"))
    );
    let made = read(
        &c,
        "write_spec_page",
        json!({
            "project": p.name,
            "path": "Paiement/Reçu fiscal",
            "title": "Reçu fiscal",
            "body": "Le reçu part par e-mail.\nIl est en PDF.",
            "tickets": ["ESC-1"],
            "decisions": [2],
        }),
    )
    .await;
    assert_eq!(made["path"], "paiement/recu-fiscal");
    assert_eq!(
        (
            made["created"].clone(),
            made["added"].clone(),
            made["removed"].clone()
        ),
        (json!(true), json!(2), json!(0))
    );
    // Where it lives: in the project's folder, in the repository's files.
    assert!(std::path::Path::new(&p.path)
        .join(".escouade/spec/paiement/recu-fiscal.md")
        .is_file());
    // Changed: the fields left out stay; append adds at the end.
    let more = read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "paiement/recu-fiscal", "body": "Il est signé.", "append": true }),
    )
    .await;
    assert_eq!(
        (more["created"].clone(), more["added"].clone()),
        (json!(false), json!(1))
    );
    let page = read(
        &c,
        "read_spec_page",
        json!({ "project": p.id, "path": "paiement/recu-fiscal" }),
    )
    .await;
    assert_eq!(page["title"], "Reçu fiscal");
    assert_eq!(page["tickets"], json!(["ESC-1"]));
    assert_eq!(page["decisions"], json!([2]));
    assert!(page["body"]
        .as_str()
        .unwrap()
        .ends_with("Il est en PDF.\n\nIl est signé."));
    // A path with no folder goes under its subject, else under general.
    let a = read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "echeances", "subject": "Paiement", "body": "Trois." }),
    )
    .await;
    let b = read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "glossaire", "body": "Mots." }),
    )
    .await;
    assert_eq!(
        (a["path"].clone(), b["path"].clone()),
        (json!("paiement/echeances"), json!("general/glossaire"))
    );
    let tree = read(&c, "get_spec_tree", json!({ "project": p.id })).await;
    assert_eq!(tree["pages"], 3);
    assert_eq!(tree["subjects"][1]["name"], "paiement");
    assert_eq!(tree["existingSpec"], json!([]));
    let hits = read(
        &c,
        "search_spec",
        json!({ "project": p.id, "query": "signé reçu" }),
    )
    .await;
    assert_eq!(hits["hits"][0]["path"], "paiement/recu-fiscal");
    assert_eq!(hits["hits"].as_array().unwrap().len(), 1);
    // Refused: a new page without text, a path that leaves the folder, a page that is not there.
    let no_body = refused(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "a/b" }),
    )
    .await;
    assert!(no_body.contains("body"), "{no_body}");
    for bad in ["../../x", "a/../../x", "C:/x", "/abs"] {
        refused(
            &c,
            "write_spec_page",
            json!({ "project": p.id, "path": bad, "body": "x" }),
        )
        .await;
    }
    refused(
        &c,
        "read_spec_page",
        json!({ "project": p.id, "path": "paiement/rien" }),
    )
    .await;
    // The call is in the log without its text.
    let (_, tool, summary, outcome) = super::tools_tests::last_entry(&h);
    assert_eq!(tool, "read_spec_page");
    assert_ne!(outcome, "ok");
    assert!(!summary.contains("Mots"), "{summary}");
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_spec_the_project_already_has_comes_back_as_a_candidate() {
    let h = harness("mcp-docs-detect");
    let p = project(&h).await;
    std::fs::create_dir_all(std::path::Path::new(&p.path).join("docs")).unwrap();
    std::fs::write(
        std::path::Path::new(&p.path).join("docs/SPEC.md"),
        "# Spec\n",
    )
    .unwrap();
    let c = external(&h).await;
    let tree = read(&c, "get_spec_tree", json!({ "project": p.id })).await;
    assert_eq!(tree["pages"], 0);
    assert_eq!(tree["existingSpec"][0]["path"], "docs/SPEC.md");
    assert_eq!(tree["existingSpec"][0]["kind"], "file");
    // Nothing is imported by the tool.
    assert!(!std::path::Path::new(&p.path).join(".escouade").exists());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn decisions_are_proposed_and_replaced_by_the_tools_and_accepted_by_none() {
    let h = harness("mcp-docs-decisions");
    let p = project(&h).await;
    let c = external(&h).await;
    // No tool accepts: the tools the server holds carry no word of it, and the one that decides is the window's.
    let tools = c.list_all_tools().await.unwrap();
    assert!(tools.iter().all(|t| !t.name.contains("accept")));
    let one = read(
        &c,
        "propose_decision",
        json!({
            "project": p.name,
            "title": "Trois échéances au plus",
            "context": "Les clients payent en plusieurs fois.",
            "decision": "Trois échéances.",
            "consequences": "Un échéancier.",
            "origin": "brainstorm",
            "originRef": "Paiement",
            "pages": ["Paiement/Échéances"],
            "changes": [{ "page": "Paiement/Échéances", "added": 12, "removed": 3 }],
        }),
    )
    .await;
    // No number until it is accepted (ESC-303): an id.
    assert_eq!(
        (
            one["number"].clone(),
            one["id"].clone(),
            one["state"].clone()
        ),
        (
            json!(null),
            json!("trois-echeances-au-plus"),
            json!("proposed")
        )
    );
    assert!(one["note"].as_str().unwrap().contains("id"));
    let id = "trois-echeances-au-plus";
    let listed = read(&c, "list_decisions", json!({ "project": p.id })).await;
    assert_eq!(
        listed["decisions"][0]["changes"][0],
        json!({ "page": "paiement/echeances", "added": 12, "removed": 3 })
    );
    assert_eq!(listed["decisions"][0]["id"], id);
    let whole = read(&c, "read_decision", json!({ "project": p.id, "id": id })).await;
    assert_eq!(whole["context"], "Les clients payent en plusieurs fois.");
    refused(&c, "read_decision", json!({ "project": p.id })).await;
    refused(&c, "read_decision", json!({ "project": p.id, "number": 1 })).await;
    assert_eq!(whole["originRef"], "Paiement");
    // A proposal still open is revised.
    let revised = read(
        &c,
        "propose_decision",
        json!({ "project": p.id, "id": id, "title": "Trois échéances au plus", "decision": "Trois, pas quatre." }),
    )
    .await;
    assert_eq!(
        (revised["id"].clone(), revised["number"].clone()),
        (json!(id), json!(null))
    );
    assert_eq!(
        read(&c, "read_decision", json!({ "project": p.id, "id": id })).await["decision"],
        "Trois, pas quatre."
    );
    // Only a proposal; replacing it is refused while it is not accepted.
    refused(
        &c,
        "replace_decision",
        json!({ "project": p.id, "number": 1, "title": "Autre" }),
    )
    .await;
    // A human accepts, from the window (the core's call): its number then.
    let r = crate::docs::Ref::Id(id.into());
    assert_eq!(
        h.core.docs_accept_decision(&p.id, &r).await.unwrap().number,
        Some(1)
    );
    let accepted = read(
        &c,
        "list_decisions",
        json!({ "project": p.id, "state": "accepted" }),
    )
    .await;
    assert_eq!(accepted["decisions"].as_array().unwrap().len(), 1);
    // An accepted decision is not revised by a tool.
    let file = std::fs::read_to_string(
        std::path::Path::new(&p.path).join(".escouade/decisions/0001-trois-echeances-au-plus.md"),
    )
    .unwrap();
    let text = refused(
        &c,
        "propose_decision",
        json!({ "project": p.id, "number": 1, "title": "Quatre", "decision": "Quatre." }),
    )
    .await;
    assert!(text.contains("1"), "{text}");
    assert_eq!(
        std::fs::read_to_string(
            std::path::Path::new(&p.path)
                .join(".escouade/decisions/0001-trois-echeances-au-plus.md")
        )
        .unwrap(),
        file
    );
    // A new one replaces it; the old one stands until the new one is accepted.
    let new = read(
        &c,
        "replace_decision",
        json!({ "project": p.id, "number": 1, "title": "Quatre échéances", "decision": "Quatre." }),
    )
    .await;
    assert_eq!(
        (
            new["number"].clone(),
            new["id"].clone(),
            new["supersedes"].clone()
        ),
        (json!(null), json!("quatre-echeances"), json!([1]))
    );
    let before = read(&c, "list_decisions", json!({ "project": p.id })).await;
    assert_eq!(before["decisions"][0]["state"], "accepted");
    assert_eq!(
        before["decisions"][0]["replacementProposed"],
        "quatre-echeances"
    );
    let r = crate::docs::Ref::Id("quatre-echeances".into());
    h.core.docs_accept_decision(&p.id, &r).await.unwrap();
    let after = read(&c, "list_decisions", json!({ "project": p.id })).await;
    assert_eq!(after["decisions"][0]["state"], "superseded");
    assert_eq!(after["decisions"][0]["supersededBy"], 2);
    refused(
        &c,
        "list_decisions",
        json!({ "project": p.id, "state": "voulue" }),
    )
    .await;
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn documents_are_added_listed_and_read_by_the_tools() {
    let h = harness("mcp-docs-documents");
    let p = project(&h).await;
    let c = external(&h).await;
    let added = read(
        &c,
        "add_document",
        json!({ "project": p.id, "folder": "comptes rendus", "name": "2026-10-27.md", "text": "# Réunion\nOn décide." }),
    )
    .await;
    assert_eq!(added["path"], "comptes rendus/2026-10-27.md");
    let again = read(&c, "add_document", json!({ "project": p.id, "folder": "comptes rendus", "name": "2026-10-27.md", "text": "autre" })).await;
    assert_eq!(again["path"], "comptes rendus/2026-10-27-2.md");
    std::fs::write(
        std::path::Path::new(&p.path).join(".escouade/documents/plan.pdf"),
        b"%PDF",
    )
    .unwrap();
    let all = read(&c, "list_documents", json!({ "project": p.id })).await;
    assert_eq!(all["documents"].as_array().unwrap().len(), 3);
    let one = read(
        &c,
        "list_documents",
        json!({ "project": p.id, "folder": "comptes rendus" }),
    )
    .await;
    assert_eq!(one["documents"].as_array().unwrap().len(), 2);
    let text = read(
        &c,
        "read_document",
        json!({ "project": p.id, "path": "comptes rendus/2026-10-27.md" }),
    )
    .await;
    assert_eq!(text["text"], "# Réunion\nOn décide.");
    let pdf = read(
        &c,
        "read_document",
        json!({ "project": p.id, "path": "plan.pdf" }),
    )
    .await;
    assert!(pdf["text"].is_null());
    let part = read(&c, "read_document", json!({ "project": p.id, "path": "comptes rendus/2026-10-27.md", "from": 2, "maxChars": 6 })).await;
    assert_eq!(
        (part["text"].clone(), part["truncated"].clone()),
        (json!("Réunio"), json!(true))
    );
    // The text is in Escouade's folder, never in the project.
    assert!(h.core.docs_text_dir(&p.id).is_dir());
    for bad in ["../x.md", "/etc/passwd"] {
        refused(&c, "read_document", json!({ "project": p.id, "path": bad })).await;
        refused(
            &c,
            "add_document",
            json!({ "project": p.id, "folder": bad, "name": "x.md", "text": "x" }),
        )
        .await;
    }
    refused(
        &c,
        "list_documents",
        json!({ "project": p.id, "folder": "../.." }),
    )
    .await;
    // A name with a way up is one name: the separators are gone. Without a folder, it is filed by
    // Escouade: nothing fits it, a folder is made for it.
    let named = read(
        &c,
        "add_document",
        json!({ "project": p.id, "name": "../../evil.md", "text": "x" }),
    )
    .await;
    let path = named["path"].as_str().unwrap();
    assert!(
        path.ends_with("evil.md") && path.matches('/').count() == 1,
        "{named}"
    );
    assert_eq!(named["placement"]["new"], true, "{named}");
    assert!(std::path::Path::new(&p.path)
        .join(".escouade/documents")
        .join(path)
        .is_file());
    assert!(!std::path::Path::new(&p.path).join("evil.md").exists());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_is_committed_by_the_tools_and_a_ticket_agent_writes_in_its_own_folder() {
    let h = harness("mcp-docs-nocommit");
    let p = project(&h).await;
    let repo = std::path::Path::new(&p.path);
    let before = git(repo, &["rev-parse", "HEAD"]);
    // An agent in a folder of its own.
    let a = put_agent(&h, &p, "Alice", 1);
    let wt = h.dir.join("ticket-worktree");
    std::fs::create_dir_all(&wt).unwrap();
    h.core.agent(&a.id).unwrap().lock().meta.cwd = wt.to_string_lossy().to_string();
    let c = as_agent(&h, &a.id).await;
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "a/b", "body": "x" }),
    )
    .await;
    read(
        &c,
        "propose_decision",
        json!({ "project": p.id, "title": "Une décision" }),
    )
    .await;
    read(
        &c,
        "add_document",
        json!({ "project": p.id, "name": "n.md", "text": "x" }),
    )
    .await;
    // In the agent's folder; the project's folder is untouched.
    assert!(wt.join(".escouade/spec/a/b.md").is_file());
    assert!(wt.join(".escouade/decisions").is_dir());
    assert!(!repo.join(".escouade").exists());
    assert_eq!(git(repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(git(repo, &["log", "--oneline", "--all"]).lines().count(), 1);
    assert!(git(repo, &["status", "--porcelain"]).is_empty());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fixed_docs_branch_holds_what_the_tools_write() {
    let h = harness("mcp-docs-fixed");
    let p = project(&h).await;
    let mut q = h.core.project(&p.id).unwrap();
    q.docs_branch = "docs".into();
    h.core.update_project(q).unwrap();
    let c = external(&h).await;
    let tree = read(&c, "get_spec_tree", json!({ "project": p.id })).await;
    assert_eq!(tree["branch"], "docs");
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "a/b", "body": "x" }),
    )
    .await;
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(space.root.join(".escouade/spec/a/b.md").is_file());
    assert!(!std::path::Path::new(&p.path).join(".escouade").exists());
    assert_eq!(
        read(
            &c,
            "read_spec_page",
            json!({ "project": p.id, "path": "a/b" })
        )
        .await["body"],
        "x"
    );
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_project_keeps_its_docs_from_the_agents_of_the_others() {
    let h = harness("mcp-docs-private");
    let open = project(&h).await;
    let vault = other_project(&h, "Coffre").await;
    let vault = make_private(&h, &vault);
    let outsider = put_agent(&h, &open, "Alice", 1);
    let insider = put_agent(&h, &vault, "Sigrid", 2);
    // Its own agent writes and reads its docs.
    let inside = as_agent(&h, &insider.id).await;
    read(
        &inside,
        "write_spec_page",
        json!({ "project": vault.name, "path": "secret/plan", "body": "Le plan secret." }),
    )
    .await;
    read(
        &inside,
        "propose_decision",
        json!({ "project": vault.name, "title": "Secrète" }),
    )
    .await;
    read(
        &inside,
        "add_document",
        json!({ "project": vault.name, "folder": "coffre", "name": "plan.md", "text": "Le plan secret du coffre." }),
    )
    .await;
    assert_eq!(
        read(
            &inside,
            "search_documents",
            json!({ "project": vault.name, "query": "coffre" })
        )
        .await["documents"][0]["path"],
        "coffre/plan.md"
    );
    assert_eq!(
        read(
            &inside,
            "search_spec",
            json!({ "project": vault.name, "query": "plan" })
        )
        .await["hits"][0]["path"],
        "secret/plan"
    );
    // The others find no such project, whatever the tool, nor a trace of what it holds.
    let outside = as_agent(&h, &outsider.id).await;
    for (tool, args) in [
        ("get_spec_tree", json!({ "project": "{}" })),
        (
            "read_spec_page",
            json!({ "project": "{}", "path": "secret/plan" }),
        ),
        ("search_spec", json!({ "project": "{}", "query": "plan" })),
        (
            "write_spec_page",
            json!({ "project": "{}", "path": "x/y", "body": "pirate" }),
        ),
        ("list_decisions", json!({ "project": "{}" })),
        ("read_decision", json!({ "project": "{}", "number": 1 })),
        (
            "propose_decision",
            json!({ "project": "{}", "title": "Pirate" }),
        ),
        (
            "replace_decision",
            json!({ "project": "{}", "number": 1, "title": "Pirate" }),
        ),
        ("list_documents", json!({ "project": "{}" })),
        ("read_document", json!({ "project": "{}", "path": "a.md" })),
        (
            "search_documents",
            json!({ "project": "{}", "query": "plan" }),
        ),
        (
            "add_document",
            json!({ "project": "{}", "name": "a.md", "text": "pirate" }),
        ),
    ] {
        for asked in [vault.id.clone(), vault.name.clone()] {
            let args = serde_json::from_str(&args.to_string().replace("{}", &asked)).unwrap();
            let (text, error) = call(&outside, tool, args).await;
            assert!(error, "{tool}: {text}");
            assert!(
                !text.contains("Le plan secret") && !text.contains("Secrète"),
                "{tool}: {text}"
            );
        }
    }
    assert!(!std::path::Path::new(&vault.path)
        .join(".escouade/spec/x")
        .exists());
    // The window and Claude outside Escouade see it.
    let all = external(&h).await;
    assert_eq!(
        read(&all, "get_spec_tree", json!({ "project": vault.id })).await["pages"],
        1
    );
    inside.cancel().await.unwrap();
    outside.cancel().await.unwrap();
    all.cancel().await.unwrap();
}

/// A remote for the project's repository.
fn with_remote(h: &crate::core_tests::Harness, p: &crate::model::Project) -> std::path::PathBuf {
    let remote = h.dir.join("remote.git");
    git(
        &h.dir,
        &["init", "-q", "--bare", "-b", "main", "remote.git"],
    );
    let at = std::path::Path::new(&p.path);
    git(at, &["remote", "add", "origin", &remote.to_string_lossy()]);
    git(at, &["push", "-q", "-u", "origin", "main"]);
    remote
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_docs_have_to_share_is_read_and_sharing_is_the_users_gesture() {
    let h = harness("mcp-docs-share");
    let p = project(&h).await;
    let remote = with_remote(&h, &p);
    let c = external(&h).await;
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.name, "path": "paiement/echeances", "body": "Trois." }),
    )
    .await;
    let to_share = read(&c, "get_docs_to_share", json!({ "project": p.name })).await;
    assert_eq!(to_share["pending"]["count"], 1);
    assert_eq!(
        to_share["pending"]["files"][0],
        ".escouade/spec/paiement/echeances.md"
    );
    // An agent of Escouade is refused: sharing is the user's gesture.
    let a = put_agent(&h, &p, "Alice", 1);
    let ca = as_agent(&h, &a.id).await;
    let said = refused(&ca, "share_docs", json!({ "project": p.name })).await;
    assert!(said.contains("userAskedForIt"), "{said}");
    let said = refused(
        &ca,
        "share_docs",
        json!({ "project": p.name, "userAskedForIt": false }),
    )
    .await;
    assert!(said.contains("userAskedForIt"), "{said}");
    assert!(git(&remote, &["log", "--oneline", "main"]).lines().count() == 1);
    // It reads what there is to share, as every client does.
    let seen = read(&ca, "get_docs_to_share", json!({ "project": p.name })).await;
    assert_eq!(seen["pending"]["count"], 1);
    // Told by the user, it may.
    let done = read(
        &ca,
        "share_docs",
        json!({ "project": p.name, "userAskedForIt": true, "message": "Les échéances" }),
    )
    .await;
    assert_eq!(done["kind"], "done");
    assert_eq!(done["committed"], 1);
    assert_eq!(
        git(&remote, &["log", "-1", "--format=%s", "main"]),
        "Les échéances"
    );
    // Claude outside Escouade, run by the user, shares as the window does: here nothing is left.
    let again = read(&c, "share_docs", json!({ "project": p.name })).await;
    assert_eq!(again["kind"], "nothing");
    let left = read(&c, "get_docs_to_share", json!({ "project": p.name })).await;
    assert_eq!(left["pending"]["count"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn code_not_pushed_is_published_by_the_tool_only_with_with_code() {
    let h = harness("mcp-docs-share-code");
    let p = project(&h).await;
    let remote = with_remote(&h, &p);
    let at = std::path::Path::new(&p.path);
    std::fs::write(at.join("code.ts"), "x\n").unwrap();
    git(at, &["add", "-A"]);
    git(at, &["commit", "-qm", "du code"]);
    let c = external(&h).await;
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.name, "path": "a/b", "body": "Texte" }),
    )
    .await;
    let before = git(&remote, &["rev-parse", "main"]);
    let said = refused(&c, "share_docs", json!({ "project": p.name })).await;
    assert!(said.contains("commits"), "{said}");
    assert_eq!(git(&remote, &["rev-parse", "main"]), before);
    let seen = read(&c, "get_docs_to_share", json!({ "project": p.name })).await;
    assert_eq!(seen["pending"]["codeCommits"], 1);
    let done = read(
        &c,
        "share_docs",
        json!({ "project": p.name, "withCode": true }),
    )
    .await;
    assert_eq!(done["kind"], "done");
    assert_ne!(git(&remote, &["rev-parse", "main"]), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tickets_agent_proposes_its_decisions_as_its_tickets() {
    let h = harness("mcp-docs-ticket-origin");
    let p = project(&h).await;
    let t = put(&h, ticket(&p, "DEM-1", "Le sien", Column::Doing, 1));
    let worker = put_agent(&h, &p, "dem-1-le-sien", 1);
    link(&h, &worker.id, &t.id);
    let c = as_agent(&h, &worker.id).await;
    // Whatever origin it gives: it comes from its ticket, which accepts it once validated.
    read(
        &c,
        "propose_decision",
        json!({ "project": p.id, "title": "Trois échéances", "origin": "brainstorm" }),
    )
    .await;
    let d = read(
        &c,
        "read_decision",
        json!({ "project": p.id, "id": "trois-echeances" }),
    )
    .await;
    assert_eq!(
        (d["origin"].clone(), d["originRef"].clone()),
        (json!("ticket"), json!("DEM-1"))
    );
    c.cancel().await.unwrap();
    // Claude outside Escouade, or an agent without a ticket, says where it comes from itself.
    let e = external(&h).await;
    read(
        &e,
        "propose_decision",
        json!({ "project": p.id, "title": "Tarif famille", "origin": "brainstorm", "originRef": "Tarif famille" }),
    )
    .await;
    let d = read(
        &e,
        "read_decision",
        json!({ "project": p.id, "id": "tarif-famille" }),
    )
    .await;
    assert_eq!(
        (d["origin"].clone(), d["originRef"].clone()),
        (json!("brainstorm"), json!("Tarif famille"))
    );
    e.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_find_read_and_cite_the_documents_and_the_drive_counts_it() {
    let h = harness("mcp-docs-cite");
    let p = project(&h).await;
    let agent = put_agent(&h, &p, "Alice", 1);
    let c = as_agent(&h, &agent.id).await;
    let docs = std::path::Path::new(&p.path).join(".escouade/documents/Comptes rendus");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("cr.pdf"),
        crate::docsextract_tests::pdf_bytes(&["Le bureau relance les adhesions a 30 jours."]),
    )
    .unwrap();
    // Found by its words (no vectors' model here), read whole: a PDF's text.
    let found = read(
        &c,
        "search_documents",
        json!({ "project": p.id, "query": "relance adhesions" }),
    )
    .await;
    assert_eq!(found["by"], "words");
    assert_eq!(found["documents"][0]["path"], "Comptes rendus/cr.pdf");
    let text = read(
        &c,
        "read_document",
        json!({ "project": p.id, "path": "Comptes rendus/cr.pdf" }),
    )
    .await;
    assert!(
        text["text"]
            .as_str()
            .unwrap()
            .contains("relance les adhesions"),
        "{text}"
    );
    // Cited by a page of the spec it wrote.
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "adhesion/relance", "body": "On relance a 30 jours.", "sources": ["documents/Comptes rendus/cr.pdf"] }),
    )
    .await;
    let all = read(&c, "list_documents", json!({ "project": p.id })).await;
    let f = &all["documents"][0];
    assert_eq!(
        (f["reads"].clone(), f["cited"].clone()),
        (json!(1), json!(1)),
        "{all}"
    );
    assert_eq!(all["folders"][0]["path"], "Comptes rendus");
    c.cancel().await.unwrap();
}

/// A discussion of the Docs tab (ESC-261) as an agent of the project.
fn put_discussion(
    h: &crate::core_tests::Harness,
    p: &crate::model::Project,
    kind: crate::discussion::Kind,
) -> String {
    let a = put_agent(h, p, "relances", 1);
    h.core.agent(&a.id).unwrap().lock().meta.discussion = Some(crate::discussion::Discussion {
        kind,
        subject: "Relance des adhésions".into(),
        ..Default::default()
    });
    a.id
}

#[tokio::test(flavor = "multi_thread")]
async fn a_discussion_proposes_pages_and_decisions_that_pour_writes() {
    let h = harness("mcp-docs-discussion");
    let p = project(&h).await;
    let spec = std::path::Path::new(&p.path).join(".escouade/spec");
    // A page that exists, written by the window.
    let e = external(&h).await;
    read(
        &e,
        "write_spec_page",
        json!({ "project": p.id, "path": "notifications/general", "title": "Notifications", "body": "Aucune notification n’est planifiée." }),
    )
    .await;
    read(
        &e,
        "propose_decision",
        json!({ "project": p.id, "title": "Une ancienne" }),
    )
    .await;
    e.cancel().await.unwrap();
    h.core
        .docs_accept_decision(&p.id, &crate::docs::Ref::Id("une-ancienne".into()))
        .await
        .unwrap();
    let before = std::fs::read_to_string(spec.join("notifications/general.md")).unwrap();
    let id = put_discussion(&h, &p, crate::discussion::Kind::Brainstorm);
    let c = as_agent(&h, &id).await;
    let made = read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "espace-adherent/relances", "title": "Relances", "body": "Trois relances par e-mail." }),
    )
    .await;
    assert_eq!(
        (
            made["proposed"].clone(),
            made["created"].clone(),
            made["added"].clone()
        ),
        (json!(true), json!(true), json!(1))
    );
    let changed = read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "notifications/general", "body": "Les relances d’adhésion sont planifiées chaque nuit." }),
    )
    .await;
    assert_eq!(
        (changed["added"].clone(), changed["removed"].clone()),
        (json!(1), json!(1))
    );
    // Nothing in the docs yet: the page as it was, no new page.
    assert!(!spec.join("espace-adherent/relances.md").exists());
    assert_eq!(
        std::fs::read_to_string(spec.join("notifications/general.md")).unwrap(),
        before
    );
    // It reads its page as it proposes it, and adds to it.
    let mine = read(
        &c,
        "read_spec_page",
        json!({ "project": p.id, "path": "espace-adherent/relances" }),
    )
    .await;
    assert_eq!(mine["body"], "Trois relances par e-mail.");
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "espace-adherent/relances", "body": "Coupables depuis l’espace.", "append": true }),
    )
    .await;
    let proposed = read(
        &c,
        "propose_decision",
        json!({ "project": p.id, "title": "Relances par e-mail seulement", "decision": "Pas de SMS.", "pages": ["espace-adherent/relances"] }),
    )
    .await;
    assert_eq!(
        (proposed["proposal"].clone(), proposed["proposed"].clone()),
        (json!(1), json!(true))
    );
    // Replacing is checked now: only an accepted decision.
    let (why, error) = call(
        &c,
        "replace_decision",
        json!({ "project": p.id, "number": 9, "title": "X" }),
    )
    .await;
    assert!(error, "{why}");
    read(
        &c,
        "replace_decision",
        json!({ "project": p.id, "number": 1, "title": "La nouvelle" }),
    )
    .await;
    c.cancel().await.unwrap();
    assert_eq!(
        h.core
            .docs_space(&p.id, None)
            .await
            .unwrap()
            .decisions()
            .len(),
        1
    );
    let d = h
        .core
        .agent(&id)
        .unwrap()
        .lock()
        .meta
        .discussion
        .clone()
        .unwrap();
    assert_eq!(d.pages.len(), 2);
    assert_eq!(
        d.page("espace-adherent/relances").unwrap().page.body,
        "Trois relances par e-mail.\n\nCoupables depuis l’espace."
    );
    assert_eq!(d.decisions.len(), 2);
    assert_eq!(d.decisions[0].origin, "brainstorm");
    assert_eq!(d.decisions[1].supersedes, vec![1]);

    // « Verser dans la spec »: written, the decisions proposed in the register, the discussion emptied.
    let poured = h.core.discussion_pour(&id).await.unwrap();
    assert_eq!(
        poured.pages,
        vec!["espace-adherent/relances", "notifications/general"]
    );
    let (two, three) = (
        crate::docs::Ref::Id("relances-par-e-mail-seulement".into()),
        crate::docs::Ref::Id("la-nouvelle".into()),
    );
    assert_eq!(poured.decisions, vec![two.clone(), three.clone()]);
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(space
        .spec_read("espace-adherent/relances")
        .unwrap()
        .body
        .ends_with("Coupables depuis l’espace."));
    assert_eq!(
        space.spec_read("notifications/general").unwrap().body,
        "Les relances d’adhésion sont planifiées chaque nuit."
    );
    let two_ref = two;
    let two = space.decision(&two_ref).unwrap();
    assert_eq!(two.number, None);
    assert_eq!(two.state, crate::docs::State::Proposed);
    assert_eq!(
        (two.origin.as_str(), two.origin_ref.as_str()),
        ("brainstorm", "Relance des adhésions")
    );
    // What it changed: its page's lines.
    assert_eq!(two.changes.len(), 1);
    assert_eq!(two.changes[0].page, "espace-adherent/relances");
    assert_eq!(space.decision(&three).unwrap().supersedes, vec![1]);
    let d = h
        .core
        .agent(&id)
        .unwrap()
        .lock()
        .meta
        .discussion
        .clone()
        .unwrap();
    assert!(d.pages.is_empty() && d.decisions.is_empty());
    assert_eq!(d.poured.unwrap().pages.len(), 2);
    // Nothing more to pour; nothing committed.
    assert!(h.core.discussion_pour(&id).await.is_err());
    let repo = std::path::Path::new(&p.path);
    assert_eq!(git(repo, &["log", "--oneline", "--all"]).lines().count(), 1);
    // What was poured is to plan (ESC-262), the decision the window proposed before too.
    let v = h.core.docs_planning(&p.id).await.unwrap();
    assert_eq!(
        v.pages,
        vec!["espace-adherent/relances", "notifications/general"]
    );
    assert_eq!(v.decisions, vec![two_ref, three]);
    // « Planifier » on the discussion: its changes made into tickets, the plan kept on it.
    let mut board = h.core.project(&p.id).unwrap().board;
    board.autopilot = false;
    h.core.board_set(&p.id, board).unwrap();
    let plan = h.core.docs_plan(&p.id, Some(&id)).await.unwrap();
    assert_eq!(plan.made.len(), 2);
    assert_eq!(plan.discussion.as_deref(), Some(id.as_str()));
    let d = h.core.agent(&id).unwrap().lock().meta.discussion.clone();
    assert_eq!(d.unwrap().planned.unwrap().id, plan.id);
    assert!(h.core.docs_plan(&p.id, Some(&id)).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proposal_taken_out_is_not_poured() {
    let h = harness("mcp-docs-discussion-drop");
    let p = project(&h).await;
    let id = put_discussion(&h, &p, crate::discussion::Kind::Discussion);
    let c = as_agent(&h, &id).await;
    for path in ["a/un", "a/deux"] {
        read(
            &c,
            "write_spec_page",
            json!({ "project": p.id, "path": path, "body": "x" }),
        )
        .await;
    }
    read(
        &c,
        "propose_decision",
        json!({ "project": p.id, "title": "D" }),
    )
    .await;
    c.cancel().await.unwrap();
    h.core.discussion_drop(&id, Some("a/un"), Some(1)).unwrap();
    let poured = h.core.discussion_pour(&id).await.unwrap();
    assert_eq!(
        (poured.pages, poured.decisions),
        (vec!["a/deux".to_string()], vec![])
    );
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(space.spec_read("a/un").is_err());
    assert!(space.decisions().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gap_a_discussion_was_opened_on_goes_when_its_page_is_poured() {
    let h = harness("mcp-docs-discussion-gap");
    let p = project(&h).await;
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    let gap = |t: &str| crate::docs::Gap {
        text: t.into(),
        code: vec!["src/payment.ts".into()],
    };
    space
        .spec_write("paiement/echeances", |pg| {
            pg.body = "Trois échéances.".into();
            pg.gaps = vec![gap("Le code en accepte quatre."), gap("Un autre écart.")];
        })
        .unwrap();
    let id = put_discussion(&h, &p, crate::discussion::Kind::Discussion);
    h.core
        .discussion_edit(&id, |d| {
            d.settles = Some(crate::discussion::Settles {
                page: "paiement/echeances".into(),
                gap: gap("Le code en accepte quatre."),
            })
        })
        .unwrap();
    let c = as_agent(&h, &id).await;
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "paiement/echeances", "body": "Quatre échéances." }),
    )
    .await;
    c.cancel().await.unwrap();
    h.core.discussion_pour(&id).await.unwrap();
    let page = space.spec_read("paiement/echeances").unwrap();
    assert_eq!(page.body, "Quatre échéances.");
    // The one it settles is gone; the other stays.
    assert_eq!(page.gaps, vec![gap("Un autre écart.")]);
    let d = h
        .core
        .agent(&id)
        .unwrap()
        .lock()
        .meta
        .discussion
        .clone()
        .unwrap();
    assert!(d.settles.is_none());
}
