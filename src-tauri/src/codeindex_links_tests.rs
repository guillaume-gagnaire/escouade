//! The resolution of calls (ESC-297, IDX-2): the tests of tokensave's resolution, ported
//! (github.com/aovestdipaperino/tokensave, commit 580936b, MIT, © 2025 Enzo Lombardi:
//! `tests/resolution_test.rs`, `tests/ambiguous_calls_test.rs`, `tests/ts_phantom_call_edges_test.rs`,
//! `tests/python_phantom_call_edges_test.rs`, `tests/resolution_order_independence_test.rs`,
//! `tests/incremental_resolution_test.rs`, and the unit tests of `src/resolution/js_specifier.rs`
//! and `src/go_import.rs`), then ours: the `forget`s of this repository, a tie told apart, a file
//! changed read alone, a worktree.

use crate::codeindex::resolution::*;
use crate::codeindex::{Service, Target, Wanted};
use crate::fence::Fence;
use crate::paths::{test_dir, DataDir};
use std::path::{Path, PathBuf};

// ---- the resolver, alone (`tests/resolution_test.rs`) ----

fn node(id: i64, name: &str, qname: &str, kind: &str, path: &str, line: u32, vis: bool) -> Node {
    Node {
        id,
        name: name.into(),
        qname: qname.into(),
        kind: kind.into(),
        path: path.into(),
        line,
        vis,
        parent: String::new(),
        ty: String::new(),
    }
}

fn call(id: i64, from: i64, name: &str, path: &str, line: u32) -> Uref {
    Uref {
        id,
        from: Some(from),
        kind: "call".into(),
        name: name.into(),
        path: path.into(),
        line,
        col: 12,
        via: 0,
    }
}

/// `setup_db_with_nodes`: `helper` in `src/utils.rs`, `main` in `src/main.rs`.
fn helper_and_main() -> Vec<Node> {
    vec![
        node(
            1,
            "helper",
            "src/utils.rs::helper",
            "function",
            "src/utils.rs",
            1,
            true,
        ),
        node(
            2,
            "main",
            "src/main.rs::main",
            "function",
            "src/main.rs",
            1,
            false,
        ),
    ]
}

fn resolver(nodes: &[Node]) -> Resolver<'_> {
    Resolver::new(nodes, &Default::default(), &[])
}

#[test]
fn test_resolve_exact_name_match() {
    let nodes = helper_and_main();
    let r = resolver(&nodes);
    let got = r
        .resolve_one(&call(1, 2, "helper", "src/main.rs", 3))
        .expect("should resolve the helper reference");
    assert!(got.confidence >= 0.7, "{}", got.confidence);
    assert_eq!(got.to, 1);
}

#[test]
fn test_resolve_qualified_name_match() {
    let nodes = helper_and_main();
    let got = resolver(&nodes)
        .resolve_one(&call(1, 2, "src/utils.rs::helper", "src/main.rs", 3))
        .expect("should resolve via qualified name match");
    assert!((got.confidence - 0.95).abs() < f64::EPSILON);
    assert_eq!(got.by, "qualified-match");
}

#[test]
fn test_resolve_all() {
    let nodes = helper_and_main();
    let out = resolver(&nodes).resolve_all(&[call(1, 2, "helper", "src/main.rs", 3)]);
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.unresolved, 0);
}

#[test]
fn test_unresolvable_reference() {
    let nodes = helper_and_main();
    assert!(resolver(&nodes)
        .resolve_one(&call(1, 2, "nonexistent", "src/main.rs", 5))
        .is_none());
}

#[test]
fn test_unresolvable_in_resolve_all() {
    let nodes = helper_and_main();
    let out = resolver(&nodes).resolve_all(&[
        call(1, 2, "helper", "src/main.rs", 3),
        call(2, 2, "nonexistent", "src/main.rs", 5),
    ]);
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.unresolved, 1);
}

#[test]
fn test_multiple_candidates_best_match_scoring() {
    let nodes = vec![
        node(
            1,
            "process",
            "src/main.rs::process",
            "function",
            "src/main.rs",
            10,
            false,
        ),
        node(
            2,
            "process",
            "src/other.rs::process",
            "function",
            "src/other.rs",
            1,
            true,
        ),
        node(
            3,
            "run",
            "src/main.rs::run",
            "function",
            "src/main.rs",
            1,
            false,
        ),
    ];
    let got = resolver(&nodes)
        .resolve_one(&call(1, 3, "process", "src/main.rs", 3))
        .expect("should resolve with multiple candidates");
    assert_eq!(got.to, 1, "should prefer the same-file candidate");
    assert!((got.confidence - 0.7).abs() < f64::EPSILON);
}

#[test]
fn test_resolve_all_empty_input() {
    let nodes = helper_and_main();
    let out = resolver(&nodes).resolve_all(&[]);
    assert!(out.resolved.is_empty() && out.ambiguous.is_empty());
    assert_eq!(out.unresolved, 0);
}

/// #141: `Self::helper` is not pre-filtered as hopeless, its simple name is known.
#[test]
fn test_resolve_all_self_qualified_call_not_dropped() {
    let nodes = helper_and_main();
    let out = resolver(&nodes).resolve_all(&[call(1, 2, "Self::helper", "src/main.rs", 3)]);
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].to, 1);
}

/// #141: `obj.helper` falls back to the trailing method name.
#[test]
fn test_resolve_all_dotted_method_call() {
    let nodes = helper_and_main();
    let out = resolver(&nodes).resolve_all(&[call(1, 2, "obj.helper", "src/main.rs", 3)]);
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].to, 1);
}

#[test]
fn test_cross_language_single_candidate_still_penalised() {
    let nodes = vec![
        node(
            1,
            "helper",
            "tools/gen.py::helper",
            "function",
            "tools/gen.py",
            1,
            true,
        ),
        node(
            2,
            "Step",
            "mover.cpp::Step",
            "function",
            "src/mover.cpp",
            1,
            true,
        ),
    ];
    let got = resolver(&nodes)
        .resolve_one(&call(1, 2, "helper", "src/mover.cpp", 12))
        .expect("a candidate is found");
    assert!(got.confidence < CONFIDENCE_FLOOR, "{}", got.confidence);
}

