//! « Index du code » (ESC-251): what each language gives, the index of a small project, its
//! freshness, the delta of a worktree, the files a private project keeps out, a renamed symbol,
//! the answers' savings, and the setting through the core.

use crate::codeindex::{Service, Target};
use crate::fence::Fence;
use crate::paths::{test_dir, DataDir};
use std::path::{Path, PathBuf};

// ---- what each language gives ----

mod langs {
    use crate::codeindex::{lang_for_tests as parse, Parsed};

    fn find<'a>(p: &'a Parsed, name: &str) -> &'a crate::codeindex::Sym {
        p.symbols
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} not found in {:?}", p.symbols))
    }

    fn calls(p: &Parsed, from: &str) -> Vec<String> {
        let idx = p.symbols.iter().position(|s| s.name == from).unwrap();
        p.edges
            .iter()
            .filter(|e| e.kind == "call" && e.from == Some(idx))
            .map(|e| e.target.clone())
            .collect()
    }

    fn links(p: &Parsed, kind: &str) -> Vec<String> {
        p.edges
            .iter()
            .filter(|e| e.kind == kind)
            .map(|e| e.target.clone())
            .collect()
    }

    #[test]
    fn rust_gives_functions_types_methods_fields_imports_calls_and_traits() {
        let p = parse(
            "src/lib.rs",
            r#"
use std::collections::HashMap;

/// Reads the settings.
#[derive(Debug)]
pub struct Config {
    pub name: String,
}

pub trait Load {
    fn load(&self) -> Config;
}

impl Load for Config {
    fn load(&self) -> Config {
        helper(1);
        self.other()
    }
}

impl Config {
    fn other(&self) -> Config { Config { name: String::new() } }
}

fn helper(n: u32) -> u32 { n }
"#,
        );
        assert_eq!(find(&p, "Config").kind, "struct");
        assert_eq!(find(&p, "Config").doc, "Reads the settings.");
        assert_eq!(find(&p, "name").kind, "field");
        assert_eq!(find(&p, "name").parent, "Config");
        assert_eq!(find(&p, "Load").kind, "trait");
        let load = p
            .symbols
            .iter()
            .find(|s| s.name == "load" && s.parent == "Config")
            .unwrap();
        assert_eq!(load.kind, "method");
        assert_eq!(find(&p, "helper").kind, "function");
        assert_eq!(calls(&p, "helper").len(), 0);
        assert!(links(&p, "import")[0].contains("HashMap"));
        assert_eq!(links(&p, "implements"), vec!["Load"]);
        let all_calls: Vec<String> = p
            .edges
            .iter()
            .filter(|e| e.kind == "call")
            .map(|e| e.target.clone())
            .collect();
        assert!(all_calls.contains(&"helper".to_string()));
        assert!(all_calls.contains(&"other".to_string()));
    }

    #[test]
    fn typescript_gives_classes_methods_arrow_functions_imports_and_inheritance() {
        let p = parse(
            "src/app.ts",
            r#"
import { Base } from './base';

/** The app. */
export class App extends Base implements Runnable {
  name: string = 'x';
  start(): void { this.boot(); run(); }
}

export interface Runnable { start(): void }
export const run = (): number => compute(2);
function compute(n: number) { return n; }
"#,
        );
        assert_eq!(find(&p, "App").kind, "class");
        assert_eq!(find(&p, "App").doc, "The app.");
        assert_eq!(find(&p, "start").kind, "method");
        assert_eq!(find(&p, "name").kind, "field");
        assert_eq!(find(&p, "Runnable").kind, "interface");
        assert_eq!(find(&p, "run").kind, "function");
        assert_eq!(find(&p, "compute").kind, "function");
        assert!(links(&p, "import")[0].contains("./base"));
        assert_eq!(links(&p, "extends"), vec!["Base"]);
        assert_eq!(links(&p, "implements"), vec!["Runnable"]);
        assert_eq!(calls(&p, "start"), vec!["boot", "run"]);
        assert_eq!(calls(&p, "run"), vec!["compute"]);
    }

    #[test]
    fn javascript_gives_functions_classes_and_calls() {
        let p = parse(
            "lib/util.js",
            "import fs from 'fs';\nclass Cache extends Map {\n  get2(k) { return this.get(k); }\n}\nfunction load(p) { return fs.readFileSync(p); }\nconst save = (p) => load(p);\n",
        );
        assert_eq!(find(&p, "Cache").kind, "class");
        assert_eq!(find(&p, "get2").kind, "method");
        assert_eq!(find(&p, "load").kind, "function");
        assert_eq!(find(&p, "save").kind, "function");
        assert_eq!(links(&p, "extends"), vec!["Map"]);
        assert_eq!(calls(&p, "save"), vec!["load"]);
        assert!(calls(&p, "load").contains(&"readFileSync".to_string()));
    }

    #[test]
    fn svelte_gives_the_component_the_functions_of_its_script_and_the_components_it_uses() {
        let p = parse(
            "src/Card.svelte",
            "<script lang=\"ts\">\n  import Child from './Child.svelte';\n  function open() { track(); }\n</script>\n\n<Child />\n<button onclick={open}>x</button>\n",
        );
        assert_eq!(find(&p, "Card").kind, "component");
        let open = find(&p, "open");
        assert_eq!(open.kind, "function");
        assert_eq!(open.line, 3);
        assert!(links(&p, "import")[0].contains("Child"));
        assert_eq!(calls(&p, "open"), vec!["track"]);
        assert_eq!(calls(&p, "Card"), vec!["Child"]);
    }

    #[test]
    fn python_gives_classes_methods_docstrings_imports_and_bases() {
        let p = parse(
            "pkg/mod.py",
            "import os\nfrom base import Base\n\nclass Repo(Base):\n    \"\"\"Keeps things.\"\"\"\n    def save(self):\n        helper()\n        os.remove('x')\n\ndef helper():\n    pass\n",
        );
        assert_eq!(find(&p, "Repo").kind, "class");
        assert_eq!(find(&p, "Repo").doc, "Keeps things.");
        assert_eq!(find(&p, "save").kind, "method");
        assert_eq!(find(&p, "helper").kind, "function");
        assert_eq!(links(&p, "import").len(), 2);
        assert_eq!(links(&p, "extends"), vec!["Base"]);
        assert_eq!(calls(&p, "save"), vec!["helper", "remove"]);
    }

    #[test]
    fn go_gives_functions_methods_with_their_type_structs_fields_and_imports() {
        let p = parse(
            "main.go",
            "package main\n\nimport \"fmt\"\n\n// Server serves.\ntype Server struct {\n\tPort int\n}\n\ntype Doer interface {\n\tDo() error\n}\n\nfunc (s *Server) Start() {\n\tfmt.Println(\"x\")\n\thelper()\n}\n\nfunc helper() {}\n",
        );
        assert_eq!(find(&p, "Server").kind, "struct");
        assert_eq!(find(&p, "Server").doc, "Server serves.");
        assert_eq!(find(&p, "Doer").kind, "interface");
        let start = find(&p, "Start");
        assert_eq!(start.kind, "method");
        assert_eq!(start.parent, "Server");
        assert_eq!(find(&p, "Port").kind, "field");
        assert_eq!(find(&p, "helper").kind, "function");
        assert!(links(&p, "import")[0].contains("fmt"));
        assert_eq!(calls(&p, "Start"), vec!["Println", "helper"]);
    }

    #[test]
    fn java_gives_classes_methods_fields_imports_and_bases() {
        let p = parse(
            "src/Shop.java",
            "import java.util.List;\n\n/** A shop. */\npublic class Shop extends Place implements Open, Close {\n  private int count;\n  public void sell() { restock(); new Item(); }\n  void restock() {}\n}\n",
        );
        assert_eq!(find(&p, "Shop").kind, "class");
        assert_eq!(find(&p, "Shop").doc, "A shop.");
        assert_eq!(find(&p, "count").kind, "field");
        assert_eq!(find(&p, "sell").kind, "method");
        assert!(links(&p, "import")[0].contains("java.util.List"));
        assert_eq!(links(&p, "extends"), vec!["Place"]);
        assert_eq!(links(&p, "implements"), vec!["Open", "Close"]);
        assert_eq!(calls(&p, "sell"), vec!["restock", "Item"]);
    }

    #[test]
    fn csharp_gives_namespaces_classes_methods_properties_and_bases() {
        let p = parse(
            "Shop.cs",
            "using System;\nnamespace Acme {\n  /// <summary>A shop.</summary>\n  public class Shop : Place, IOpen {\n    public int Count { get; set; }\n    private string name;\n    public void Sell() { Restock(); var i = new Item(); }\n    void Restock() {}\n  }\n}\n",
        );
        assert_eq!(find(&p, "Acme").kind, "module");
        assert_eq!(find(&p, "Shop").kind, "class");
        assert_eq!(find(&p, "Count").kind, "field");
        assert_eq!(find(&p, "name").kind, "field");
        assert_eq!(find(&p, "Sell").kind, "method");
        assert!(links(&p, "import")[0].contains("System"));
        assert_eq!(links(&p, "extends"), vec!["Place", "IOpen"]);
        assert_eq!(calls(&p, "Sell"), vec!["Restock", "Item"]);
    }

    #[test]
    fn php_gives_classes_methods_properties_functions_and_bases() {
        let p = parse(
            "src/Shop.php",
            "<?php\nnamespace Acme;\nuse Acme\\Place;\n\n/** A shop. */\nclass Shop extends Place implements Open {\n    private $count = 0;\n    public function sell() { $this->restock(); helper(); new Item(); }\n    private function restock() {}\n}\n\nfunction helper() {}\n",
        );
        assert_eq!(find(&p, "Shop").kind, "class");
        assert_eq!(find(&p, "Shop").doc, "A shop.");
        assert_eq!(find(&p, "count").kind, "field");
        assert_eq!(find(&p, "sell").kind, "method");
        assert_eq!(find(&p, "helper").kind, "function");
        assert!(links(&p, "import")[0].contains("Place"));
        assert_eq!(links(&p, "extends"), vec!["Place"]);
        assert_eq!(links(&p, "implements"), vec!["Open"]);
        assert_eq!(calls(&p, "sell"), vec!["restock", "helper", "Item"]);
    }

    #[test]
    fn markdown_titles_are_nodes_nested_by_level() {
        let p = parse(
            "docs/SPEC.md",
            "# Spec\n\nIntro.\n\n## Index du code\n\nText.\n\n### Fraîcheur\n\nMore.\n\n## Autre\n",
        );
        let spec = find(&p, "Spec");
        assert_eq!(spec.kind, "heading");
        assert_eq!(find(&p, "Index du code").parent, "Spec");
        assert_eq!(find(&p, "Fraîcheur").parent, "Spec.Index du code");
        assert_eq!(find(&p, "Autre").parent, "Spec");
        assert!(find(&p, "Index du code").end_line > find(&p, "Index du code").line);
    }
}

