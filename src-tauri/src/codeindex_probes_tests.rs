//! The scores of the code index's probes (ESC-294): precision, recall, rank, tokens, reading.

use crate::codeindex::probes::score::{
    economy, parse_expect, read_count, score, tokens, windows, Alt, Item,
};

fn expect(items: &[&str]) -> Vec<Vec<Alt>> {
    items.iter().map(|s| parse_expect(s)).collect()
}

#[test]
fn an_expected_item_is_a_file_a_line_or_lines_and_its_alternatives() {
    assert_eq!(
        parse_expect("src/a.rs:10-20|src\\b.rs:3| docs/c.md "),
        vec![
            Alt {
                path: "src/a.rs".into(),
                from: 10,
                to: 20
            },
            Alt {
                path: "src/b.rs".into(),
                from: 3,
                to: 3
            },
            Alt {
                path: "docs/c.md".into(),
                from: 0,
                to: 0
            },
        ]
    );
    // What is not a range is part of the path: a whole file.
    assert_eq!(parse_expect("a.rs:x")[0].from, 0);
    assert_eq!(parse_expect("a.rs:9-3")[0].from, 0);
}

#[test]
fn precision_recall_and_rank_of_the_first_good_result() {
    let e = expect(&["src/a.rs:10-20", "src/b.rs:3|src/c.rs:7", "docs/d.md"]);
    let items = vec![
        Item::new("src/z.rs", 1),
        Item::new("src\\a.rs", 15),
        Item::new("src/a.rs", 12),
        Item::new("src/c.rs", 7),
        Item::new("src/a.rs", 21),
    ];
    let s = score(&e, &items);
    assert_eq!(s.results, 5);
    assert_eq!(s.good, 3);
    assert_eq!(s.found, 2);
    assert_eq!(s.expected, 3);
    assert!((s.precision - 0.6).abs() < 1e-9);
    assert!((s.recall - 2.0 / 3.0).abs() < 1e-9);
    assert_eq!(s.first_good, Some(2));
}

#[test]
fn a_whole_file_hits_any_line_and_a_file_result_hits_only_a_whole_file() {
    let e = expect(&["docs/d.md"]);
    assert_eq!(score(&e, &[Item::new("docs/d.md", 40)]).found, 1);
    assert_eq!(score(&e, &[Item::new("docs/d.md", 0)]).found, 1);
    let lines = expect(&["docs/d.md:3-5"]);
    assert_eq!(score(&lines, &[Item::new("docs/d.md", 0)]).found, 0);
}

#[test]
fn nothing_found_and_nothing_expected() {
    let e = expect(&["a.rs:1"]);
    let none = score(&e, &[]);
    assert_eq!(
        (none.precision, none.recall, none.first_good),
        (0.0, 0.0, None)
    );
    // Nothing must come out: right when nothing does, wrong for each result else.
    let empty: Vec<Vec<Alt>> = Vec::new();
    let quiet = score(&empty, &[]);
    assert_eq!((quiet.precision, quiet.recall), (1.0, 1.0));
    let noisy = score(&empty, &[Item::new("a.rs", 1)]);
    assert_eq!((noisy.precision, noisy.recall, noisy.good), (0.0, 1.0, 0));
}

#[test]
fn tokens_are_four_bytes_each_rounded_up() {
    assert_eq!(tokens(""), 0);
    assert_eq!(tokens("abcd"), 1);
    assert_eq!(tokens("abcde"), 2);
    // Bytes, not characters: « é » is two.
    assert_eq!(tokens("éé"), 1);
    assert_eq!(tokens("ééé"), 2);
}

#[test]
fn the_reading_merges_windows_of_one_file_and_keeps_the_first_results() {
    let items = vec![
        Item::new("a.rs", 30),
        Item::new("b.rs", 5),
        Item::new("a.rs", 55),
        Item::new("x.md", 0),
        Item::new("a.rs", 200),
        Item::new("c.rs", 9),
    ];
    // A whole file is read from its top.
    assert_eq!(
        windows(&items, 20, 5),
        vec![
            ("a.rs".to_string(), 10, 75),
            ("b.rs".to_string(), 1, 25),
            ("x.md".to_string(), 1, 41),
            ("a.rs".to_string(), 180, 220),
        ]
    );
    // A window between two others joins them; windows that do not touch stay apart.
    let bridged = vec![
        Item::new("a.rs", 10),
        Item::new("a.rs", 100),
        Item::new("a.rs", 55),
    ];
    assert_eq!(
        windows(&bridged, 25, 10),
        vec![("a.rs".to_string(), 1, 125)]
    );
    assert_eq!(windows(&bridged, 20, 10).len(), 3);
}

#[test]
fn one_answer_is_read_up_to_the_first_good_result_a_list_whole() {
    let flags = [false, false, true, false, true];
    assert_eq!(read_count(1, &flags), 3);
    assert_eq!(read_count(0, &flags), 3);
    assert_eq!(read_count(1, &[false, false]), 2);
    assert_eq!(read_count(4, &flags), 5);
    assert_eq!(read_count(1, &[]), 0);
}

#[test]
fn the_economy_is_signed() {
    assert_eq!(economy(10, 100), 90);
    assert_eq!(economy(150, 100), -50);
    assert_eq!(economy(5, 0), 0);
}

#[test]
fn a_run_is_compared_to_the_reference_by_tool_and_probe() {
    use crate::codeindex::probes::compared;
    use serde_json::json;
    let probe = |id: &str, tool: &str, recall: f64, tokens: u64| {
        json!({"id": id, "tool": tool, "tags": ["tâche"],
               "index": {"at10": {"recall": recall, "precision": recall}, "realTokens": tokens}})
    };
    let before = json!({"commit": "abc", "probes": [
        probe("a", "index_context", 0.0, 5000), probe("b", "index_callers", 1.0, 300)]});
    let after = json!({"commit": "def", "probes": [
        probe("a", "index_context", 1.0, 900), probe("b", "index_callers", 1.0, 280),
        probe("c", "index_search", 1.0, 10)]});
    let s = compared(&before, &after);
    assert!(s.contains("Face à la référence (abc)"), "{s}");
    assert!(
        s.contains("| context | 1 | 0 → 100 | 0 → 100 | 5000 → 900 |"),
        "{s}"
    );
    assert!(s.contains("| « tâche » | 2 | 50 → 100 |"), "{s}");
    // A probe the reference lacks is left out; only the moved recall is listed.
    assert!(!s.contains("| search |"), "{s}");
    assert!(s.contains("Rappel changé : a 0 → 100."), "{s}");
}