/// The weights of `score_candidate`, one by one.
#[test]
fn score_candidate_weighs_file_line_folder_language_pub_callable_and_import() {
    let near = node(1, "f", "a/b/x.rs::f", "function", "a/b/x.rs", 10, false);
    let far = node(2, "f", "a/b/x.rs::f", "function", "a/b/x.rs", 400, false);
    let other_dir = node(3, "f", "a/c/y.rs::f", "function", "a/c/y.rs", 1, true);
    let ts = node(4, "f", "a/b/z.ts::f", "function", "a/b/z.ts", 1, false);
    let field = node(5, "f", "a/b/w.rs::f", "field", "a/b/w.rs", 1, false);
    let nodes = vec![
        near.clone(),
        far.clone(),
        other_dir.clone(),
        ts.clone(),
        field.clone(),
    ];
    let imports = [Import {
        path: "a/b/x.rs",
        name: "crate::c::f",
    }];
    let r = Resolver::new(&nodes, &Default::default(), &imports);
    let u = call(1, 9, "f", "a/b/x.rs", 12);
    // Same file +100, line proximity 20 - 2/10, Rust +50, callable +25, `f` imported +30.
    assert_eq!(r.score_candidate(&u, &near), 100 + 20 + 50 + 25 + 30);
    assert_eq!(r.score_candidate(&u, &far), 100 + 50 + 25 + 30);
    // Two shared segments (a, then not b/c) +5, pub +10.
    assert_eq!(r.score_candidate(&u, &other_dir), 5 + 50 + 10 + 25 + 30);
    // Another language -80.
    assert_eq!(r.score_candidate(&u, &ts), 10 - 80 + 25 + 30);
    // Not callable: no +25.
    assert_eq!(r.score_candidate(&u, &field), 10 + 50 + 30);
    assert_eq!(
        path_proximity("a/b/c/d/e/f/g/h/i/j", "a/b/c/d/e/f/g/h/i/j"),
        40
    );
}

#[test]
fn a_tie_is_never_guessed_but_told_with_its_candidates_by_id() {
    let nodes = vec![
        node(
            7,
            "dispose",
            "src/a.ts::A::dispose",
            "method",
            "src/a.ts",
            2,
            true,
        ),
        node(
            3,
            "dispose",
            "src/b.ts::B::dispose",
            "method",
            "src/b.ts",
            2,
            true,
        ),
        node(9, "go", "src/c.ts::go", "function", "src/c.ts", 1, true),
    ];
    let out = resolver(&nodes).resolve_all(&[call(1, 9, "thing.dispose", "src/c.ts", 3)]);
    assert!(out.resolved.is_empty());
    assert_eq!(out.ambiguous.len(), 1);
    assert_eq!(out.ambiguous[0].candidates, vec![3, 7]);
}

#[test]
fn the_confidence_floor_and_the_names_too_common_across_files() {
    assert_eq!(CONFIDENCE_FLOOR, 0.6);
    for n in ["new", "clone", "unwrap", "get", "run", "lock", "Some"] {
        assert!(CROSS_FILE_BLOCKLIST.contains(&n), "{n}");
    }
    // A blocklisted name binds in its own file only.
    let nodes = vec![
        node(1, "get", "src/a.rs::get", "function", "src/a.rs", 1, true),
        node(2, "main", "src/b.rs::main", "function", "src/b.rs", 1, true),
        node(3, "get", "src/b.rs::get", "function", "src/b.rs", 9, false),
    ];
    let r = resolver(&nodes);
    let got = r.resolve_one(&call(1, 2, "get", "src/b.rs", 3)).unwrap();
    assert_eq!((got.to, got.by.as_str()), (3, "same-file-blocklist"));
    assert!(r.resolve_one(&call(2, 2, "get", "src/c.rs", 3)).is_none());
}

#[test]
fn a_bare_name_in_python_js_or_ts_needs_evidence_of_an_import() {
    let nodes = vec![node(
        1,
        "helper",
        "lib/h.py::helper",
        "function",
        "lib/h.py",
        1,
        true,
    )];
    let none = resolver(&nodes);
    assert!(none
        .resolve_one(&call(1, 2, "helper", "app/main.py", 3))
        .is_none());
    let imports = [Import {
        path: "app/main.py",
        name: "lib.h.helper",
    }];
    let with = Resolver::new(&nodes, &Default::default(), &imports);
    assert!(with
        .resolve_one(&call(1, 2, "helper", "app/main.py", 3))
        .is_some());
    // Rust is not gated: its module system is not what the gate sees.
    let rs = vec![node(
        1,
        "helper",
        "lib/h.rs::helper",
        "function",
        "lib/h.rs",
        1,
        true,
    )];
    assert!(resolver(&rs)
        .resolve_one(&call(1, 2, "helper", "app/main.rs", 3))
        .is_some());
}

#[test]
fn a_typed_call_drops_its_name_based_siblings_and_their_tie() {
    let mut nodes = vec![
        node(
            1,
            "forget",
            "src/rt.rs::Runtime::forget",
            "method",
            "src/rt.rs",
            5,
            true,
        ),
        node(
            2,
            "forget",
            "src/wsl.rs::forget",
            "function",
            "src/wsl.rs",
            1,
            true,
        ),
        node(
            3,
            "call",
            "src/cmd.rs::call",
            "function",
            "src/cmd.rs",
            1,
            true,
        ),
    ];
    nodes[0].parent = "Runtime".into();
    let r = resolver(&nodes);
    let typed = Uref {
        via: 1,
        ..call(1, 3, "Runtime::forget", "src/cmd.rs", 4)
    };
    let out = r.resolve_all(&[typed, call(2, 3, "forget", "src/cmd.rs", 4)]);
    assert_eq!(out.resolved.len(), 1, "{:?}", out.resolved);
    assert_eq!(out.resolved[0].to, 1);
    assert!(out.ambiguous.is_empty(), "{:?}", out.ambiguous);
}

// ---- `js_specifier.rs` and `go_import.rs` ----

#[test]
fn js_specifier_from_ts_prefers_ts_source() {
    assert_eq!(
        relative_module_candidates("tests/lib/hash.test.ts", "../../src/lib/hash.js"),
        vec![
            "src/lib/hash.ts",
            "src/lib/hash.tsx",
            "src/lib/hash.d.ts",
            "src/lib/hash.js",
        ]
    );
}