// ---- a small project ----

struct Fixture {
    root: PathBuf,
    svc: Service,
    data: PathBuf,
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

const SHOP: &str = "pub fn checkout(cart: &Cart) -> u32 {\n    let total = price(cart);\n    charge(total)\n}\n\nfn price(cart: &Cart) -> u32 { cart.items.len() as u32 }\n\nfn charge(n: u32) -> u32 { n }\n\npub struct Cart { pub items: Vec<u32> }\n";
const ORDERS: &str = "use crate::shop::checkout;\n\n/// Places an order from a cart.\npub fn place_order(cart: &Cart) {\n    checkout(cart);\n}\n";

fn fixture(name: &str) -> Fixture {
    let base = test_dir(name);
    let root = base.join("repo");
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    write(&root, "src/shop.rs", SHOP);
    write(&root, "src/orders.rs", ORDERS);
    write(
        &root,
        "README.md",
        "# Shop\n\n## Paiement\n\nCheckout of the cart.\n",
    );
    let data = base.join("data");
    Fixture {
        svc: Service::new(DataDir::new(data.clone())),
        root,
        data,
    }
}

impl Fixture {
    fn target(&self) -> Target {
        Target {
            project_id: "p1".into(),
            root: self.root.clone(),
            dir: self.root.clone(),
            fence: Fence::default(),
            exclude: Vec::new(),
        }
    }
}

#[test]
fn the_database_is_in_the_data_folder_never_in_the_repository_and_fts5_is_there() {
    let f = fixture("ci-place");
    f.svc.ensure_fresh(&f.target(), true).unwrap();
    let db = DataDir::new(f.data.clone()).code_index_db("p1");
    assert!(db.is_file());
    assert!(db.starts_with(&f.data));
    let in_repo = walk(&f.root);
    assert!(
        in_repo.iter().all(|p| !p.ends_with("code-index.db")),
        "{in_repo:?}"
    );
    let conn = rusqlite::Connection::open(&db).unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sym_fts WHERE sym_fts MATCH 'checkout'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(n > 0);
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        if e.path().is_dir() {
            out.extend(walk(&e.path()));
        } else {
            out.push(e.path());
        }
    }
    out
}

