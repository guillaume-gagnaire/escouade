//! The jokers of `permissions.commands` (ESC-528, §5.3 of docs/PLUGINS-CONTRACT-STUDY.md): each a whole word of a closed
//! form, never an option nor `--`, the entry of a fixed length; `{pid}` one of the programs alive this instance runs,
//! `{path:temp}` and `{path:plugin}` a file under their root, resolved by the host and given as its real absolute path.
//! The hostile cases are a table, run the same under Windows and Unix; each refusal is said in the journal, with why.

use crate::procs::tests::{bed, words, Bed};
use crate::procs::{allowed, Entry, Held};
use serde_json::{json, Value};
use std::path::Path;

/// Whether `asked` runs under the entry `line`, for a host with the pids `pids` alive and the folders `temp`, `plugin`;
/// the words it would run.
fn run_words(
    line: &str,
    asked: &[String],
    pids: &[u32],
    temp: &Path,
    plugin: &Path,
) -> Result<Vec<String>, String> {
    let held = Held {
        pids: &|p| pids.contains(&p),
        temp,
        plugin,
    };
    allowed(&[Entry::of(line)], asked, &held).map(|m| m.words)
}

fn one(line: &str, word: &str) -> Result<Vec<String>, String> {
    let mut asked = words(line);
    let at = asked
        .iter()
        .position(|w| w.starts_with('{'))
        .expect("a joker");
    asked[at] = word.to_string();
    let nowhere = Path::new("");
    run_words(line, &asked, &[41_823], nowhere, nowhere)
}

#[test]
fn each_joker_admits_one_word_of_its_form_and_nothing_else() {
    let long = "a".repeat(64);
    let longer = "a".repeat(65);
    // (entry, word, admitted)
    let table: Vec<(&str, &str, bool)> = vec![
        ("tool {int}", "0", true),
        ("tool {int}", "7", true),
        ("tool {int}", "4294967295", true),
        ("tool {int}", "4294967296", false),
        ("tool {int}", "99999999999", false),
        ("tool {int}", "01", false),
        ("tool {int}", "00", false),
        ("tool {int}", "-1", false),
        ("tool {int}", "+1", false),
        ("tool {int}", "0x1F", false),
        ("tool {int}", "1e3", false),
        ("tool {int}", "1.0", false),
        ("tool {int}", " 1", false),
        ("tool {int}", "1 ", false),
        ("tool {int}", "1\n", false),
        ("tool {int}", "1\0", false),
        ("tool {int}", "\u{0661}", false),
        ("tool {int}", "", false),
        ("tool {int}", "--", false),
        ("tool {int}", "{int}", false),
        ("tool {port}", "1", true),
        ("tool {port}", "65535", true),
        ("tool {port}", "0", false),
        ("tool {port}", "65536", false),
        ("tool {port}", "080", false),
        ("tool {port}", "-80", false),
        ("tool {name}", "tap0", true),
        ("tool {name}", "a.b-c_d", true),
        ("tool {name}", "_x", true),
        ("tool {name}", "9lives", true),
        ("tool {name}", long.as_str(), true),
        ("tool {name}", longer.as_str(), false),
        ("tool {name}", "-x", false),
        ("tool {name}", "--up", false),
        ("tool {name}", "--", false),
        ("tool {name}", ".", false),
        ("tool {name}", "..", false),
        ("tool {name}", ".hidden", false),
        ("tool {name}", "a/b", false),
        ("tool {name}", "a\\b", false),
        ("tool {name}", "C:", false),
        ("tool {name}", "a=b", false),
        ("tool {name}", "a@b", false),
        ("tool {name}", "$HOME", false),
        ("tool {name}", "%PATH%", false),
        ("tool {name}", "`id`", false),
        ("tool {name}", "\"x\"", false),
        ("tool {name}", "'x'", false),
        ("tool {name}", "a^b", false),
        ("tool {name}", "a&b", false),
        ("tool {name}", "a|b", false),
        ("tool {name}", "<x", false),
        ("tool {name}", ">x", false),
        ("tool {name}", "*", false),
        ("tool {name}", "a?", false),
        ("tool {name}", "~", false),
        ("tool {name}", "a b", false),
        ("tool {name}", "a\tb", false),
        ("tool {name}", "a\u{1b}b", false),
        ("tool {name}", "\u{e9}t\u{e9}", false),
        ("tool {name}", "NUL", false),
        ("tool {name}", "con", false),
        ("tool {name}", "Com1", false),
        ("tool {name}", "LPT9.txt", false),
        ("tool {name}", "nul.tar.gz", false),
        ("tool {name}", "aux", false),
        ("tool {name}", "prn", false),
        ("tool {name}", "COM", true),
        ("tool {name}", "console", true),
        ("tool {name}", "", false),
        ("tool {pid}", "41823", true),
        ("tool {pid}", "41824", false),
        ("tool {pid}", "041823", false),
        ("tool {pid}", "-41823", false),
        ("tool {pid}", "0", false),
        ("tool {pid}", "1", false),
    ];
    for (line, word, admitted) in table {
        let r = one(line, word);
        assert_eq!(r.is_ok(), admitted, "{line} with {word:?}: {r:?}");
        if let Err(why) = r {
            assert!(why.contains("is not in permissions.commands: "), "{why}");
        }
    }
}