#[test]
fn js_specifier_from_js_prefers_named_file() {
    assert_eq!(
        relative_module_candidates("src/a.mjs", "./b.js")[0],
        "src/b.js"
    );
}

#[test]
fn module_extension_mapping() {
    assert_eq!(
        relative_module_candidates("a.ts", "./v.jsx"),
        vec!["v.tsx", "v.jsx"]
    );
    assert_eq!(
        relative_module_candidates("a.ts", "./m.mjs"),
        vec!["m.mts", "m.d.mts", "m.mjs"]
    );
    assert_eq!(
        relative_module_candidates("a.ts", "./c.cjs"),
        vec!["c.cts", "c.d.cts", "c.cjs"]
    );
}

#[test]
fn extensionless_tries_sources_then_index() {
    let c = relative_module_candidates("src/a.ts", "./util");
    assert_eq!(c[0], "src/util");
    assert_eq!(c[1], "src/util.ts");
    assert!(c.contains(&"src/util/index.ts".to_string()));
}

#[test]
fn non_relative_or_escaping_specifiers_yield_nothing() {
    assert!(relative_module_candidates("src/a.ts", "vitest").is_empty());
    assert!(relative_module_candidates("src/a.ts", "@scope/pkg/x.js").is_empty());
    assert!(relative_module_candidates("a.ts", "../outside.js").is_empty());
    assert!(relative_module_candidates("src/a.py", "./b.js").is_empty());
}

#[test]
fn query_suffix_and_directory_specifiers() {
    assert_eq!(
        relative_module_candidates("src/a.ts", "./w.js?worker")[0],
        "src/w.ts"
    );
    assert_eq!(
        relative_module_candidates("src/a/b.ts", "..")[0],
        "src/index.ts"
    );
}

#[test]
fn go_import_identifiers() {
    assert_eq!(go_import_identifier("net/url").as_deref(), Some("url"));
    assert_eq!(
        go_import_identifier("github.com/golang-jwt/jwt/v5").as_deref(),
        Some("jwt")
    );
    assert_eq!(
        go_import_identifier("github.com/jackc/pgx/v5").as_deref(),
        Some("pgx")
    );
    assert_eq!(
        go_import_identifier("example.com/x as y").as_deref(),
        Some("y")
    );
}

// ---- projects, read and resolved ----

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

struct Project {
    svc: Service,
    root: PathBuf,
    id: String,
}

impl Project {
    fn target(&self) -> Target {
        self.target_in(&self.root)
    }

    fn target_in(&self, dir: &Path) -> Target {
        Target {
            project_id: self.id.clone(),
            root: self.root.clone(),
            dir: dir.to_path_buf(),
            fence: Fence::default(),
            exclude: Vec::new(),
        }
    }

    fn read(&self) {
        self.svc.ensure_fresh(&self.target(), true).unwrap();
    }

    /// `from -> to` of the call links, by the last two segments of their qualified names.
    fn calls(&self) -> Vec<String> {
        calls_of(&self.svc, &self.target())
    }
}

fn tail(q: &str) -> String {
    let mut parts: Vec<&str> = q.rsplit("::").take(2).collect();
    parts.reverse();
    parts.join("::")
}

fn calls_of(svc: &Service, t: &Target) -> Vec<String> {
    let mut v: Vec<String> = svc
        .links_for_tests(t)
        .0
        .into_iter()
        .filter_map(|l| {
            let p: Vec<&str> = l.split('|').collect();
            (p[2] == "call").then(|| format!("{} -> {}", tail(p[0]), tail(p[1])))
        })
        .collect();
    v.sort();
    v
}

fn project(name: &str, files: &[(&str, &str)]) -> Project {
    let base = test_dir(name);
    let root = base.join("repo");
    for (rel, text) in files {
        write(&root, rel, text);
    }
    let p = Project {
        svc: Service::new(DataDir::new(base.join("data"))),
        root,
        id: "p".into(),
    };
    p.read();
    p
}

fn named(name: &str) -> Wanted<'_> {
    Wanted::Name {
        name,
        path: None,
        max_matches: 3,
    }
}

// ---- `tests/ambiguous_calls_test.rs` ----

const STATUS_BAR: &str =
    "export class StatusBar {\n    dispose(): void {\n        console.log('bar');\n    }\n}\n";
const PROVIDER_MANAGER: &str =
    "export class ProviderManager {\n    dispose(): void {\n        console.log('mgr');\n    }\n}\n";
const CALLER_TS: &str =
    "export class Extension {\n    deactivate(thing): void {\n        thing.dispose();\n    }\n}\n";
const WIDGET: &str =
    "export class Widget {\n    render(): void {\n        console.log('w');\n    }\n}\n";
const APP: &str = "import { Widget } from './widget';\n\
                   export class App {\n    private widget = new Widget();\n\
                   \n    start(): void {\n        this.widget.render();\n    }\n}\n";

fn ambiguous_project(name: &str) -> Project {
    project(
        name,
        &[
            ("src/a.ts", STATUS_BAR),
            ("src/b.ts", PROVIDER_MANAGER),
            ("src/extension.ts", CALLER_TS),
        ],
    )
}

#[test]
fn an_ambiguous_call_records_its_candidates() {
    let p = ambiguous_project("lk-amb-record");
    let (_, ties) = p.svc.links_for_tests(&p.target());
    let dispose: Vec<&String> = ties.iter().filter(|t| t.contains("dispose")).collect();
    assert_eq!(dispose.len(), 1, "recorded once: {ties:?}");
    let parts: Vec<&str> = dispose[0].split('|').collect();
    assert_eq!(
        parts[2], "src/extension.ts",
        "the call site, not a candidate"
    );
    assert!(parts[4].split(',').count() >= 2, "{dispose:?}");
}

#[test]
fn recording_an_ambiguity_still_creates_no_edge() {
    let p = ambiguous_project("lk-amb-noedge");
    assert!(
        !p.calls().iter().any(|c| c.ends_with("::dispose")),
        "{:?}",
        p.calls()
    );
}

#[test]
fn an_unambiguous_project_records_no_ambiguity() {
    let p = project(
        "lk-amb-none",
        &[("src/widget.ts", WIDGET), ("src/app.ts", APP)],
    );
    assert!(p.svc.links_for_tests(&p.target()).1.is_empty());
}

