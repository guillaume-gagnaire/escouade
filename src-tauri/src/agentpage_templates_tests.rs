//! Tests of the templates of the agents' pages (`agentpage_templates`, ESC-674): their schemas
//! checked by the core (every problem told at once, by its path), the agent's text escaped (no
//! HTML of it ever reaches the page), the Markdown of its own, the anchors for `drive_browser`, the
//! files served under `_t/` and the embedded Mermaid. What a real webview draws of them is told by
//! `tests/e2e/agentpage-templates.spec.ts`.

use crate::agentpage::{locate, Route, Shelter};
use crate::agentpage_templates::*;
use crate::i18n::Lang;
use serde_json::{json, Value};

const TRAP: &str = "<img src=x onerror=alert(1)><script>alert(2)</script>\"'&";

fn look(lang: Lang) -> Look {
    Look {
        lang,
        theme: Theme::Dark,
        tint: None,
        choice: None,
    }
}

fn review() -> Value {
    json!({
        "subject": { "ticket": "POR-41", "branch": "ticket/por-41", "target": "main", "loop": "2/5" },
        "facts": { "files": 6, "added": 214, "removed": 38, "tests": { "passed": 48, "total": 48 } },
        "criteria": [
            { "text": "La liste montre 25 adhérents par page", "state": "met", "md": "Vérifié par `users.spec.ts`." },
            { "text": "Suivante et précédente au clavier", "state": "met" },
            { "text": "Le filtre est gardé", "state": "partial", "md": "Le **tri** n'est pas gardé." }
        ],
        "changes": [
            { "path": "src/lib/pagination.ts", "added": 58, "removed": 0, "status": "added",
              "diff": "@@ -0,0 +1,2 @@\n+export const PAGE = 25;\n+export function page() {}" },
            { "path": "tests/users.spec.ts", "added": 44, "removed": 9 }
        ],
        "risks": [{ "md": "Le tri n'est pas gardé dans l'adresse.", "level": "medium" }],
        "sections": [{ "id": "perf", "title": "Performances", "md": "Rien ne change." }]
    })
}

fn diagram() -> Value {
    json!({ "kind": "mermaid", "source": "flowchart LR\n  A[Agent] --> B{Escouade}\n  B --> C[Page]", "md": "La *légende*." })
}

fn compare() -> Value {
    json!({
        "options": [
            { "id": "a", "title": "Les repères", "md": "Côte à côte.", "images": ["shots/a.png"], "pros": ["Lisible"], "cons": ["Étroit à 1024"] },
            { "id": "b", "title": "La visite", "images": [{ "dark": "shots/b.png", "light": "shots/b-clair.png" }] },
            { "id": "c", "title": "L'onglet" }
        ],
        "criteria": ["Le pilotage se lit", "La conversation reste entière"],
        "recommend": "a"
    })
}

fn problems(t: Template, data: &Value) -> Vec<Problem> {
    check(t, data, Lang::En).expect_err("refused")
}

fn at(p: &[Problem]) -> Vec<&str> {
    p.iter().map(|p| p.at.as_str()).collect()
}

#[test]
fn the_three_templates_are_named() {
    for t in Template::ALL {
        assert_eq!(Template::parse(t.name()), Some(t));
    }
    assert_eq!(Template::parse("html"), None);
    assert_eq!(Template::parse("Review"), None);
}

#[test]
fn good_data_passes_for_each_template() {
    check(Template::Review, &review(), Lang::Fr).unwrap();
    check(Template::Diagram, &diagram(), Lang::Fr).unwrap();
    check(Template::Compare, &compare(), Lang::Fr).unwrap();
    // A lot: its tickets, and items tied to one.
    let lot = json!({
        "subject": { "tickets": ["POR-41", "POR-42"], "target": "main" },
        "sections": [{ "id": "por-41", "title": "POR-41", "md": "…", "ticket": "POR-41" }]
    });
    check(Template::Review, &lot, Lang::Fr).unwrap();
}

#[test]
fn a_field_too_many_or_badly_typed_is_refused_all_at_once_by_its_path() {
    let mut d = review();
    d["verdict"] = json!("fusionner");
    d["criteria"][2]["state"] = json!("done");
    d["criteria"][0]["text"] = json!(12);
    d["facts"]["files"] = json!("six");
    d["changes"][1]["html"] = json!("<b>x</b>");
    d["subject"]["loop"] = json!(null);
    let p = problems(Template::Review, &d);
    let paths = at(&p);
    for want in [
        "verdict",
        "criteria.2.state",
        "criteria.0.text",
        "facts.files",
        "changes.1.html",
    ] {
        assert!(paths.contains(&want), "{want} in {paths:?}");
    }
    assert_eq!(p.len(), 5, "{p:?}");
    let told = tell(&p);
    assert!(told.contains("verdict: unknown field (fields allowed: subject, facts, criteria, changes, risks, sections)"), "{told}");
    assert!(
        told.contains("criteria.2.state: one of: met, partial, unmet, unknown"),
        "{told}"
    );
    assert!(told.contains("criteria.0.text: expected: text"), "{told}");
}