#[test]
fn an_entry_keeps_its_length_a_joker_is_never_an_option_and_a_last_star_is_as_it_was() {
    let nowhere = Path::new("");
    let ok = |line: &str, asked: &str| run_words(line, &words(asked), &[], nowhere, nowhere);
    // A word more, a word less: refused, and said.
    let why = ok("tool -n {int} up", "tool -n 3 up now").unwrap_err();
    assert!(why.contains("5 words where the entry has 4"), "{why}");
    let why = ok("tool -n {int} up", "tool -n 3").unwrap_err();
    assert!(why.contains("3 words where the entry has 4"), "{why}");
    // An option slipped in for a joker: refused, whatever it holds.
    for asked in [
        "tool -n -x up",
        "tool -n --up up",
        "tool -n -- up",
        "tool -n -1 up",
    ] {
        let why = ok("tool -n {int} up", asked).unwrap_err();
        assert!(
            why.contains("an option") || why.contains("not a whole number"),
            "{asked}: {why}"
        );
    }
    // `--` is a word of its own when the entry says it.
    assert_eq!(
        ok("tool log -- {name}", "tool log -- web").unwrap(),
        words("tool log -- web")
    );
    assert!(ok("tool log -- {name}", "tool log -- --").is_err());
    // A joker in a word, an unknown joker, a joker first: they match nothing, not even themselves.
    for (line, asked) in [
        ("tool --pid={int}", "tool --pid=3"),
        ("tool --pid={int}", "tool --pid={int}"),
        ("tool {float}", "tool 1.5"),
        ("tool {float}", "tool {float}"),
        ("tool {path:project}", "tool a.txt"),
        ("tool {path:agent}", "tool a.txt"),
        ("{name} x", "tool x"),
        ("{name} x", "{name} x"),
        ("{pid}", "41823"),
        ("*", "rm -rf x"),
        ("*", "*"),
    ] {
        assert!(ok(line, asked).is_err(), "{line} admits {asked}");
    }
    // A last `*` lets any words follow its head, its jokers checked; never in its head.
    assert_eq!(
        ok("nsenter -t {int} *", "nsenter -t 7 ip addr").unwrap(),
        words("nsenter -t 7 ip addr")
    );
    assert!(ok("nsenter -t {int} *", "nsenter -t x ip").is_err());
    assert!(ok("nsenter -t {int} *", "nsenter -t").is_err());
    // A literal is byte for byte.
    assert!(ok("tool up", "tool UP").is_err());
    assert!(ok("tool up", "tool up\u{200b}").is_err());
}