#[test]
fn search_finds_a_symbol_by_its_name_a_word_of_it_or_a_notion_of_its_doc() {
    let f = fixture("ci-search");
    let r = f.svc.reader(&f.target()).unwrap();
    let hits = r.search("checkout", 10, None).unwrap().result;
    assert_eq!(hits[0].name, "checkout");
    assert_eq!(hits[0].kind, "function");
    assert_eq!(hits[0].path, "src/shop.rs");
    assert_eq!(hits[0].line, 1);
    // A word of a snake_case name.
    let by_word = r.search("order", 10, None).unwrap().result;
    assert!(by_word.iter().any(|h| h.name == "place_order"));
    // A notion in the docs.
    let by_doc = r
        .search("places an order", 10, Some("function"))
        .unwrap()
        .result;
    assert_eq!(by_doc[0].name, "place_order");
    // The headings of a Markdown file.
    let md = r.search("Paiement", 10, None).unwrap().result;
    assert_eq!(md[0].kind, "heading");
    assert!(r
        .search("nothingofthiskind", 10, None)
        .unwrap()
        .result
        .is_empty());
}

#[test]
fn callers_callees_and_the_impact_of_a_function() {
    let f = fixture("ci-links");
    let r = f.svc.reader(&f.target()).unwrap();
    let callers = r.callers("checkout", None, 20).unwrap().result;
    assert_eq!(callers.defs[0].path, "src/shop.rs");
    assert_eq!(callers.callers.len(), 1);
    assert_eq!(callers.callers[0].name, "place_order");
    assert_eq!(callers.callers[0].path, "src/orders.rs");
    assert_eq!(callers.callers[0].line, 5);
    let callees = r.callees("checkout", None, 20).unwrap().result;
    let names: Vec<&str> = callees.calls.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(names, vec!["price", "charge"]);
    let impact = r.impact("charge", 3, 50).unwrap().result;
    let who: Vec<&str> = impact.callers.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(who, vec!["checkout", "place_order"]);
    assert_eq!(impact.callers[1].depth, 2);
    assert_eq!(impact.files, vec!["src/orders.rs", "src/shop.rs"]);
    // A file: what calls into it, and who imports it.
    let by_file = r.impact("src/shop.rs", 2, 50).unwrap().result;
    assert!(by_file.callers.iter().any(|c| c.name == "place_order"));
    assert_eq!(by_file.importers, vec!["src/orders.rs"]);
}

#[test]
fn the_context_of_a_task_gives_entry_points_related_symbols_and_code() {
    let f = fixture("ci-context");
    let r = f.svc.reader(&f.target()).unwrap();
    let a = r
        .task_context("Le checkout doit refuser un panier vide", 6)
        .unwrap();
    let entry: Vec<&str> = a
        .result
        .entry_points
        .iter()
        .map(|h| h.name.as_str())
        .collect();
    assert!(entry.contains(&"checkout"), "{entry:?}");
    let related: Vec<&str> = a.result.related.iter().map(|h| h.name.as_str()).collect();
    assert!(
        related.contains(&"price") || related.contains(&"place_order"),
        "{related:?}"
    );
    let snippet = a
        .result
        .snippets
        .iter()
        .find(|s| s.path == "src/shop.rs")
        .unwrap();
    assert!(snippet.text.contains("charge(total)"));
}