// ---- `tests/ts_phantom_call_edges_test.rs` ----

fn two_dispose(name: &str, first: &str, second: &str) -> Project {
    project(
        name,
        &[
            (&format!("src/{first}"), STATUS_BAR),
            (&format!("src/{second}"), PROVIDER_MANAGER),
            (
                "src/extension.ts",
                "import { StatusBar } from './a';\nimport { ProviderManager } from './b';\n\
                 export class Extension {\n    private statusBar = new StatusBar();\n\
                 \n    deactivate(): void {\n        this.statusBar.dispose();\n    }\n}\n",
            ),
        ],
    )
}

#[test]
fn call_edges_do_not_depend_on_file_enumeration_order() {
    let a = two_dispose("lk-ts-order-a", "a.ts", "b.ts");
    let b = two_dispose("lk-ts-order-b", "z_a.ts", "b.ts");
    assert_eq!(a.calls(), b.calls());
}

#[test]
fn a_call_on_an_untypeable_receiver_resolves_to_nothing() {
    let p = ambiguous_project("lk-ts-untyped");
    assert!(!p.calls().iter().any(|c| c.ends_with("dispose")));
}

#[test]
fn an_unambiguous_receiver_call_still_resolves() {
    let p = project(
        "lk-ts-unamb",
        &[("src/widget.ts", WIDGET), ("src/app.ts", APP)],
    );
    assert!(
        p.calls().iter().any(|c| c.ends_with("render")),
        "{:?}",
        p.calls()
    );
}

#[test]
fn reindexing_the_same_tree_produces_the_same_edges() {
    let p = two_dispose("lk-ts-again", "a.ts", "b.ts");
    let first = p.svc.links_for_tests(&p.target());
    p.svc.delete(&p.id);
    p.read();
    assert_eq!(first, p.svc.links_for_tests(&p.target()));
}

#[test]
fn a_field_initialised_with_new_resolves_to_that_class() {
    let p = two_dispose("lk-ts-new", "a.ts", "b.ts");
    let dispose: Vec<String> = p
        .calls()
        .into_iter()
        .filter(|c| c.ends_with("dispose"))
        .collect();
    assert_eq!(dispose, vec!["Extension::deactivate -> StatusBar::dispose"]);
}

#[test]
fn a_field_with_a_type_annotation_resolves_to_that_class() {
    let p = project(
        "lk-ts-annot",
        &[
            (
                "src/a.ts",
                "export class Alpha {\n    run(): void {\n        console.log('a');\n    }\n}\n",
            ),
            (
                "src/b.ts",
                "export class Beta {\n    run(): void {\n        console.log('b');\n    }\n}\n",
            ),
            (
                "src/app.ts",
                "import { Alpha } from './a';\nimport { Beta } from './b';\n\
                 export class App {\n    private worker: Beta;\n\
                 \n    constructor(w: Beta) {\n        this.worker = w;\n    }\n\
                 \n    go(): void {\n        this.worker.run();\n    }\n}\n",
            ),
        ],
    );
    let run: Vec<String> = p
        .calls()
        .into_iter()
        .filter(|c| c.ends_with("run"))
        .collect();
    assert_eq!(run, vec!["App::go -> Beta::run"]);
}

// ---- `tests/python_phantom_call_edges_test.rs` ----

#[test]
fn a_lone_candidate_in_an_unreachable_module_is_not_a_match() {
    let p = project(
        "lk-py-lone",
        &[
            (
                "src/a.py",
                "import logging\n\nlog = logging.getLogger(__name__)\n\n\n\
                 def do_work(payload):\n    log.info(\"starting %s\", payload)\n    return payload\n",
            ),
            ("tests/helper.py", "def info(msg, *args):\n    return (msg, args)\n"),
        ],
    );
    assert!(
        !p.calls().iter().any(|c| c.contains("info")),
        "{:?}",
        p.calls()
    );
}

#[test]
fn an_imported_or_sibling_candidate_still_resolves() {
    let p = project(
        "lk-py-import",
        &[
            (
                "other/caller.py",
                "import pkg.helpers\n\n\ndef run(x):\n    return pkg.helpers.transmogrify(x)\n",
            ),
            ("pkg/helpers.py", "def transmogrify(x):\n    return x + 1\n"),
            ("pkg/neighbour.py", "def embiggen(x):\n    return x * 2\n"),
            (
                "pkg/user.py",
                "def use(thing):\n    return thing.embiggen(3)\n",
            ),
        ],
    );
    let calls = p.calls();
    assert!(
        calls.iter().any(|c| c.ends_with("transmogrify")),
        "{calls:?}"
    );
    assert!(calls.iter().any(|c| c.ends_with("embiggen")), "{calls:?}");
}

// ---- `tests/resolution_order_independence_test.rs` ----

#[test]
fn tied_candidates_resolve_the_same_way_in_either_scan_order() {
    let f = project(
        "lk-ord-tie-f",
        &[
            ("src/a.ts", STATUS_BAR),
            ("src/b.ts", PROVIDER_MANAGER),
            ("src/extension.ts", CALLER_TS),
        ],
    );
    let r = project(
        "lk-ord-tie-r",
        &[
            ("src/a.ts", PROVIDER_MANAGER),
            ("src/b.ts", STATUS_BAR),
            ("src/extension.ts", CALLER_TS),
        ],
    );
    assert_eq!(f.calls(), r.calls());
}

#[test]
fn kind_filtered_candidates_resolve_the_same_way_in_either_scan_order() {
    const HANDLER: &str =
        "pub struct Handler;\n\nimpl Handler {\n    pub fn process(&self) {}\n}\n";
    const WORKER: &str = "pub struct Worker;\n\nimpl Worker {\n    pub fn process(&self) {}\n}\n";
    const CALLER_RS: &str =
        "pub struct process;\n\npub fn run(thing: Unknown) {\n    thing.process();\n}\n";
    let f = project(
        "lk-ord-kind-f",
        &[
            ("src/a.rs", HANDLER),
            ("src/b.rs", WORKER),
            ("src/caller.rs", CALLER_RS),
        ],
    );
    let r = project(
        "lk-ord-kind-r",
        &[
            ("src/a.rs", WORKER),
            ("src/b.rs", HANDLER),
            ("src/caller.rs", CALLER_RS),
        ],
    );
    assert_eq!(f.calls(), r.calls());
}