/// What an Escouade that does not serve the jokers did (2.2.4 and before, `procs::allowed` word for word): a joker was a
/// literal word, so the entry ran nothing a plugin would ask. Never wider: the plugin says `escouade.min` and
/// `capabilities.typedCommands` tells it.
#[test]
fn an_app_that_does_not_serve_the_jokers_refuses_the_entry_and_never_widens_it() {
    let before = |line: &str, asked: &str| words(line) == words(asked);
    for (line, asked) in [
        (
            "slirp4netns --configure --mtu=65520 {pid} tap0",
            "slirp4netns --configure --mtu=65520 41823 tap0",
        ),
        (
            "unshare -Urn openvpn --config {path:temp} --script-security 1",
            "unshare -Urn openvpn --config /tmp/x.ovpn --script-security 1",
        ),
        ("tool {int}", "tool 3"),
        ("tool {name}", "tool web"),
        ("tool {port}", "tool 8080"),
    ] {
        assert!(!before(line, asked), "{asked}");
    }
    // And this host does not take a joker as its own name either.
    let nowhere = Path::new("");
    for line in [
        "tool {int}",
        "tool {pid}",
        "tool {name}",
        "tool {port}",
        "tool {path:temp}",
        "tool {path:plugin}",
    ] {
        assert!(
            run_words(line, &words(line), &[], nowhere, nowhere).is_err(),
            "{line}"
        );
    }
}