#[test]
fn every_answer_tells_what_it_saved() {
    let f = fixture("ci-saving");
    // Files big enough that reading them whole costs more than the answer.
    let mut big = String::from(SHOP);
    for i in 0..400 {
        big.push_str(&format!("fn filler_{i}(x: u32) -> u32 {{ x + {i} }}\n"));
    }
    write(&f.root, "src/shop.rs", &big);
    let r = f.svc.reader(&f.target()).unwrap();
    let on_size = std::fs::metadata(f.root.join("src/shop.rs")).unwrap().len();
    let a = r.search("checkout", 3, None).unwrap();
    assert!(a.saving.files_tokens >= on_size / 4, "{:?}", a.saving);
    assert!(a.saving.answer_tokens > 0);
    assert!(a.saving.answer_tokens < a.saving.files_tokens);
    assert_eq!(
        a.saving.saved_tokens,
        a.saving.files_tokens - a.saving.answer_tokens
    );
    assert!(a.saving.saved_percent > 50);
    let json = serde_json::to_value(&a).unwrap();
    assert!(json["saving"]["savedTokens"].as_u64().unwrap() > 0);
    for s in [
        r.callers("checkout", None, 5).unwrap().saving,
        r.callees("checkout", None, 5).unwrap().saving,
        r.impact("checkout", 2, 5).unwrap().saving,
        r.task_context("checkout", 4).unwrap().saving,
    ] {
        assert!(s.files > 0 && s.files_tokens > 0, "{s:?}");
    }
}

// ---- freshness ----

#[test]
fn a_query_sees_a_file_changed_since_but_not_more_often_than_the_minimal_delay() {
    let f = fixture("ci-fresh");
    let t = f.target();
    f.svc.reader(&t).unwrap();
    write(&f.root, "src/extra.rs", "pub fn brand_new() {}\n");
    // Within the delay: nothing read again.
    let r = f.svc.reader(&t).unwrap();
    assert!(r.search("brand_new", 5, None).unwrap().result.is_empty());
    // A git event does not wait for it.
    let done = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!(done.parsed, 1);
    let r = f.svc.reader(&t).unwrap();
    assert_eq!(
        r.search("brand_new", 5, None).unwrap().result[0].path,
        "src/extra.rs"
    );
}

#[test]
fn after_the_delay_a_query_reads_again_by_itself_and_only_what_changed() {
    let f = fixture("ci-fresh2");
    let t = f.target();
    let first = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!(first.parsed, 3);
    // Nothing changed: nothing parsed.
    let again = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!((again.parsed, again.dropped), (0, 0));
    std::thread::sleep(crate::codeindex::MIN_GAP + std::time::Duration::from_millis(200));
    write(&f.root, "src/shop.rs", &format!("{SHOP}fn late() {{}}\n"));
    let r = f.svc.reader(&t).unwrap();
    assert_eq!(r.search("late", 5, None).unwrap().result[0].name, "late");
    std::fs::remove_file(f.root.join("src/orders.rs")).unwrap();
    let done = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!((done.parsed, done.dropped), (0, 1));
    let r = f.svc.reader(&t).unwrap();
    assert!(r.search("place_order", 5, None).unwrap().result.is_empty());
}

#[test]
fn a_renamed_symbol_is_found_by_its_new_name_with_its_callers_and_no_longer_by_the_old() {
    let f = fixture("ci-rename");
    let t = f.target();
    f.svc.ensure_fresh(&t, true).unwrap();
    write(
        &f.root,
        "src/shop.rs",
        &SHOP.replace("checkout", "pay_cart"),
    );
    write(
        &f.root,
        "src/orders.rs",
        &ORDERS.replace("checkout", "pay_cart"),
    );
    f.svc.ensure_fresh(&t, true).unwrap();
    let r = f.svc.reader(&t).unwrap();
    assert!(r
        .search("checkout", 5, Some("function"))
        .unwrap()
        .result
        .is_empty());
    assert_eq!(
        r.search("pay_cart", 5, None).unwrap().result[0].name,
        "pay_cart"
    );
    let c = r.callers("pay_cart", None, 5).unwrap().result;
    assert_eq!(c.callers[0].name, "place_order");
    assert!(r
        .callers("checkout", None, 5)
        .unwrap()
        .result
        .callers
        .is_empty());
}

// ---- worktrees ----

/// A copy of the project's folder, as a worktree is.
fn worktree_of(f: &Fixture) -> PathBuf {
    let wt = f.root.parent().unwrap().join("wt");
    std::fs::create_dir_all(&wt).unwrap();
    for rel in ["src/shop.rs", "src/orders.rs", "README.md"] {
        write(
            &wt,
            rel,
            &std::fs::read_to_string(f.root.join(rel)).unwrap(),
        );
    }
    wt
}

#[test]
fn a_worktree_keeps_only_what_differs_from_the_project_and_sees_the_rest_of_it() {
    let f = fixture("ci-wt");
    let base = f.target();
    f.svc.ensure_fresh(&base, true).unwrap();
    let base_symbols = f.svc.status("p1").symbols;
    let wt = worktree_of(&f);
    write(
        &wt,
        "src/shop.rs",
        &format!("{SHOP}pub fn only_here() {{}}\n"),
    );
    write(&wt, "src/new.rs", "pub fn brand_new() {}\n");
    std::fs::remove_file(wt.join("README.md")).unwrap();
    let t = Target {
        dir: wt.clone(),
        ..base.clone()
    };
    let done = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    // The two files that differ are cut; the one that is the same is not.
    assert_eq!(done.parsed, 2);
    let st = f.svc.status("p1");
    assert_eq!(st.worktrees, 1);
    // The project's own rows did not change; the worktree's are the two files, not the third.
    assert_eq!(st.symbols, base_symbols);
    assert!(st_delta(&f, &t) > 0 && st_delta(&f, &t) < base_symbols);
    let r = f.svc.reader(&t).unwrap();
    // Its own.
    assert_eq!(
        r.search("only_here", 5, None).unwrap().result[0].path,
        "src/shop.rs"
    );
    assert_eq!(
        r.search("brand_new", 5, None).unwrap().result[0].path,
        "src/new.rs"
    );
    // The project's, for the file that is the same; once, not twice.
    let orders = r.search("place_order", 5, None).unwrap().result;
    assert_eq!(orders.len(), 1);
    let checkout = r.search("checkout", 5, Some("function")).unwrap().result;
    assert_eq!(checkout.len(), 1);
    // A file the worktree removed is gone from its view.
    assert!(r.search("Paiement", 5, None).unwrap().result.is_empty());
    // The project does not see the worktree's.
    let rb = f.svc.reader(&base).unwrap();
    assert!(rb.search("only_here", 5, None).unwrap().result.is_empty());
    assert!(rb.search("brand_new", 5, None).unwrap().result.is_empty());
    assert_eq!(rb.search("Paiement", 5, None).unwrap().result.len(), 1);
}