#[test]
fn an_unambiguous_call_still_resolves_in_this_harness() {
    let p = project(
        "lk-ord-control",
        &[
            ("src/widget.ts", WIDGET),
            (
                "src/app.ts",
                "import { Widget } from './widget';\n\nexport class App {\n    private widget = new Widget();\n\n    draw(): void {\n        this.widget.render();\n    }\n}\n",
            ),
        ],
    );
    assert!(
        p.calls().iter().any(|e| e == "App::draw -> Widget::render"),
        "{:?}",
        p.calls()
    );
}

// ---- `tests/incremental_resolution_test.rs` ----

const LIB: &str = "pub mod shapes;\npub mod extra;\npub mod render;\npub use shapes::Circle;\n";
const SHAPES: &str = r#"
pub trait Shape {
    fn area(&self) -> f64;
}

pub struct Circle {
    pub radius: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}

pub fn unit_circle() -> Circle {
    Circle { radius: 1.0 }
}
"#;
const EXTRA: &str = r#"
use crate::shapes::Shape;

pub struct Square {
    pub side: f64,
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
}

pub fn unit_square() -> Square {
    Square { side: 1.0 }
}
"#;
const RENDER: &str = r#"
use crate::extra::{unit_square, Square};
use crate::shapes::{unit_circle, Circle, Shape};

pub fn describe(c: &Circle) -> String {
    let a = c.area();
    format!("area {a}")
}

pub fn describe_square(s: &Square) -> String {
    let a = s.area();
    format!("area {a}")
}

pub fn both() -> f64 {
    let c = unit_circle();
    let s = unit_square();
    c.area() + s.area()
}
"#;

fn shapes_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("src/lib.rs", LIB),
        ("src/shapes.rs", SHAPES),
        ("src/extra.rs", EXTRA),
        ("src/render.rs", RENDER),
    ]
}

fn append(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    let mut current = std::fs::read_to_string(&path).unwrap();
    current.push_str(text);
    std::fs::write(&path, current).unwrap();
}

fn replace_in(root: &Path, rel: &str, from: &str, to: &str) {
    let path = root.join(rel);
    let current = std::fs::read_to_string(&path).unwrap();
    assert!(current.contains(from), "{rel}: {from:?}");
    std::fs::write(&path, current.replace(from, to)).unwrap();
}