/// A file under `base`, made with its folders.
fn file(base: &Path, rel: &str, text: &str) {
    let p = base.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

#[test]
fn a_path_is_a_file_under_its_root_resolved_by_the_host_and_given_whole() {
    let base =
        std::env::temp_dir().join(format!("plugin-jokers-{}", uuid::Uuid::new_v4().simple()));
    let (temp, plugin) = (base.join("temp"), base.join("plugin"));
    file(&temp, "conf.ovpn", "client\n");
    file(&temp, "sub/x.conf", "x\n");
    file(&temp, ".hidden.conf", "x\n");
    file(&plugin, "data/rules.txt", "r\n");
    file(&plugin, ".env", "SECRET=1\n");
    file(&base, "secret/key.txt", "hors du dossier\n");
    std::fs::create_dir_all(temp.join("dir")).unwrap();
    // A link (a junction under Windows) that leads out of the root, one that stays in it.
    crate::files::tests::link_folder(&base.join("secret"), &temp.join("out"));
    crate::files::tests::link_folder(&temp.join("sub"), &temp.join("in"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(base.join("secret/key.txt"), temp.join("key.txt")).unwrap();
    let real = |p: &Path| {
        let r = std::fs::canonicalize(p).unwrap();
        let t = r.to_string_lossy().into_owned();
        t.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(t)
    };
    let run = |line: &str, word: &str| {
        let mut asked = words(line);
        asked[1] = word.to_string();
        run_words(line, &asked, &[], &temp, &plugin)
    };
    // Admitted: the real absolute path takes its place.
    for (line, word, at) in [
        ("tool {path:temp}", "conf.ovpn", temp.join("conf.ovpn")),
        ("tool {path:temp}", "sub/x.conf", temp.join("sub/x.conf")),
        (
            "tool {path:temp}",
            ".hidden.conf",
            temp.join(".hidden.conf"),
        ),
        ("tool {path:temp}", "in/x.conf", temp.join("sub/x.conf")),
        (
            "tool {path:plugin}",
            "data/rules.txt",
            plugin.join("data/rules.txt"),
        ),
    ] {
        let got = run(line, word).unwrap_or_else(|e| panic!("{word}: {e}"));
        assert_eq!(got[1], real(&at), "{word}");
        assert!(
            Path::new(&got[1]).is_absolute() && !got[1].starts_with('-'),
            "{got:?}"
        );
    }
    // Refused, each with why.
    let refused: Vec<(&str, &str, &str)> = vec![
        ("tool {path:temp}", "/etc/passwd", "absolute"),
        (
            "tool {path:temp}",
            "C:\\Windows\\win.ini",
            "letters, digits",
        ),
        ("tool {path:temp}", "C:/Windows/win.ini", "letters, digits"),
        ("tool {path:temp}", "c:conf.ovpn", "letters, digits"),
        (
            "tool {path:temp}",
            "\\\\server\\share\\x",
            "letters, digits",
        ),
        ("tool {path:temp}", "//server/share/x", "absolute"),
        ("tool {path:temp}", "\\\\?\\C:\\x", "letters, digits"),
        ("tool {path:temp}", "\\\\.\\NUL", "letters, digits"),
        ("tool {path:temp}", "~/x", "letters, digits"),
        ("tool {path:temp}", "~", "letters, digits"),
        ("tool {path:temp}", "../secret/key.txt", ". or .."),
        ("tool {path:temp}", "sub/../conf.ovpn", ". or .."),
        ("tool {path:temp}", "./conf.ovpn", ". or .."),
        ("tool {path:temp}", "sub//x.conf", "empty part"),
        ("tool {path:temp}", "sub/", "empty part"),
        ("tool {path:temp}", "-conf", "an option"),
        ("tool {path:temp}", "sub/-x", "starts with -"),
        ("tool {path:temp}", "--config", "an option"),
        ("tool {path:temp}", "conf.ovpn:stream", "letters, digits"),
        ("tool {path:temp}", "conf.ovpn ", "letters, digits"),
        ("tool {path:temp}", "PROGRA~1", "letters, digits"),
        ("tool {path:temp}", "NUL", "device of Windows"),
        ("tool {path:temp}", "sub/con.txt", "device of Windows"),
        ("tool {path:temp}", "out/key.txt", "leads out of its folder"),
        ("tool {path:temp}", "missing.conf", "no such file"),
        ("tool {path:temp}", "dir", "no file"),
        ("tool {path:temp}", "", "empty part"),
        ("tool {path:plugin}", ".env", "hidden"),
        ("tool {path:plugin}", "../temp/conf.ovpn", ". or .."),
        ("tool {path:plugin}", "conf.ovpn", "no such file"),
    ];
    for (line, word, why) in refused {
        let r = run(line, word);
        let said = r.as_ref().expect_err(word);
        assert!(said.contains(why), "{word:?}: {said}");
    }
    #[cfg(unix)]
    {
        let said = run("tool {path:temp}", "key.txt").unwrap_err();
        assert!(said.contains("leads out of its folder"), "{said}");
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// What `proc/spawn` answered: its proc and its pid.
fn spawned(b: &Bed, params: Value) -> (u64, u32) {
    let r = b.procs.serve("proc/spawn", &params).unwrap();
    (
        r["proc"].as_u64().unwrap(),
        u32::try_from(r["pid"].as_u64().unwrap()).unwrap(),
    )
}

/// A program that waits on its input, in the bed's worktree, made a repository.
fn waiting(b: &Bed) -> (u64, u32) {
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(b.base.join("wt"))
        .status()
        .unwrap();
    spawned(
        b,
        json!({ "command": "git", "args": ["cat-file", "--batch"], "dir": "agent", "agent": "a" }),
    )
}

#[tokio::test]
async fn a_pid_is_one_of_its_own_programs_alive_never_anothers_nor_one_ended() {
    let list = ["git cat-file --batch", "git --version", "git var {pid}"];
    let mut b = bed(&list);
    let mut other = bed(&list);
    let (id, pid) = waiting(&b);
    assert!(pid > 0);
    // Its own, alive: it runs, with the very words, said in the journal.
    let (var, _) = spawned(
        &b,
        json!({ "command": "git", "args": ["var", pid.to_string()] }),
    );
    let (_, code) = b.run(var, "stderr").await;
    assert_ne!(code, Value::Null);
    let ran = format!("proc/spawn: ran git var {pid} in temp");
    assert!(
        b.told.lock().iter().any(|l| l == &ran),
        "{:?}",
        b.told.lock()
    );
    // Another instance's program, the test's own process, a pid of nobody: refused.
    let (other_id, other_pid) = waiting(&other);
    let ended = {
        let (v, pid) = spawned(&b, json!({ "command": "git", "args": ["--version"] }));
        b.run(v, "stdout").await;
        pid
    };
    for p in [other_pid, std::process::id(), 1, ended] {
        b.refused(json!({ "command": "git", "args": ["var", p.to_string()] }));
    }
    // Once it ends, its pid is no `{pid}` any more.
    b.procs.serve("proc/kill", &json!({ "proc": id })).unwrap();
    b.run(id, "stdout").await;
    b.refused(json!({ "command": "git", "args": ["var", pid.to_string()] }));
    let told = b.told.lock().clone();
    let refusals: Vec<&String> = told.iter().filter(|l| l.contains("refused")).collect();
    assert_eq!(refusals.len(), 5, "{told:?}");
    assert!(
        refusals
            .iter()
            .all(|l| l.contains("no program it runs is alive with this pid")),
        "{refusals:?}"
    );
    other
        .procs
        .serve("proc/kill", &json!({ "proc": other_id }))
        .unwrap();
    other.run(other_id, "stdout").await;
}

/// The hostile cases through `proc/spawn` itself: nothing runs, each refusal is in the journal with why.
#[tokio::test]
async fn the_hostile_cases_run_nothing_and_each_refusal_says_why_in_the_journal() {
    let mut b = bed(&[
        "git hash-object {path:temp}",
        "git var {name}",
        "git var {int} {port}",
    ]);
    file(&b.base.join("temp"), "conf.ovpn", "client\n");
    file(&b.base, "secret.txt", "hors du dossier\n");
    let cases: Vec<(Value, &str)> = vec![
        (json!(["hash-object", "-w"]), "an option"),
        (json!(["hash-object", "--stdin"]), "an option"),
        (json!(["hash-object", "--"]), "an option"),
        (json!(["hash-object", "../secret.txt"]), ". or .."),
        (json!(["hash-object", "/etc/passwd"]), "absolute"),
        (
            json!(["hash-object", "C:\\Windows\\win.ini"]),
            "letters, digits",
        ),
        (
            json!(["hash-object", "\\\\server\\share\\x"]),
            "letters, digits",
        ),
        (
            json!(["hash-object", "conf.ovpn", "-w"]),
            "4 words where the entry has 3",
        ),
        (json!(["hash-object"]), "2 words where the entry has 3"),
        (json!(["var", "--list"]), "an option"),
        (json!(["var", "NUL"]), "device of Windows"),
        (json!(["var", "a b"]), "letters, digits"),
        (json!(["var", "1", "0"]), "out of 1 to 65535"),
        (json!(["var", "1", "0x50"]), "not a whole number"),
        (json!(["var", "1", "+80"]), "not a whole number"),
        (json!(["var", "1", "080"]), "not a whole number"),
        (json!(["var", "1", "-80"]), "an option"),
    ];
    let n = cases.len();
    for (args, _) in &cases {
        b.refused(json!({ "command": "git", "args": args }));
    }
    assert_eq!(b.procs.running(), 0);
    let told = b.told.lock().clone();
    assert_eq!(told.len(), n, "{told:?}");
    for (line, (args, why)) in told.iter().zip(&cases) {
        assert!(line.starts_with("proc/spawn: refused, git "), "{line}");
        assert!(line.contains(why), "{args}: {line}");
    }
    // What holds runs, on the real path the host resolved.
    let id = b.spawn(json!({ "command": "git", "args": ["hash-object", "conf.ovpn"] }));
    let (out, code) = b.run(id, "stdout").await;
    assert_eq!((out.trim().len(), code), (40, json!(0)), "{out}");
    let real = std::fs::canonicalize(b.base.join("temp/conf.ovpn")).unwrap();
    let real = real.to_string_lossy();
    let real = real.strip_prefix(r"\\?\").unwrap_or(&real);
    let ran = format!("proc/spawn: ran git hash-object {real} in temp");
    assert!(
        b.told.lock().iter().any(|l| l == &ran),
        "{:?}",
        b.told.lock()
    );
}