#[test]
fn what_is_missing_or_out_of_bounds_is_refused() {
    assert_eq!(at(&problems(Template::Review, &json!({}))), ["subject"]);
    assert_eq!(at(&problems(Template::Review, &json!([]))), ["."]);
    assert_eq!(
        at(&problems(
            Template::Diagram,
            &json!({ "kind": "d2", "source": "a" })
        )),
        ["kind"]
    );
    assert_eq!(
        at(&problems(Template::Diagram, &json!({ "source": "a" }))),
        ["kind"]
    );
    // Empty, too long, a control character, a negative count.
    let p = problems(
        Template::Review,
        &json!({
            "subject": { "ticket": "", "branch": "a".repeat(201) },
            "criteria": [{ "text": "a\u{0007}b", "state": "met" }],
            "facts": { "added": -1 }
        }),
    );
    assert_eq!(
        at(&p),
        [
            "subject.ticket",
            "subject.branch",
            "facts.added",
            "criteria.0.text"
        ]
    );
    // A key that is not one, an id that is not one.
    let p = problems(
        Template::Review,
        &json!({ "subject": { "ticket": "POR 41" }, "sections": [{ "id": "Perf!", "title": "t", "md": "m" }] }),
    );
    assert_eq!(at(&p), ["subject.ticket", "sections.0.id"]);
    // Too few, too many options.
    let p = problems(
        Template::Compare,
        &json!({ "options": [{ "id": "a", "title": "A" }] }),
    );
    assert_eq!(at(&p), ["options"]);
    let nine: Vec<Value> = (0..9)
        .map(|i| json!({ "id": format!("o{i}"), "title": "x" }))
        .collect();
    assert_eq!(
        at(&problems(Template::Compare, &json!({ "options": nine }))),
        ["options"]
    );
}

#[test]
fn the_sizes_are_bounded() {
    let big = "x".repeat(DATA_MAX);
    let p = problems(
        Template::Diagram,
        &json!({ "kind": "mermaid", "source": big }),
    );
    assert_eq!(at(&p), ["."]);
    assert!(p[0].what.contains("262144 at most"), "{p:?}");
    let source = "a".repeat(SOURCE_MAX + 1);
    assert_eq!(
        at(&problems(
            Template::Diagram,
            &json!({ "kind": "mermaid", "source": source })
        )),
        ["source"]
    );
    // A diff of 200 KB passes, one byte more does not (in a data of 256 KB at most).
    let mut d = review();
    d["changes"][0]["diff"] = json!("+".repeat(DIFF_MAX));
    check(Template::Review, &d, Lang::Fr).unwrap();
    d["changes"][0]["diff"] = json!("+".repeat(DIFF_MAX + 1));
    assert_eq!(at(&problems(Template::Review, &d)), ["changes.0.diff"]);
}

#[test]
fn ids_are_unique_and_a_recommendation_names_an_option() {
    let mut d = compare();
    d["options"][1]["id"] = json!("a");
    d["recommend"] = json!("z");
    let p = problems(Template::Compare, &d);
    assert_eq!(at(&p), ["options.1.id", "recommend"]);
    let mut r = review();
    r["sections"] =
        json!([{ "id": "s", "title": "1", "md": "1" }, { "id": "s", "title": "2", "md": "2" }]);
    assert_eq!(at(&problems(Template::Review, &r)), ["sections.1.id"]);
}

#[test]
fn an_image_is_a_relative_path_to_an_image_never_an_svg() {
    for bad in [
        "../x.png",
        "/etc/x.png",
        "C:/x.png",
        "a//b.png",
        "a\\b.png",
        "shots/x.svg",
        "x.html",
        "a/./b.png",
        "x.png?y",
    ] {
        let mut d = compare();
        d["options"][0]["images"] = json!([bad]);
        assert_eq!(
            at(&problems(Template::Compare, &d)),
            ["options.0.images.0"],
            "{bad}"
        );
    }
    let mut d = compare();
    d["options"][1]["images"] = json!([{ "dark": "b.png", "light": "../b.png" }]);
    assert_eq!(
        at(&problems(Template::Compare, &d)),
        ["options.1.images.0.light"]
    );
    d["options"][1]["images"] = json!([{ "dark": "b.png" }]);
    assert_eq!(
        at(&problems(Template::Compare, &d)),
        ["options.1.images.0.light"]
    );
    assert_eq!(
        images(Template::Compare, &compare()),
        ["shots/a.png", "shots/b.png", "shots/b-clair.png"]
    );
}