type Edit = (&'static str, fn(&Path));

fn edits() -> Vec<Edit> {
    vec![
        ("comment-only change", |root| {
            append(root, "src/shapes.rs", "\n// a comment, and nothing else\n");
        }),
        ("add a definition called from elsewhere", |root| {
            append(
                root,
                "src/shapes.rs",
                "\npub fn perimeter(c: &Circle) -> f64 {\n    6.28318 * c.radius\n}\n",
            );
            append(
                root,
                "src/render.rs",
                "\npub fn outline(c: &Circle) -> f64 {\n    crate::shapes::perimeter(c)\n}\n",
            );
        }),
        ("remove a definition", |root| {
            replace_in(
                root,
                "src/shapes.rs",
                "pub fn perimeter(c: &Circle) -> f64 {\n    6.28318 * c.radius\n}",
                "",
            );
        }),
        ("rename a definition", |root| {
            replace_in(root, "src/shapes.rs", "unit_circle", "make_unit_circle");
            replace_in(root, "src/render.rs", "unit_circle", "make_unit_circle");
        }),
        ("move a definition between files", |root| {
            replace_in(
                root,
                "src/extra.rs",
                "pub fn unit_square() -> Square {\n    Square { side: 1.0 }\n}",
                "",
            );
            append(
                root,
                "src/shapes.rs",
                "\npub fn unit_square() -> crate::extra::Square {\n    crate::extra::Square { side: 1.0 }\n}\n",
            );
        }),
        (
            "remove one of two same-named candidates, caller untouched",
            |root| {
                replace_in(
                root,
                "src/extra.rs",
                "impl Shape for Square {\n    fn area(&self) -> f64 {\n        self.side * self.side\n    }\n}",
                "",
            );
            },
        ),
        ("delete a file, caller untouched", |root| {
            std::fs::remove_file(root.join("src").join("extra.rs")).unwrap();
            replace_in(root, "src/lib.rs", "pub mod extra;\n", "");
        }),
    ]
}

#[test]
fn incremental_resolution_matches_a_full_pass_across_a_series_of_edits() {
    let inc = project("lk-inc-i", &shapes_files());
    let full = project("lk-inc-f", &shapes_files());
    let (first, _) = inc.svc.links_for_tests(&inc.target());
    assert!(!first.is_empty(), "the fixture resolves cross-file links");
    for (name, apply) in edits() {
        apply(&inc.root);
        apply(&full.root);
        inc.read();
        full.read();
        full.svc.resolve_again_for_tests(&full.target());
        let (il, ia) = inc.svc.links_for_tests(&inc.target());
        let (fl, fa) = full.svc.links_for_tests(&full.target());
        let missing: Vec<&String> = fl.iter().filter(|e| !il.contains(e)).collect();
        assert!(missing.is_empty(), "after {name}: dropped {missing:?}");
        assert_eq!(il, fl, "after {name}: links");
        assert_eq!(ia, fa, "after {name}: ties");
    }
}

#[test]
fn touched_name_keys_match_the_resolvers_own_name_index() {
    let p = project("lk-inc-keys", &shapes_files());
    let (known, derived) = p.svc.known_names_for_tests(&p.id);
    assert!(!known.is_empty());
    let mut only_known: Vec<_> = known.difference(&derived).collect();
    let mut only_derived: Vec<_> = derived.difference(&known).collect();
    only_known.sort();
    only_derived.sort();
    assert!(only_known.is_empty(), "{only_known:?}");
    assert!(only_derived.is_empty(), "{only_derived:?}");
}

// ---- ours: the `forget`s of this repository ----

/// The shapes of the `forget`s of Escouade's code: `Runtime::forget` and its 7 calls (4 through a
/// field of `Core`, one in a closure of `runtime.rs`, two in its tests through a field of their
/// fixture), and 12 other `forget`s with their calls.
fn forgets() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "src/plugins/runtime.rs",
            "use std::sync::{Arc, Weak};\n\
             pub struct Runtime {\n    me: Weak<Runtime>,\n}\n\
             impl Runtime {\n\
             \x20   pub fn new() -> Arc<Runtime> {\n\
             \x20       let rt = Arc::new(Runtime { me: Weak::new() });\n\
             \x20       let weak = Arc::downgrade(&rt);\n\
             \x20       on_reload(move |id| {\n\
             \x20           if let Some(rt) = weak.upgrade() {\n\
             \x20               rt.forget(id);\n\
             \x20           }\n\
             \x20       });\n\
             \x20       rt\n\
             \x20   }\n\
             \x20   pub fn forget(&self, id: &str) {\n        let _ = id;\n    }\n\
             }\n\
             fn on_reload(f: impl Fn(&str)) {\n    f(\"x\");\n}\n",
        ),
        (
            "src/core.rs",
            "pub struct Core {\n\
             \x20   pub(crate) plugin_runtime: Arc<crate::plugins::runtime::Runtime>,\n\
             \x20   pub(crate) sidechains: Mutex<crate::agent::sidechain::Sidechains>,\n\
             }\n\
             impl Core {\n\
             \x20   fn ended(&self, id: &str, call: &str) {\n\
             \x20       self.sidechains.lock().forget(id, call);\n\
             \x20   }\n\
             \x20   fn forget_memory(&self, dir: &Path, name: &str) {\n\
             \x20       crate::memory::forget(dir, name);\n\
             \x20   }\n\
             }\n",
        ),
        (
            "src/commands.rs",
            "type CoreState<'a> = State<'a, Arc<Core>>;\n\
             pub fn plugin_view_closed(core: CoreState, view: View) {\n    core.plugin_runtime.forget(&view.id);\n}\n\
             pub fn plugin_disable(core: CoreState, id: String) {\n    core.plugin_runtime.forget(&id);\n}\n\
             pub fn plugin_uninstall(core: CoreState, id: String) {\n    core.plugin_runtime.forget(&id);\n}\n\
             pub fn plugin_revoke(core: CoreState, id: String) {\n    core.plugin_runtime.forget(&id);\n}\n\
             pub fn engines_reset() {\n    crate::engine::registry::forget();\n}\n\
             pub fn wsl_forget(all: bool) {\n    crate::wsl::forget(all);\n}\n",
        ),
        ("src/wsl.rs", "pub fn forget(all: bool) {\n    let _ = all;\n}\n"),
        ("src/engine/registry.rs", "pub fn forget() {}\n"),
        (
            "src/memory.rs",
            "pub fn forget(dir: &Path, name: &str) -> Result<bool> {\n    Ok(true)\n}\n",
        ),
        (
            "src/memory_tests.rs",
            "use crate::memory::*;\n#[test]\nfn forgets() {\n    assert!(forget(&dir, \"a\").unwrap());\n}\n",
        ),
        (
            "src/plugins_runtime_tests.rs",
            "use crate::plugins::runtime::Runtime;\n\
             struct Fixture {\n    rt: Arc<Runtime>,\n}\n\
             #[tokio::test]\nasync fn a_secret_taken_back() {\n\
             \x20   let f = fixture();\n    f.rt.forget(ID);\n    f.rt.forget(ID);\n}\n",
        ),
        (
            "src/engine/bench.rs",
            "pub struct Bench {\n    rt: AgentRt,\n}\n\
             impl Bench {\n    fn done(&self) {\n        self.rt.forget(1);\n    }\n}\n",
        ),
        (
            "src/agent/rt.rs",
            "pub struct AgentRt;\nimpl AgentRt {\n    pub fn forget(&self, n: u32) {\n        let _ = n;\n    }\n}\n",
        ),
        (
            "src/core_tests.rs",
            "#[test]\nfn leaks_the_app() {\n    let app = 1;\n    std::mem::forget(app);\n}\n",
        ),
        (
            "src/attachfiles.rs",
            "struct Uploads;\nimpl Uploads {\n    fn forget(&self, handle: &str) {\n        let _ = handle;\n    }\n}\n\
             pub struct Files {\n    uploads: Uploads,\n}\n\
             impl Files {\n    fn drop_one(&self, handle: &str) {\n        self.uploads.forget(handle);\n    }\n}\n",
        ),
        (
            "src/agent/sidechain.rs",
            "#[derive(Default)]\npub struct Sidechains;\n\
             impl Sidechains {\n    pub fn forget(&mut self, agent: &str, call: &str) {\n        let _ = (agent, call);\n    }\n}\n\
             #[test]\nfn found_once() {\n    let mut found = Sidechains::default();\n    found.forget(\"a2\", \"c11\");\n}\n",
        ),
    ]
}

/// `path:line` of each caller of the definitions in the answer.
fn sites(svc: &Service, t: &Target, wanted: Wanted) -> Vec<String> {
    let r = svc.reader(t).unwrap();
    let a = r.callers(wanted, 50).unwrap().result;
    let mut v: Vec<String> = a
        .matches
        .iter()
        .flat_map(|m| m.callers.iter().map(|c| format!("{}:{}", c.path, c.line)))
        .collect();
    v.sort();
    v
}

const RUNTIME_FORGET: &[&str] = &[
    "src/commands.rs:12",
    "src/commands.rs:3",
    "src/commands.rs:6",
    "src/commands.rs:9",
    "src/plugins/runtime.rs:11",
    "src/plugins_runtime_tests.rs:8",
    "src/plugins_runtime_tests.rs:9",
];