/// The symbols the worktree holds of its own.
fn st_delta(f: &Fixture, t: &Target) -> u64 {
    let conn =
        rusqlite::Connection::open(DataDir::new(f.data.clone()).code_index_db("p1")).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM symbols WHERE scope = ?1",
        [t.scope()],
        |r| r.get::<_, i64>(0),
    )
    .unwrap() as u64
}

#[test]
fn a_file_changed_in_a_worktree_hides_the_projects_symbols_of_it_and_goes_back_when_it_is_the_same_again(
) {
    let f = fixture("ci-wt2");
    let base = f.target();
    f.svc.ensure_fresh(&base, true).unwrap();
    let wt = worktree_of(&f);
    let t = Target {
        dir: wt.clone(),
        ..base.clone()
    };
    f.svc.ensure_fresh(&t, true).unwrap();
    assert_eq!(st_delta(&f, &t), 0);
    write(&wt, "src/shop.rs", &SHOP.replace("charge", "bill"));
    f.svc.ensure_fresh(&t, true).unwrap();
    let r = f.svc.reader(&t).unwrap();
    assert!(r
        .search("charge", 5, Some("function"))
        .unwrap()
        .result
        .is_empty());
    assert_eq!(
        r.search("bill", 5, Some("function")).unwrap().result.len(),
        1
    );
    // Back to the project's content: no delta.
    write(&wt, "src/shop.rs", SHOP);
    f.svc.ensure_fresh(&t, true).unwrap();
    assert_eq!(st_delta(&f, &t), 0);
    let r = f.svc.reader(&t).unwrap();
    assert_eq!(
        r.search("charge", 5, Some("function"))
            .unwrap()
            .result
            .len(),
        1
    );
}

#[test]
fn the_delta_follows_the_project_when_it_changes_and_goes_with_a_worktree_that_is_gone() {
    let f = fixture("ci-wt3");
    let base = f.target();
    f.svc.ensure_fresh(&base, true).unwrap();
    let wt = worktree_of(&f);
    write(&wt, "src/shop.rs", &SHOP.replace("charge", "bill"));
    let t = Target {
        dir: wt.clone(),
        ..base.clone()
    };
    f.svc.ensure_fresh(&t, true).unwrap();
    assert!(st_delta(&f, &t) > 0);
    // The project's folder takes the worktree's change (a merge): the delta is empty.
    write(&f.root, "src/shop.rs", &SHOP.replace("charge", "bill"));
    f.svc.ensure_fresh(&t, true).unwrap();
    assert_eq!(st_delta(&f, &t), 0);
    // The worktree is removed: the next reading of the project drops its rows.
    std::fs::remove_dir_all(&wt).unwrap();
    f.svc.ensure_fresh(&base, true).unwrap();
    assert_eq!(f.svc.status("p1").worktrees, 0);
}

// ---- projects kept out ----

#[test]
fn a_file_the_fence_refuses_is_never_listed_read_indexed_nor_shown() {
    let f = fixture("ci-fence");
    write(&f.root, "vault/secret.rs", "pub fn leaked_token() {}\n");
    write(&f.root, "vault/notes.md", "# Leaked heading\n");
    let t = Target {
        fence: Fence::new(vec![f.root.join("vault")]),
        ..f.target()
    };
    let done = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!(done.seen, 3);
    let r = f.svc.reader(&t).unwrap();
    assert!(r.search("leaked_token", 5, None).unwrap().result.is_empty());
    assert!(r.search("Leaked", 5, None).unwrap().result.is_empty());
    let conn =
        rusqlite::Connection::open(DataDir::new(f.data.clone()).code_index_db("p1")).unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE path LIKE 'vault/%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
    // The same project with no fence does index them.
    let open = Target {
        project_id: "p2".into(),
        ..f.target()
    };
    assert_eq!(f.svc.ensure_fresh(&open, true).unwrap().unwrap().seen, 5);
}

#[test]
fn big_binary_and_generated_files_are_left_out() {
    let f = fixture("ci-skip");
    write(&f.root, "app.min.js", "function minified(){}\n");
    write(&f.root, "node_modules/x/index.js", "function dep(){}\n");
    write(
        &f.root,
        "types.d.ts",
        "declare function declared(): void;\n",
    );
    std::fs::write(f.root.join("bin.rs"), b"fn a() {}\0\0\0").unwrap();
    std::fs::write(f.root.join("huge.rs"), "fn h() {}\n".repeat(80_000)).unwrap();
    let t = f.target();
    let done = f.svc.ensure_fresh(&t, true).unwrap().unwrap();
    assert_eq!(done.seen, 3);
}

// ---- the setting and the core ----

mod core {
    use crate::core_tests::{harness, repo};