#[test]
fn the_problems_are_told_in_the_language_asked() {
    let fr = check(Template::Diagram, &json!({ "kind": "mermaid" }), Lang::Fr).unwrap_err();
    assert_eq!(fr[0].what, "manquant");
    let en = check(Template::Diagram, &json!({ "kind": "mermaid" }), Lang::En).unwrap_err();
    assert_eq!(en[0].what, "missing");
}

/// Every value of `data` set to the trap, wherever it is text.
fn trapped(v: &Value, keep: &[&str], key: &str) -> Value {
    match v {
        Value::String(_) if keep.contains(&key) => v.clone(),
        Value::String(_) => json!(TRAP),
        Value::Array(a) => Value::Array(a.iter().map(|x| trapped(x, keep, key)).collect()),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| (k.clone(), trapped(x, keep, k)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// No markup of the trap: what the agent wrote is text, escaped, wherever it lands.
fn assert_escaped(html: &str) {
    assert!(!html.contains("<img src=x"), "{html}");
    assert!(!html.contains("<script>alert"), "{html}");
    assert!(
        !html.contains("onerror=alert(1)>"),
        "an attribute was left open"
    );
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;&lt;script&gt;alert(2)&lt;/script&gt;&quot;&#39;&amp;"), "{html}");
}

#[test]
fn a_trapped_name_stays_text_in_every_template() {
    let keep = [
        "loop",
        "dark",
        "light",
        "state",
        "status",
        "level",
        "kind",
        "id",
        "ticket",
        "tickets",
        "recommend",
        "images",
    ];
    for (t, d) in [
        (Template::Review, review()),
        (Template::Diagram, diagram()),
        (Template::Compare, compare()),
    ] {
        let d = trapped(&d, &keep, "");
        let r = render(t, TRAP, &d, &look(Lang::Fr))
            .unwrap_or_else(|p| panic!("{}: {}", t.name(), tell(&p)));
        assert_escaped(&r.html);
        // The page's own scripts alone, its own files alone.
        let scripts = r.html.matches("<script").count();
        assert_eq!(
            scripts,
            if t == Template::Diagram { 2 } else { 1 },
            "{}",
            t.name()
        );
        assert!(!r.html.contains("http"), "nothing reaches out: {}", r.html);
    }
    // A request's words too (AP-4 gives them from the box).
    let mut l = look(Lang::En);
    l.choice = Some(Choice {
        key: "a-31".into(),
        title: TRAP.into(),
        options: vec![ChoiceOption {
            id: "a".into(),
            label: TRAP.into(),
            description: Some(TRAP.into()),
        }],
        recommend: Some("a".into()),
        multi: false,
        answered: vec![],
    });
    let r = render(Template::Review, "t", &review(), &l).unwrap();
    assert_escaped(&r.html);
    // An option id that is not one is no attribute's end.
    l.choice.as_mut().unwrap().options[0].id = "a\" onclick=\"x".into();
    let r = render(Template::Review, "t", &review(), &l).unwrap();
    assert!(!r.html.contains("onclick=\"x"), "{}", r.html);
    assert!(r.html.contains("id=\"option-o0\""));
}

#[test]
fn a_diagram_source_is_text_that_closes_nothing() {
    let d = json!({ "kind": "mermaid", "source": "flowchart LR\nA[\"</pre><script>alert(1)</script>\"] --> B" });
    let r = render(Template::Diagram, "Schéma", &d, &look(Lang::Fr)).unwrap();
    assert!(
        r.html
            .contains("&lt;/pre&gt;&lt;script&gt;alert(1)&lt;/script&gt;"),
        "{}",
        r.html
    );
    assert_eq!(r.html.matches("</pre>").count(), 1);
    assert!(r.html.contains("<script src=\"_t/mermaid.js\"></script>"));
}

#[test]
fn the_title_is_one_line_of_120_characters_at_most() {
    let l = look(Lang::Fr);
    assert!(render(Template::Diagram, "", &diagram(), &l).is_err());
    assert!(render(Template::Diagram, "a\nb", &diagram(), &l).is_err());
    assert!(render(Template::Diagram, &"é".repeat(121), &diagram(), &l).is_err());
    assert!(render(Template::Diagram, &"é".repeat(120), &diagram(), &l).is_ok());
}

#[test]
fn the_markdown_writes_its_few_tags_alone() {
    assert_eq!(md_html("Un **gras**, de l'*italique* et `du <code>`."),
        "<p>Un <strong>gras</strong>, de l&#39;<em>italique</em> et <code>du &lt;code&gt;</code>.</p>");
    assert_eq!(
        md_html("- un\n- deux\n\n1. trois"),
        "<ul><li>un</li><li>deux</li></ul><ol><li>trois</li></ol>"
    );
    assert_eq!(
        md_html("```\n<b>x</b>\n```"),
        "<pre><code>&lt;b&gt;x&lt;/b&gt;</code></pre>"
    );
    assert_eq!(
        md_html("## Titre"),
        "<p class=\"h\"><strong>Titre</strong></p>"
    );
    assert_eq!(md_html("> cité"), "<blockquote><p>cité</p></blockquote>");
    assert_eq!(md_html("ligne\nsuite"), "<p>ligne suite</p>");
    assert_eq!(
        md_html("[la doc](https://escouade.dev/docs?a=1&b=\"2\")"),
        "<p><a href=\"https://escouade.dev/docs?a=1&amp;b=&quot;2&quot;\" rel=\"noreferrer noopener\">la doc</a></p>"
    );
    // A link but to the web is its text alone.
    assert_eq!(md_html("[clic](javascript:alert(1))"), "<p>clic</p>");
    assert_eq!(md_html("[clic](data:text/html,x)"), "<p>clic</p>");
    // HTML is text; snake_case stays whole; an unclosed mark is a character.
    assert_eq!(
        md_html("<b onclick=x>a</b>"),
        "<p>&lt;b onclick=x&gt;a&lt;/b&gt;</p>"
    );
    assert_eq!(md_html("a_b_c et **x"), "<p>a_b_c et **x</p>");
}

#[test]
fn the_anchors_are_stable_and_say_where_a_key_is() {
    let d = review();
    let r = render(Template::Review, "Revue", &d, &look(Lang::Fr)).unwrap();
    for (key, sel) in [
        ("criteria.2", "#crit-3"),
        ("criteria", "#criteria"),
        ("changes.0", "#change-1"),
        ("risks.0", "#risk-1"),
        ("sections.0", "#sec-perf"),
        ("sections.perf", "#sec-perf"),
        ("subject", "#subject"),
        ("facts", "#facts"),
    ] {
        assert_eq!(
            anchor(Template::Review, &d, None, key).unwrap(),
            sel,
            "{key}"
        );
        let id = sel.trim_start_matches('#');
        assert!(r.html.contains(&format!("id=\"{id}\"")), "{id} in the page");
    }
    assert_eq!(r.anchors.len(), anchors(Template::Review, &d, None).len());
    let e = anchor(Template::Review, &d, None, "criteria.9").unwrap_err();
    assert!(e.contains("criteria.2"), "{e}");
    // A diagram's node, by its id in the source.
    assert_eq!(
        anchor(Template::Diagram, &diagram(), None, "nodes.B").unwrap(),
        "#diagram [data-node=\"B\"]"
    );
    assert!(anchor(Template::Diagram, &diagram(), None, "nodes.B\"]").is_err());
    assert_eq!(
        anchor(Template::Diagram, &diagram(), None, "legend").unwrap(),
        "#legend"
    );
    // A plate, by its place or its id; a criterion of the comparison.
    assert_eq!(
        anchor(Template::Compare, &compare(), None, "options.1").unwrap(),
        "#option-b"
    );
    assert_eq!(
        anchor(Template::Compare, &compare(), None, "options.c").unwrap(),
        "#option-c"
    );
    assert_eq!(
        anchor(Template::Compare, &compare(), None, "criteria.1").unwrap(),
        "#ccrit-2"
    );
    // A lot's tabs, declared for `tab`.
    let lot = json!({ "subject": { "tickets": ["POR-41", "POR-42"] } });
    assert_eq!(
        anchor(Template::Review, &lot, None, "tabs.POR-42").unwrap(),
        "#tab-2"
    );
    let html = render(Template::Review, "Lot", &lot, &look(Lang::Fr))
        .unwrap()
        .html;
    assert!(html.contains("role=\"tablist\"") && html.contains("id=\"tab-2\" role=\"tab\""));
}

#[test]
fn the_page_speaks_the_apps_language_and_wears_its_theme() {
    let fr = render(Template::Review, "Revue", &review(), &look(Lang::Fr))
        .unwrap()
        .html;
    assert!(fr.contains("<html lang=\"fr\" data-theme=\"dark\" data-template=\"review\">"));
    assert!(fr.contains("Critères d&#39;acceptation") || fr.contains("Critères d'acceptation"));
    assert!(fr.contains("À moitié") && fr.contains("6 fichiers"));
    let mut l = look(Lang::En);
    l.theme = Theme::Light;
    l.tint = Some("#35c1bd".into());
    let en = render(Template::Review, "Review", &review(), &l)
        .unwrap()
        .html;
    assert!(en.contains(
        "<html lang=\"en\" data-theme=\"light\" data-template=\"review\" style=\"--p: #35c1bd\">"
    ));
    assert!(en.contains("Acceptance criteria") && en.contains("Partly") && en.contains("6 files"));
    l.tint = Some("red;}body{display:none".into());
    let en = render(Template::Review, "Review", &review(), &l)
        .unwrap()
        .html;
    assert!(!en.contains("style=\"--p"));
}

#[test]
fn a_long_diff_is_folded() {
    let mut d = review();
    let long: Vec<String> = (0..=DIFF_OPEN_LINES)
        .map(|i| format!("+line {i}"))
        .collect();
    d["changes"][1]["diff"] = json!(long.join("\n"));
    let html = render(Template::Review, "Revue", &d, &look(Lang::Fr))
        .unwrap()
        .html;
    assert_eq!(
        html.matches("<details open>").count(),
        1,
        "the short one open"
    );
    assert!(html.contains("<details><summary>") && html.contains("401 lignes"));
    assert!(html.contains("<span class=\"add\">+export const PAGE = 25;</span>"));
    assert!(html.contains("<span class=\"hunk\">@@ -0,0 +1,2 @@</span>"));
}

#[test]
fn the_files_of_the_templates_are_served_to_a_page_alone() {
    assert_eq!(
        locate("escouade-page://p-12.localhost/_t/page.js"),
        Some(("p-12".into(), Route::Template("page.js".into())))
    );
    let dir = std::env::temp_dir().join(format!("agentpage-t-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    let s = Shelter::default();
    s.put("p-12", dir.clone()).unwrap();
    let uri = "escouade-page://p-12.localhost/_t/page.css";
    let ok = s.serve("agentpage-p-12", "GET", uri, false);
    assert_eq!(ok.status(), 200);
    assert_eq!(ok.headers()["Content-Type"], "text/css; charset=utf-8");
    assert!(ok.headers()["Content-Security-Policy"]
        .to_str()
        .unwrap()
        .contains("default-src 'none'"));
    assert!(String::from_utf8_lossy(ok.body()).contains("--panel"));
    // Not to another page, the window, by POST, nor a file the templates do not have.
    assert_eq!(s.serve("agentpage-p-13", "GET", uri, false).status(), 404);
    assert_eq!(s.serve("main", "GET", uri, false).status(), 404);
    assert_eq!(s.serve("agentpage-p-12", "POST", uri, false).status(), 404);
    assert_eq!(
        s.serve(
            "agentpage-p-12",
            "GET",
            "escouade-page://p-12.localhost/_t/../Cargo.toml",
            false
        )
        .status(),
        404
    );
    assert_eq!(
        s.serve(
            "agentpage-p-12",
            "GET",
            "escouade-page://p-12.localhost/_t/x.js",
            false
        )
        .status(),
        404
    );
    // Nor to a page the shelter does not serve.
    assert_eq!(
        s.serve(
            "agentpage-p-14",
            "GET",
            "escouade-page://p-14.localhost/_t/page.css",
            false
        )
        .status(),
        404
    );
    let js = s.serve(
        "agentpage-p-12",
        "GET",
        "escouade-page://p-12.localhost/_t/mermaid.js",
        false,
    );
    assert_eq!(js.status(), 200);
    assert!(js.body().len() > 1_000_000);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_embedded_mermaid_is_the_one_checked() {
    use sha2::{Digest, Sha256};
    let bytes = mermaid();
    let hex: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hex, MERMAID_SHA256,
        "Mermaid {MERMAID_VERSION}: scripts/mermaid-vendor.sh says its digest"
    );
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(400)]).to_string();
    assert!(!text.is_empty());
    // Strict by the template, never loose.
    let js = std::str::from_utf8(asset("page.js").unwrap().1.as_ref())
        .unwrap()
        .to_string();
    assert!(js.contains("securityLevel: 'strict'"));
    assert!(!js.contains("innerHTML = source") && !js.contains("loose"));
}