#[test]
fn runtime_forget_has_its_seven_callers_and_no_other_forget_s() {
    let p = project("lk-forget", &forgets());
    let t = p.target();
    assert_eq!(sites(&p.svc, &t, named("Runtime::forget")), RUNTIME_FORGET);
    assert_eq!(sites(&p.svc, &t, named("Runtime.forget")), RUNTIME_FORGET);
    // By the file, and by the id.
    let in_file = Wanted::Name {
        name: "forget",
        path: Some("src/plugins/runtime.rs"),
        max_matches: 3,
    };
    assert_eq!(sites(&p.svc, &t, in_file), RUNTIME_FORGET);
    let r = p.svc.reader(&t).unwrap();
    let a = r.callers(named("Runtime::forget"), 50).unwrap().result;
    assert_eq!(a.matches.len(), 1);
    let id = a.matches[0].def.id;
    assert_eq!(sites(&p.svc, &t, Wanted::Id(id)), RUNTIME_FORGET);
    assert_eq!(a.matches[0].total, 7);
    // The others reach their own.
    assert_eq!(
        sites(&p.svc, &t, named("Uploads::forget")),
        vec!["src/attachfiles.rs:12"]
    );
    assert_eq!(
        sites(&p.svc, &t, named("AgentRt::forget")),
        vec!["src/engine/bench.rs:6"]
    );
    assert_eq!(
        sites(&p.svc, &t, named("Sidechains::forget")),
        vec!["src/agent/sidechain.rs:11"]
    );
    // A bare name is answered definition by definition, up to max_matches, the rest listed.
    let all = r.callers(named("forget"), 50).unwrap().result;
    assert_eq!(all.matches.len(), 3);
    assert_eq!(all.matches.len() + all.more.len(), 7, "{:?}", all.more);
    let wide = Wanted::Name {
        name: "forget",
        path: None,
        max_matches: 20,
    };
    let every = r.callers(wide, 50).unwrap().result;
    let rt = every
        .matches
        .iter()
        .find(|m| m.def.path == "src/plugins/runtime.rs")
        .unwrap();
    assert_eq!(rt.callers.len(), 7);
    // No caller of one is a caller of another.
    let mut seen = std::collections::HashSet::new();
    for m in &every.matches {
        for c in &m.callers {
            assert!(seen.insert((c.path.clone(), c.line)), "{c:?} twice");
        }
    }
}

#[test]
fn a_call_left_to_a_tie_is_told_apart_never_as_a_caller() {
    let p = project("lk-forget-tie", &forgets());
    let r = p.svc.reader(&p.target()).unwrap();
    let a = r.callers(named("Runtime::forget"), 50).unwrap().result;
    let m = &a.matches[0];
    // `std::mem::forget(app)` and the bare `forget(&dir, "a")` of the memory's tests tie between
    // the public `forget`s: said apart, with how many they may mean.
    let tied: Vec<String> = m
        .ambiguous
        .iter()
        .map(|t| format!("{}:{} {}", t.path, t.line, t.called))
        .collect();
    assert!(
        tied.contains(&"src/core_tests.rs:4 std::mem::forget".to_string()),
        "{tied:?}"
    );
    assert!(m.ambiguous.iter().all(|t| t.candidates >= 2));
    assert!(!m.callers.iter().any(|c| c.path == "src/core_tests.rs"));
    // What a symbol calls tells its ties apart too.
    let c = r.callees(named("leaks_the_app"), 50).unwrap().result;
    assert!(c.matches[0].calls.is_empty());
    assert_eq!(c.matches[0].ambiguous.len(), 1);
}

#[test]
fn a_changed_file_is_read_alone_and_its_links_stay_right() {
    let p = project("lk-forget-change", &forgets());
    let t = p.target();
    let before = p.svc.links_for_tests(&t);
    replace_in(
        &p.root,
        "src/commands.rs",
        "pub fn plugin_revoke(core: CoreState, id: String) {\n    core.plugin_runtime.forget(&id);\n}\n",
        "pub fn plugin_revoke(core: CoreState, id: String) {\n    let _ = (core, id);\n}\n",
    );
    let done = p.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!(
        (done.parsed, done.dropped),
        (1, 0),
        "only the changed file is read"
    );
    let got = sites(&p.svc, &t, named("Runtime::forget"));
    assert_eq!(got.len(), 6, "{got:?}");
    assert!(!got.contains(&"src/commands.rs:12".to_string()));
    // The links into other files did not move.
    let after = p.svc.links_for_tests(&t);
    assert_ne!(before.0, after.0);
    p.svc.resolve_again_for_tests(&t);
    assert_eq!(after, p.svc.links_for_tests(&t), "as a full resolution");
    // A definition removed takes the links to it with it; its callers are told again.
    replace_in(
        &p.root,
        "src/attachfiles.rs",
        "    fn forget(&self, handle: &str) {\n        let _ = handle;\n    }\n",
        "",
    );
    p.svc.ensure_fresh(&t, true).unwrap();
    assert!(sites(&p.svc, &t, named("Uploads::forget")).is_empty());
    let now = p.svc.links_for_tests(&t);
    p.svc.resolve_again_for_tests(&t);
    assert_eq!(now, p.svc.links_for_tests(&t));
}

#[test]
fn a_worktree_resolves_its_delta_and_the_project_keeps_its_links() {
    let p = project("lk-forget-wt", &forgets());
    let base = p.target();
    let wt = p.root.parent().unwrap().join("wt");
    for (rel, text) in forgets() {
        write(&wt, rel, text);
    }
    // The worktree drops a call, adds one in a new file, and moves `AgentRt::forget`.
    replace_in(
        &wt,
        "src/commands.rs",
        "    core.plugin_runtime.forget(&view.id);\n",
        "    let _ = (core, view);\n",
    );
    write(
        &wt,
        "src/plugins/view.rs",
        "pub fn closed(core: CoreState, id: String) {\n    core.plugin_runtime.forget(&id);\n}\n",
    );
    replace_in(
        &wt,
        "src/attachfiles.rs",
        "    fn forget(&self, handle: &str) {\n        let _ = handle;\n    }\n",
        "    fn forget(&self, handle: &str) {\n\n        let _ = handle;\n    }\n",
    );
    let t = p.target_in(&wt);
    p.svc.ensure_fresh(&t, true).unwrap();
    let in_wt = sites(&p.svc, &t, named("Runtime::forget"));
    assert_eq!(in_wt.len(), 7, "{in_wt:?}");
    assert!(in_wt.contains(&"src/plugins/view.rs:2".to_string()));
    assert!(!in_wt.contains(&"src/commands.rs:3".to_string()));
    // A caller in an unchanged file, of a definition the worktree changed, follows it.
    let r = p.svc.reader(&t).unwrap();
    let a = r.callers(named("AgentRt::forget"), 50).unwrap().result;
    assert_eq!(a.matches[0].def.line, 3, "the worktree's definition");
    assert_eq!(
        sites(&p.svc, &t, named("AgentRt::forget")),
        vec!["src/engine/bench.rs:6"]
    );
    drop(r);
    // The project's folder is unchanged.
    assert_eq!(
        sites(&p.svc, &base, named("Runtime::forget")),
        RUNTIME_FORGET
    );
    // As a full resolution, both.
    let got = p.svc.links_for_tests(&t);
    p.svc.resolve_again_for_tests(&t);
    assert_eq!(got, p.svc.links_for_tests(&t));
    // The project changes: the worktree keeps its own copy of the file, the project loses the call.
    replace_in(
        &p.root,
        "src/plugins/runtime.rs",
        "                rt.forget(id);\n",
        "                let _ = (rt, id);\n",
    );
    p.read();
    p.svc.ensure_fresh(&t, true).unwrap();
    let in_wt = sites(&p.svc, &t, named("Runtime::forget"));
    assert!(
        in_wt.contains(&"src/plugins/runtime.rs:11".to_string()),
        "{in_wt:?}"
    );
    let in_base = sites(&p.svc, &base, named("Runtime::forget"));
    assert_eq!(in_base.len(), 6, "{in_base:?}");
    assert!(!in_base.contains(&"src/plugins/runtime.rs:11".to_string()));
    let got = p.svc.links_for_tests(&t);
    p.svc.resolve_again_for_tests(&t);
    assert_eq!(got, p.svc.links_for_tests(&t));
}