    #[tokio::test]
    async fn the_setting_is_off_by_default_and_turns_the_index_on_and_off() {
        let h = harness("ci-core1");
        let r = repo(&h.dir.join("proj"));
        std::fs::create_dir_all(r.join("src")).unwrap();
        std::fs::write(
            r.join("src/a.rs"),
            "pub fn alpha() { beta(); }\nfn beta() {}\n",
        )
        .unwrap();
        let p = h
            .core
            .create_project(
                &r.to_string_lossy(),
                "proj",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        assert_eq!(p.code_index, None);
        let info = h.core.code_index_info(&p.id).unwrap();
        assert!(!info.enabled);
        assert_eq!(info.status.files, 0);
        // Off: nothing is read.
        assert!(h.core.code_index(&p.id, None).await.is_err());
        assert!(!h.core.data.code_index_db(&p.id).exists());
        // On: read at once.
        let info = h.core.set_code_index(&p.id, Some(true)).await.unwrap();
        assert!(info.enabled);
        assert!(info.status.symbols >= 2, "{info:?}");
        assert!(info.status.bytes > 0);
        assert_eq!(h.core.project(&p.id).unwrap().code_index, Some(true));
        let reader = h.core.code_index(&p.id, None).await.unwrap();
        assert_eq!(
            reader.search("alpha", 3, None).unwrap().result[0].name,
            "alpha"
        );
        drop(reader);
        // A folder that is none of the project's is refused.
        let elsewhere = h.dir.join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        assert!(h
            .core
            .code_index(&p.id, Some(&elsewhere.to_string_lossy()))
            .await
            .is_err());
        // Off again: erased.
        let info = h.core.set_code_index(&p.id, Some(false)).await.unwrap();
        assert!(!info.enabled);
        assert_eq!(info.status.files, 0);
        assert!(!h.core.data.code_index_db(&p.id).exists());
    }

    #[tokio::test]
    async fn the_setting_is_saved_with_the_project() {
        let h = harness("ci-core2");
        let r = repo(&h.dir.join("proj2"));
        let p = h
            .core
            .create_project(
                &r.to_string_lossy(),
                "proj2",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        h.core.set_code_index(&p.id, Some(true)).await.unwrap();
        let json = serde_json::to_value(h.core.project(&p.id).unwrap()).unwrap();
        assert_eq!(json["codeIndexChoice"], true);
        assert!(json.get("codeIndex").is_none());
        let back: crate::model::Project = serde_json::from_value(json).unwrap();
        assert_eq!(back.code_index, Some(true));
        // A project saved before the setting follows the app's.
        let mut old = serde_json::to_value(h.core.project(&p.id).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("codeIndexChoice");
        let old: crate::model::Project = serde_json::from_value(old).unwrap();
        assert_eq!(old.code_index, None);
        // Following, it is written without a choice.
        h.core.set_code_index(&p.id, None).await.unwrap();
        let json = serde_json::to_value(h.core.project(&p.id).unwrap()).unwrap();
        assert!(json.get("codeIndexChoice").is_none(), "{json}");
    }

    /// ESC-293: the switch saved before (`codeIndex`): on stays a choice of the project's; off
    /// (nothing told the off by default from the off wanted) follows the app's setting.
    #[test]
    fn the_projects_saved_before_keep_on_as_their_choice_and_follow_the_app_when_off() {
        let blank = || serde_json::to_value(crate::core::blank_project("C:/x")).unwrap();
        let read = |v: serde_json::Value| {
            let mut json = blank();
            json["codeIndex"] = v;
            let mut p: crate::model::Project = serde_json::from_value(json).unwrap();
            p.settle_legacy();
            p
        };
        let on = read(serde_json::json!(true));
        assert_eq!(on.code_index, Some(true));
        assert!(on.code_index_on(false));
        let off = read(serde_json::json!(false));
        assert_eq!(off.code_index, None);
        assert!(!off.code_index_on(false));
        assert!(off.code_index_on(true));
        // Written again, the old key is gone, the choice kept.
        let json = serde_json::to_value(&on).unwrap();
        assert!(json.get("codeIndex").is_none());
        assert_eq!(json["codeIndexChoice"], true);
        // A choice of the new kind is never read as an old one.
        let mut json = blank();
        json["codeIndexChoice"] = serde_json::json!(false);
        let mut p: crate::model::Project = serde_json::from_value(json).unwrap();
        p.settle_legacy();
        assert_eq!(p.code_index, Some(false));
        assert!(!p.code_index_on(true));
    }

    async fn projects(h: &crate::core_tests::Harness, names: &[&str]) -> Vec<String> {
        let mut ids = Vec::new();
        for name in names {
            let r = repo(&h.dir.join(name));
            let p = h
                .core
                .create_project(
                    &r.to_string_lossy(),
                    name,
                    "oklch(0.7 0.1 200)",
                    false,
                    None,
                    true,
                )
                .await
                .unwrap();
            ids.push(p.id);
        }
        ids
    }

    async fn wait_until(mut ok: impl FnMut() -> bool) -> bool {
        for _ in 0..400 {
            if ok() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        false
    }

    /// ESC-293: the effective state, the project's choice ahead of the app's setting.
    #[tokio::test]
    async fn a_project_follows_the_apps_setting_until_it_chooses() {
        let h = harness("ci-follow");
        let ids = projects(&h, &["fa", "fb", "fc"]).await;
        let (a, b, c) = (&ids[0], &ids[1], &ids[2]);
        // b chosen off, c chosen on, a follows.
        h.core.set_code_index(b, Some(false)).await.unwrap();
        h.core.set_code_index(c, Some(true)).await.unwrap();
        let on = |id: &str| h.core.code_index_on(&h.core.project(id).unwrap());
        assert!(!on(a) && !on(b) && on(c));
        let info = h.core.code_index_info(a).unwrap();
        assert!(!info.chosen && !info.enabled);
        assert!(h.core.code_index_info(b).unwrap().chosen);

        // Turned on: the followers alone are read.
        let n = h.core.set_code_index_all(true).await.unwrap();
        assert_eq!(n, 1);
        assert!(h.core.settings.read().code_index_all);
        assert!(on(a) && !on(b) && on(c));
        let read = wait_until(|| h.core.code_index_info(a).unwrap().status.files > 0).await;
        assert!(read, "the follower was not read");
        assert!(wait_until(|| h.core.codeindex.queue.lock().is_empty()).await);
        assert!(!h.core.data.code_index_db(b).exists());
        assert!(h.core.code_index(a, None).await.is_ok());
        // The settings modal's draft, older, does not turn it back off.
        let mut draft = h.core.settings.read().clone();
        draft.code_index_all = false;
        h.core.save_settings(draft).unwrap();
        assert!(h.core.settings.read().code_index_all);

        // Back to the app's setting from a choice: b follows, and is read.
        let info = h.core.set_code_index(b, None).await.unwrap();
        assert!(info.enabled && !info.chosen);
        assert!(h.core.data.code_index_db(b).exists());

        // Turned off: the followers' index erased, the one chosen on kept.
        let n = h.core.set_code_index_all(false).await.unwrap();
        assert_eq!(n, 2);
        assert!(!h.core.data.code_index_db(a).exists());
        assert!(!h.core.data.code_index_db(b).exists());
        assert!(h.core.data.code_index_db(c).exists());
        assert!(on(c) && !on(a));
        assert!(h.core.code_index(a, None).await.is_err());
        // Saved: read back as it was left.
        let saved: crate::model::Settings =
            serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
        assert!(!saved.code_index_all);
    }

    /// ESC-293: the projects are read one at a time, never all at once, without the caller waiting.
    #[tokio::test]
    // Held on purpose across the awaits: the reading of the first project waits for it, on a
    // blocking thread of its own, never on this task.
    #[allow(clippy::await_holding_lock)]
    async fn turning_it_on_reads_the_followers_one_at_a_time() {
        let h = harness("ci-queue");
        let ids = projects(&h, &["qa", "qb", "qc"]).await;
        // The first project's reading held: the others wait their turn behind it.
        let lock = h.core.codeindex.lock_for_tests(&ids[0]);
        let guard = lock.lock();
        assert_eq!(h.core.set_code_index_all(true).await.unwrap(), 3);
        assert!(wait_until(|| h.core.code_index_info(&ids[0]).unwrap().reading).await);
        for id in &ids[1..] {
            let info = h.core.code_index_info(id).unwrap();
            assert!(info.waiting && !info.reading, "{info:?}");
            assert!(!h.core.data.code_index_db(id).exists());
        }
        drop(guard);
        let all = wait_until(|| {
            ids.iter()
                .all(|id| h.core.code_index_info(id).unwrap().status.files > 0)
        })
        .await;
        assert!(all, "not every follower was read");
        assert!(ids.iter().all(|id| {
            let i = h.core.code_index_info(id).unwrap();
            !i.reading && !i.waiting
        }));
    }

    /// A new project follows the app's setting: read when it is on.
    #[tokio::test]
    async fn a_project_added_with_the_apps_setting_on_is_read() {
        let h = harness("ci-new");
        h.core.set_code_index_all(true).await.unwrap();
        let ids = projects(&h, &["na"]).await;
        assert_eq!(h.core.project(&ids[0]).unwrap().code_index, None);
        let read = wait_until(|| h.core.code_index_info(&ids[0]).unwrap().status.files > 0).await;
        assert!(read);
    }

    #[tokio::test]
    async fn a_private_projects_neighbour_inside_its_folder_stays_out_of_its_index() {
        let h = harness("ci-core3");
        let outer = repo(&h.dir.join("outer"));
        let inner = outer.join("vault");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(outer.join("main.rs"), "pub fn public_fn() {}\n").unwrap();
        std::fs::write(inner.join("secret.rs"), "pub fn secret_fn() {}\n").unwrap();
        let p = h
            .core
            .create_project(
                &outer.to_string_lossy(),
                "outer",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        let v = h
            .core
            .create_project(
                &inner.to_string_lossy(),
                "vault",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        {
            let mut projects = h.core.projects.write();
            projects.iter_mut().find(|x| x.id == v.id).unwrap().private = true;
        }
        let info = h.core.set_code_index(&p.id, Some(true)).await.unwrap();
        // The settings say which of its folders is left out (ESC-254); the private one leaves out nothing of its own.
        assert_eq!(info.excluded, ["vault"]);
        assert!(h.core.code_index_info(&v.id).unwrap().excluded.is_empty());
        let r = h.core.code_index(&p.id, None).await.unwrap();
        assert_eq!(r.search("public_fn", 3, None).unwrap().result.len(), 1);
        assert!(r
            .search("secret_fn", 3, None)
            .unwrap()
            .result
            .iter()
            .all(|h| h.name != "secret_fn"));
        // The private project indexes its own folder.
        h.core.set_code_index(&v.id, Some(true)).await.unwrap();
        let rv = h.core.code_index(&v.id, None).await.unwrap();
        assert_eq!(rv.search("secret_fn", 3, None).unwrap().result.len(), 1);
    }
}

// ---- performance, by hand ----

/// `cargo test codeindex_tests::on_this_repository -- --ignored --nocapture`: Escouade's own code.
#[test]
#[ignore = "measure, by hand"]
fn on_this_repository_full_then_incremental_indexing_and_size() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let base = test_dir("ci-perf");
    let svc = Service::new(DataDir::new(base.join("data")));
    let t = Target {
        project_id: "escouade".into(),
        root: root.clone(),
        dir: root.clone(),
        fence: Fence::default(),
        exclude: vec![root.join(".claude")],
    };
    let full = svc.ensure_fresh(&t, true).unwrap().unwrap();
    let st = svc.status("escouade");
    println!(
        "full: {} files read, {} ms; {} symbols, {} links; database {:.1} MB",
        full.parsed,
        full.millis,
        st.symbols,
        st.links,
        st.bytes as f64 / 1_048_576.0
    );
    let none = svc.ensure_fresh(&t, true).unwrap().unwrap();
    println!(
        "nothing changed: {} files parsed, {} ms",
        none.parsed, none.millis
    );
    let touched = root.join("src-tauri/src/codeindex/mod.rs");
    let text = std::fs::read_to_string(&touched).unwrap();
    let new_mtime = std::fs::File::options().write(true).open(&touched).unwrap();
    new_mtime
        .set_modified(std::time::SystemTime::now())
        .unwrap();
    drop(new_mtime);
    assert_eq!(text, std::fs::read_to_string(&touched).unwrap());
    let inc = svc.ensure_fresh(&t, true).unwrap().unwrap();
    println!("one file touched: {} parsed, {} ms", inc.parsed, inc.millis);
    let r = svc.reader(&t).unwrap();
    for q in [
        "worktree base branch",
        "fence private project files",
        "agent turn cost",
    ] {
        let a = r.task_context(q, 8).unwrap();
        println!(
            "context \"{q}\": {} entry points, {} files = {} tokens against {} (saved {} %)",
            a.result.entry_points.len(),
            a.saving.files,
            a.saving.files_tokens,
            a.saving.answer_tokens,
            a.saving.saved_percent
        );
    }
}

mod events {
    use crate::core_tests::{harness, repo};

    #[tokio::test]
    async fn a_git_event_or_the_end_of_a_turn_reads_what_changed_without_waiting_for_a_query() {
        let h = harness("ci-event");
        let r = repo(&h.dir.join("proj"));
        std::fs::write(r.join("a.rs"), "pub fn alpha() {}\n").unwrap();
        let p = h
            .core
            .create_project(
                &r.to_string_lossy(),
                "proj",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        // Off: an event reads nothing.
        h.core.code_index_event(&p.id);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert_eq!(h.core.code_index_info(&p.id).unwrap().status.files, 0);
        h.core.set_code_index(&p.id, Some(true)).await.unwrap();
        let before = h.core.code_index_info(&p.id).unwrap().status.symbols;
        std::fs::write(r.join("b.rs"), "pub fn beta() {}\npub fn gamma() {}\n").unwrap();
        h.core.code_index_event(&p.id);
        let mut after = before;
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            after = h.core.code_index_info(&p.id).unwrap().status.symbols;
            if after != before {
                break;
            }
        }
        assert_eq!(after, before + 2);
    }

    // ---- the rule in the agents' instructions (ESC-253) ----

    #[tokio::test]
    async fn the_agents_instructions_carry_the_rule_only_when_the_index_is_on() {
        use crate::core::AgentOptions;
        use crate::engine::{escaped_len, Engine, PROTOCOL_BUDGET};
        use crate::model::{Criterion, Ticket};
        let h = harness("ci-rule");
        let r = repo(&h.dir.join("proj"));
        let p = h
            .core
            .create_project(
                &r.to_string_lossy(),
                "proj",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        // An agent with no ticket: no rule while the index is off, only the shared memory's line
        // (ESC-255) every agent is told.
        let plain = h
            .core
            .create_agent_with(&p.id, AgentOptions::default())
            .await
            .unwrap();
        let told = plain.meta.append_prompt.clone().unwrap();
        assert!(!told.contains("index_search"), "{told}");
        assert!(told.contains("search_memory"), "{told}");
        h.core.set_code_index(&p.id, Some(true)).await.unwrap();
        let on = h
            .core
            .create_agent_with(&p.id, AgentOptions::default())
            .await
            .unwrap();
        let rule = on.meta.append_prompt.clone().unwrap();
        for tool in [
            "index_search",
            "index_context",
            "index_callers",
            "index_callees",
            "index_impact",
        ] {
            assert!(rule.contains(tool), "{tool} in {rule}");
        }
        assert!(rule.contains("grep"));

        // A ticket's protocol keeps its room: the rule is in, the criteria give way, and nothing
        // is added when the index is off.
        let long = "critère très long ".repeat(12);
        let t = Ticket {
            key: "ESC-1".into(),
            title: "Un ticket".into(),
            criteria: (0..40)
                .map(|i| Criterion {
                    text: format!("{i} {long}"),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let project = h.core.project(&p.id).unwrap();
        let with =
            h.core
                .ticket_instructions(&project, crate::i18n::Lang::Fr, &t, None, false, None);
        assert!(with.contains("index_search") && with.contains("ESC-1"));
        assert!(!with.contains('\n'));
        // With the shared memory's line added after it (`core::with_memory`), it still fits.
        let whole = format!(
            "{with} {}",
            crate::memory::instruction(crate::i18n::Lang::Fr)
        );
        assert!(
            escaped_len(&whole) <= PROTOCOL_BUDGET,
            "{}",
            escaped_len(&whole)
        );
        let _ = crate::engine::CLAUDE.instructions_room();
        h.core.set_code_index(&p.id, Some(false)).await.unwrap();
        let project = h.core.project(&p.id).unwrap();
        let without =
            h.core
                .ticket_instructions(&project, crate::i18n::Lang::Fr, &t, None, false, None);
        assert!(!without.contains("index_search"));
        assert!(without.contains("ESC-1"));
    }
}