#[test]
fn self_and_a_typed_parameter_become_type_method_in_each_language() {
    use crate::codeindex::lang_for_tests as parse;
    let typed = |path: &str, src: &str| -> Vec<String> {
        let mut v: Vec<String> = parse(path, src)
            .edges
            .into_iter()
            .filter(|e| e.via > 0)
            .map(|e| format!("{}:{}", e.via, e.target))
            .collect();
        v.sort();
        v
    };
    assert_eq!(
        typed(
            "a.rs",
            "struct S { w: Arc<W> }\nimpl S {\n    fn f(&self, p: &mut P) {\n        self.g();\n        p.h();\n        let q = Q::new();\n        q.i();\n        self.w.j();\n    }\n}\n"
        ),
        vec!["1:P::h", "1:Q::i", "1:S::g", "2:w::j"]
    );
    assert_eq!(
        typed(
            "a.ts",
            "class C {\n  private d = new D();\n  m(p: P) {\n    this.n();\n    p.o();\n    this.d.q();\n    const e: E = make();\n    e.r();\n  }\n}\n"
        ),
        vec!["1:C::n", "1:D::q", "1:E::r", "1:P::o"]
    );
    assert_eq!(
        typed(
            "a.py",
            "class C:\n    def m(self, p: P):\n        self.n()\n        p.o()\n        x = Q()\n        x.r()\n"
        ),
        vec!["1:C::n", "1:P::o", "1:Q::r"]
    );
}

/// `cargo test codeindex_links_tests::on_this_repository -- --ignored --nocapture`: the 15 `forget`s
/// of Escouade's code; `Runtime::forget` has its 7 calls, and no other.
#[test]
#[ignore = "the whole repository, by hand"]
fn on_this_repository_runtime_forget_has_its_seven_callers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let base = test_dir("lk-repo");
    let svc = Service::new(DataDir::new(base.join("data")));
    let t = Target {
        project_id: "escouade".into(),
        root: root.clone(),
        dir: root.clone(),
        fence: Fence::default(),
        exclude: vec![root.join(".claude")],
    };
    let started = std::time::Instant::now();
    let full = svc.ensure_fresh(&t, true).unwrap().unwrap();
    let st = svc.status("escouade");
    println!(
        "full: {} files, {} ms ({} ms in all); {} symbols, {} links",
        full.parsed,
        full.millis,
        started.elapsed().as_millis(),
        st.symbols,
        st.links
    );
    for (name, bytes) in svc.sizes_for_tests("escouade") {
        println!("size {name}: {:.1} MB", bytes as f64 / 1_048_576.0);
    }
    let r = svc.reader(&t).unwrap();
    let wide = Wanted::Name {
        name: "forget",
        path: None,
        max_matches: 20,
    };
    let all = r.callers(wide, 50).unwrap().result;
    for m in &all.matches {
        println!(
            "{}:{} {}.{}: {} callers, {} ties",
            m.def.path,
            m.def.line,
            m.def.parent,
            m.def.name,
            m.callers.len(),
            m.ambiguous.len()
        );
        for c in &m.callers {
            println!("    {}:{} {}", c.path, c.line, c.name);
        }
    }
    let rt = r.callers(named("Runtime::forget"), 50).unwrap().result;
    assert_eq!(rt.matches.len(), 1);
    let mut got: Vec<String> = rt.matches[0]
        .callers
        .iter()
        .map(|c| format!("{}:{}", c.path, c.line))
        .collect();
    got.sort();
    let mut want: Vec<String> = [
        "src-tauri/src/commands.rs",
        "src-tauri/src/plugins/runtime.rs",
        "src-tauri/src/plugins_runtime_tests.rs",
    ]
    .iter()
    .flat_map(|f| {
        let text = std::fs::read_to_string(root.join(f)).unwrap();
        text.lines()
            .enumerate()
            .filter(|(_, l)| l.contains("plugin_runtime.forget(") || l.contains("rt.forget("))
            .map(|(i, _)| format!("{f}:{}", i + 1))
            .collect::<Vec<_>>()
    })
    .collect();
    want.sort();
    assert_eq!(want.len(), 7, "{want:?}");
    assert_eq!(got, want);
    let inc_started = std::time::Instant::now();
    let touched = root.join("src-tauri/src/plugins/runtime.rs");
    let f = std::fs::File::options().write(true).open(&touched).unwrap();
    f.set_modified(std::time::SystemTime::now()).unwrap();
    drop(f);
    let inc = svc.ensure_fresh(&t, true).unwrap().unwrap();
    println!(
        "one file touched: {} parsed, {} ms ({} ms in all)",
        inc.parsed,
        inc.millis,
        inc_started.elapsed().as_millis()
    );
}
