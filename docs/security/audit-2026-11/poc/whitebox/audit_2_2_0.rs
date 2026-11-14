//! The attack tests of the security audit of Escouade 2.2.0 (2026-11, ESC-418), on the code of the tag v2.2.0.
//! `src-tauri/src/plugins/mod.rs` names this file: the app's tests compile it, and none runs unless asked (all ignored).
//!
//! The host of the plugins is a crate of its own (`escouade-plugins`, docs/CRATES.md): a test here reaches it by what
//! it makes public alone. Two tests go through another door for it: the check of a `version.json`, by the seed's; the
//! hooks and the refusals of a view, by the plugin that writes them.
//!
//!   cargo nextest run --lib -p escouade --run-ignored ignored-only -E 'test(audit_2_2_0)' --no-capture
//!
//! - `holds_*`: an attack that the code turns away (a control in place of the report).
//! - `limit_*`: a control that holds for what it names, and what it lets through (printed, and said in the report).
//! - `finding_*`: a weakness of the report, shown as it is (the test passes while it is there).
//! - `fuzz_*`: generated inputs for `AUDIT_FUZZ_SECONDS` seconds each.
//!
//! No payload here does anything but write a marker file in a folder of the test.

use super::files::{Access, AgentPlace, Agents, REFUSED as FS_REFUSED};
use super::harness::{check_launch, read_frame, write_files, Vars};
use super::journal::Kind;
use super::manifest::{self, Hand};
use super::procs::{allowed, forbidden_var, var_refused, Procs, ProcsOf, REFUSED as PROC_REFUSED};
use super::seed::{Entry as SeedEntry, Seed};
use super::store::{self, RevocationList};
use super::store_keys::{self, Refused};
use super::wasm::{self, Dns, Fetcher, Net, REFUSED as NET_REFUSED};
use escouade_plugin_protocol::engine as proto;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---- What the tests share ----------------------------------------------------------------------------------------

/// A folder of the test `name`, new, its links followed.
fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "audit-2026-11-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&d).unwrap();
    PathBuf::from(plain(&d.canonicalize().unwrap()))
}

/// `path` without the `\\?\` Windows' canonical paths start with.
fn plain(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

/// A link `link` to the folder `to`: a junction under Windows (no right needed), a symbolic link elsewhere.
fn link_folder(to: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(to, link).unwrap();
    #[cfg(windows)]
    assert!(std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        // `mklink` reads a `/` as the start of an option.
        .arg(plain(link).replace('/', "\\"))
        .arg(plain(to).replace('/', "\\"))
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap()
        .success());
}

fn file(path: impl AsRef<Path>, text: &str) -> proto::FileToWrite {
    proto::FileToWrite {
        path: path.as_ref().to_string_lossy().into_owned(),
        text: text.into(),
    }
}

fn var(name: &str, value: &str) -> proto::EnvVar {
    proto::EnvVar {
        name: name.into(),
        value: value.into(),
    }
}

/// A launch of a third party's plugin that writes `files`, judged for an agent whose folder is `home`.
fn judged(home: &Path, files: Vec<proto::FileToWrite>) -> Result<(), String> {
    let launch = proto::Launch {
        files,
        ..proto::Launch::default()
    };
    check_launch("poc", false, None, &[], false, Some(&plain(home)), &launch)
}

/// A small generator of the fuzz loops (xorshift64*): the same inputs at each run of a seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn pick<'a, T>(&mut self, of: &'a [T]) -> &'a T {
        &of[self.below(of.len())]
    }
}

/// How long each fuzz loop runs.
fn fuzz_time() -> Duration {
    let seconds = std::env::var("AUDIT_FUZZ_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    Duration::from_secs(seconds)
}

/// Runs `one` on generated inputs for `fuzz_time`, and says how many it ran.
fn fuzz(name: &str, mut one: impl FnMut(&mut Rng)) {
    let (start, mut rng, mut n) = (Instant::now(), Rng(0x9E37_79B9_7F4A_7C15), 0u64);
    while start.elapsed() < fuzz_time() {
        for _ in 0..64 {
            one(&mut rng);
            n += 1;
        }
    }
    eprintln!(
        "AUDIT-FUZZ {name}: {n} inputs in {:.1} s, no panic, every invariant held",
        start.elapsed().as_secs_f64()
    );
}

/// `seed` with a few of its bytes changed, cut, repeated or with others put in.
fn mutated(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    const WORDS: &[&[u8]] = &[
        b"..",
        b"../",
        b"..\\",
        b"\0",
        b"%00",
        b"%2e%2e%2f",
        b"\\\\?\\",
        b"//",
        b":",
        b"\"",
        b"'",
        b"{",
        b"}",
        b"[",
        b"]",
        b"\n",
        b"\r\n",
        b"\xff\xfe",
        b"99999999999999999999",
        b"-1",
        b"dev.escouade.",
        b"<script>",
        b"\xe2\x80\xae",
    ];
    let mut out = seed.to_vec();
    for _ in 0..=rng.below(6) {
        match rng.below(6) {
            0 if !out.is_empty() => {
                let at = rng.below(out.len());
                out[at] ^= 1 << rng.below(8);
            }
            1 if !out.is_empty() => {
                let at = rng.below(out.len());
                out.truncate(at);
            }
            2 => {
                let at = rng.below(out.len() + 1);
                let word = rng.pick(WORDS);
                out.splice(at..at, word.iter().copied());
            }
            3 if !out.is_empty() => {
                let (a, b) = (rng.below(out.len()), rng.below(out.len()));
                let (a, b) = (a.min(b), a.max(b));
                let piece = out[a..b].to_vec();
                out.splice(a..a, piece);
            }
            4 => out.push(rng.next() as u8),
            _ if !out.is_empty() => {
                let at = rng.below(out.len());
                out.remove(at);
            }
            _ => {}
        }
    }
    out
}

// ---- The launch a plugin describes (`plugins/harness.rs`) -----------------------------------------------------------

/// The files of a launch stay in the agent's own folder, as long as the path is read as it is written: `..`, a path
/// elsewhere, a relative one, a share, a path of another spelling.
#[test]
#[ignore = "audit 2026-11"]
fn holds_a_launch_file_out_of_the_agent_folder_is_refused() {
    let base = dir("launch-out");
    let home = base.join("home");
    std::fs::create_dir_all(&home).unwrap();
    assert_eq!(
        judged(&home, vec![file(home.join("config.toml"), "")]),
        Ok(())
    );
    assert_eq!(
        judged(&home, vec![file(home.join("sub").join("deep.json"), "")]),
        Ok(())
    );
    let h = plain(&home);
    let mut refused = vec![
        format!("{h}/../outside.txt"),
        format!("{h}/sub/../../outside.txt"),
        plain(&base.join("outside.txt")),
        "outside.txt".to_string(),
        "./outside.txt".to_string(),
        String::new(),
        format!("{h}-other/x.txt"),
    ];
    if cfg!(windows) {
        refused.extend([
            r"\\server\share\x.txt".to_string(),
            // The same folder, spelled another way: refused too (the comparison is by the letter).
            format!(r"\\?\{h}\..\outside.txt"),
            format!(r"\\?\{h}\x.txt"),
            format!(r"\\localhost\C$\{}", h.trim_start_matches(r"C:\")),
            h.to_uppercase() + r"\x.txt",
            r"C:outside.txt".to_string(),
            r"\outside.txt".to_string(),
        ]);
    } else {
        refused.push("/etc/cron.d/x".to_string());
    }
    for path in refused {
        // An upper-case spelling of a folder already in upper case is the folder itself.
        if Path::new(&path).starts_with(&h) && !path.contains("..") {
            continue;
        }
        let got = judged(&home, vec![file(&path, "")]);
        assert!(got.is_err(), "{path:?} was accepted");
    }
    // No folder for the agent: no file at all.
    let launch = proto::Launch {
        files: vec![file(home.join("x"), "")],
        ..proto::Launch::default()
    };
    assert!(check_launch("poc", false, None, &[], false, None, &launch).is_err());
}

/// SEC-06. The path of a launch's file is judged as it is written, its links not followed: through a link of the
/// agent's folder, the file is written outside it.
#[test]
#[ignore = "audit 2026-11"]
fn finding_a_launch_file_is_written_through_a_link_out_of_the_agent_folder() {
    let base = dir("launch-link");
    let (home, outside) = (base.join("home"), base.join("outside"));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    // What an agent's harness can make in its own folder.
    link_folder(&outside, &home.join("cache"));
    let files = vec![file(
        home.join("cache").join("audit-marker.txt"),
        "AUDIT-POC",
    )];
    assert_eq!(judged(&home, files.clone()), Ok(()));
    write_files(&proto::Launch {
        files,
        ..proto::Launch::default()
    })
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(outside.join("audit-marker.txt")).unwrap(),
        "AUDIT-POC",
        "the file went through the link, out of the agent's folder"
    );
}

/// SEC-02. The fence of a private project is « in the launch » as soon as each of its paths is a piece of any of its
/// texts and the plugin says `fenceApplied`: nothing tells that the harness is given it as a rule.
#[test]
#[ignore = "audit 2026-11"]
fn finding_the_fence_of_a_launch_is_judged_by_a_word_found_anywhere() {
    let fence = vec!["C:/clients/acme-secret".to_string()];
    let said_nowhere = proto::Launch {
        fence_applied: true,
        ..proto::Launch::default()
    };
    assert!(check_launch("poc", false, None, &fence, false, None, &said_nowhere).is_err());
    // A third party's word that it holds the fence itself is not taken.
    let by_client = proto::Launch {
        fence_by_client: true,
        ..proto::Launch::default()
    };
    assert!(check_launch("poc", false, None, &fence, false, None, &by_client).is_err());
    // The path in a variable no harness reads: accepted.
    let decoy = proto::Launch {
        fence_applied: true,
        env: vec![var("AUDIT_NOTE", "ignored: C:/clients/acme-secret")],
        ..proto::Launch::default()
    };
    assert_eq!(
        check_launch("poc", false, None, &fence, false, None, &decoy),
        Ok(())
    );
    // Or in an argument that allows it instead of refusing it.
    let reversed = proto::Launch {
        fence_applied: true,
        args: vec!["--add-dir".into(), "C:\\clients\\acme-secret".into()],
        ..proto::Launch::default()
    };
    assert_eq!(
        check_launch("poc", false, None, &fence, false, None, &reversed),
        Ok(())
    );
}

/// Escouade's token never goes in the arguments of a harness, which every program of the machine may read.
#[test]
#[ignore = "audit 2026-11"]
fn holds_the_token_of_escouade_in_an_argument_is_refused() {
    let token = "tok-0123456789abcdef0123456789abcdef";
    let launch = |args: &[&str]| proto::Launch {
        args: args.iter().map(|a| a.to_string()).collect(),
        ..proto::Launch::default()
    };
    let check = |l: &proto::Launch| check_launch("poc", false, Some(token), &[], false, None, l);
    assert!(check(&launch(&[
        "--header",
        &format!("Authorization: Bearer {token}")
    ]))
    .is_err());
    assert!(check(&launch(&[&format!("--token={token}")])).is_err());
    assert_eq!(check(&launch(&["--model", "x"])), Ok(()));
    // In a variable or a private file, where it belongs: taken.
    let by_env = proto::Launch {
        env: vec![var("HARNESS_TOKEN", token)],
        ..proto::Launch::default()
    };
    assert_eq!(check(&by_env), Ok(()));
}

// ---- `proc/*` and the variables a plugin names (`plugins/procs.rs`) -----------------------------------------------

/// The variables refused to a plugin: Escouade's own, the loader's, the PATH, in any case.
#[test]
#[ignore = "audit 2026-11"]
fn holds_the_variables_that_change_the_program_itself_are_refused() {
    for name in [
        "PATH",
        "Path",
        "PATHEXT",
        "COMSPEC",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "ld_audit",
        "DYLD_INSERT_LIBRARIES",
        "NODE_OPTIONS",
        "BASH_ENV",
        "ENV",
        "IFS",
        "ESCOUADE_DATA_DIR",
        "escouade_hook_token",
        "",
        "A=B",
        "A\0B",
    ] {
        assert!(forbidden_var(name), "{name:?} passes");
    }
}

/// What a plugin's programs are made of, for a plugin that declares `commands`.
struct Bed {
    procs: Procs,
    repo: PathBuf,
    _rx: tokio::sync::mpsc::UnboundedReceiver<wasm::Frame>,
    _tx: tokio::sync::mpsc::UnboundedSender<wasm::Frame>,
}

fn bed(commands: &[&str]) -> Bed {
    bed_with(commands, None)
}

/// The same, for a plugin of anyone's that declares `env` (`permissions.env`; None: it has none).
fn bed_with(commands: &[&str], env: Option<&[&str]>) -> Bed {
    let base = dir("procs");
    let repo = base.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let agents: Agents = Arc::default();
    agents.lock().insert(
        "a".into(),
        AgentPlace {
            cwd: repo.clone(),
            project: "open".into(),
            ..AgentPlace::default()
        },
    );
    let project = repo.clone();
    let procs = Procs::new(ProcsOf {
        commands: commands.iter().map(|c| c.to_string()).collect(),
        env: env.map(|names| names.iter().map(|n| n.to_string()).collect()),
        first_hand: false,
        temp: base.join("temp"),
        agents,
        projects: Arc::new(move |_: &str| Ok(project.clone())),
        agent_env: Vec::new(),
        back: tx.downgrade(),
        told: Arc::new(|_: Kind, _: &str| {}),
    });
    Bed {
        procs,
        repo,
        _rx: rx,
        _tx: tx,
    }
}

/// A plugin runs the programs of its `permissions.commands` word for word, and nothing else.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn holds_a_program_or_an_argument_not_declared_is_refused() {
    let b = bed(&["git status", "git log *"]);
    let refused = |params: Value| {
        let r = b.procs.serve("proc/spawn", &params);
        assert_eq!(
            r.as_ref().err().map(|e| e.0),
            Some(PROC_REFUSED),
            "{params}: {r:?}"
        );
    };
    refused(json!({ "command": "git", "args": ["push"], "dir": "agent", "agent": "a" }));
    refused(
        json!({ "command": "git", "args": ["status", "--porcelain"], "dir": "agent", "agent": "a" }),
    );
    refused(json!({ "command": "git", "dir": "agent", "agent": "a" }));
    refused(json!({ "command": "cmd", "args": ["/C", "echo x"], "dir": "agent", "agent": "a" }));
    refused(json!({ "command": "powershell", "args": ["-c", "1"], "dir": "agent", "agent": "a" }));
    refused(json!({ "command": "git status", "dir": "agent", "agent": "a" }));
    refused(json!({ "command": "GIT", "args": ["status"], "dir": "agent", "agent": "a" }));
    // A folder is a root, never a path; an agent it was not given is none.
    refused(json!({ "command": "git", "args": ["status"], "dir": "C:\\Windows" }));
    refused(json!({ "command": "git", "args": ["status"], "dir": "..", "agent": "a" }));
    refused(json!({ "command": "git", "args": ["status"], "dir": "agent", "agent": "zz" }));
    // The variables of `holds_the_variables_that_change_the_program_itself_are_refused`.
    refused(
        json!({ "command": "git", "args": ["status"], "dir": "agent", "agent": "a",
        "env": { "PATH": "C:\\evil" } }),
    );
    // A manifest never declares a path, nor a bare `*`.
    for line in [
        "C:\\Windows\\System32\\cmd.exe /C x",
        "/bin/sh -c x",
        "../x",
        "./x",
        "~/x",
        "*",
        "git * status",
    ] {
        assert!(!manifest::command_holds(line), "{line}");
    }
    assert!(!allowed(
        &[vec!["*".to_string()]],
        &["rm".to_string(), "-rf".to_string()]
    ));
}

// ---- `fs/*` (`plugins/files.rs`) ------------------------------------------------------------------------------------

struct Files {
    base: PathBuf,
    access: Access,
}

impl Files {
    /// A plugin that reads and writes its agent's folder, and reads its own.
    fn new() -> Self {
        Self::fenced(&["private", "wt/fenced"])
    }

    /// The same, the folders `fence` (under its base) fenced for its agent.
    fn fenced(fence: &[&str]) -> Self {
        let base = dir("fs");
        for d in ["plugin", "wt/src", "secret", "private"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        std::fs::write(base.join("wt/src/a.txt"), "dans le worktree").unwrap();
        std::fs::write(base.join("secret/key.txt"), "hors du dossier").unwrap();
        std::fs::write(base.join("private/c.txt"), "projet privé").unwrap();
        let manifest = manifest::check(
            &json!({
                "id": "dev.exemple.audit",
                "name": "Audit",
                "version": "1.0.0",
                "escouade": { "api": "1", "min": "2.2.0" },
                "permissions": { "files": [
                    { "root": "plugin", "access": "read" },
                    { "root": "agent", "access": "write" },
                ] },
            }),
            Hand::Third,
        )
        .manifest
        .expect("a manifest that holds");
        let agents: Agents = Arc::default();
        agents.lock().insert(
            "a".into(),
            AgentPlace {
                cwd: base.join("wt"),
                project: "p".into(),
                fence: crate::fence::Fence::new(fence.iter().map(|d| base.join(d)).collect()),
                ..AgentPlace::default()
            },
        );
        let access = Access::new(&manifest, &base.join("plugin"), base.join("temp"), agents);
        Files { base, access }
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, (i64, String)> {
        self.access.serve(method, &params, &|_: Kind, _: &str| {})
    }

    fn refused(&self, method: &str, params: Value) {
        let r = self.call(method, params.clone());
        assert_eq!(
            r.as_ref().err().map(|e| e.0),
            Some(FS_REFUSED),
            "{method} {params}: {r:?}"
        );
    }
}

/// A plugin never leaves the roots it declares: by `..`, an absolute path, a link, a stream, a device, another
/// spelling of a fenced folder.
#[test]
#[ignore = "audit 2026-11"]
fn holds_fs_never_leaves_its_roots_nor_enters_the_fence() {
    let f = Files::new();
    let read = |path: &str| json!({ "agent": "a", "root": "agent", "path": path });
    assert_eq!(
        f.call("fs/read", read("src/a.txt")).unwrap()["text"],
        "dans le worktree"
    );
    for path in [
        "../secret/key.txt",
        "src/../../secret/key.txt",
        "..",
        "~/.ssh/id_ed25519",
        "src/a.txt:stream",
        "a\0b",
        "",
    ] {
        f.refused("fs/read", read(path));
    }
    // An absolute path names no root, and one given without a root must lie under one.
    let secret = plain(&f.base.join("secret/key.txt"));
    f.refused("fs/read", read(&secret));
    f.refused("fs/read", json!({ "agent": "a", "path": secret }));
    f.refused(
        "fs/write",
        json!({ "agent": "a", "path": secret, "text": "x" }),
    );
    if cfg!(windows) {
        for path in [
            format!(r"\\?\{secret}"),
            format!(r"\\localhost\C$\{}", secret.trim_start_matches(r"C:\")),
            secret.to_uppercase(),
        ] {
            f.refused("fs/read", json!({ "agent": "a", "path": path }));
        }
    }
    // A link of the agent's folder that leads out of it: followed, then judged.
    link_folder(&f.base.join("secret"), &f.base.join("wt/out"));
    f.refused("fs/read", read("out/key.txt"));
    f.refused(
        "fs/write",
        json!({ "agent": "a", "root": "agent", "path": "out/audit-marker.txt", "text": "x" }),
    );
    assert!(!f.base.join("secret/audit-marker.txt").exists());
    // The fence of a private project: a folder of the agent's own root, and another through a link, in any case.
    std::fs::create_dir_all(f.base.join("wt/fenced")).unwrap();
    std::fs::write(f.base.join("wt/fenced/d.txt"), "privé").unwrap();
    f.refused("fs/read", read("fenced/d.txt"));
    if cfg!(windows) {
        f.refused("fs/read", read("FENCED/D.TXT"));
    }
    f.refused("fs/read", read("fenced/../fenced/d.txt"));
    link_folder(&f.base.join("private"), &f.base.join("wt/door"));
    f.refused("fs/read", read("door/c.txt"));
    // Windows' devices are no file of a root.
    if cfg!(windows) {
        for device in ["NUL", "CON", "src/nul", "COM1", "nul.txt"] {
            let r = f.call(
                "fs/write",
                json!({ "agent": "a", "root": "agent", "path": device, "text": "x" }),
            );
            // Read by its `\\?\` name: without it, Windows takes `NUL` in any folder for the device.
            let landed = PathBuf::from(format!(
                r"\\?\{}",
                plain(&f.base.join("wt").join(device)).replace('/', "\\")
            ));
            assert!(
                r.is_err() || landed.is_file(),
                "{device}: {r:?} wrote somewhere that is no file of the root"
            );
            // Noted in the report: the write lands in the root, as a file named like a device, which the usual tools of
            // Windows then neither open nor remove.
            if landed.is_file() {
                eprintln!(
                    "AUDIT fs/write {device}: a file of that name was made in the agent's folder"
                );
                std::fs::remove_file(&landed).unwrap();
            }
        }
    }
    // The plugin's own folder is read, never written; the manifest and hidden files are not read by `files/read`.
    f.refused(
        "fs/write",
        json!({ "root": "plugin", "path": "x.txt", "text": "x" }),
    );
    for path in [
        "escouade-plugin.json",
        ".env",
        "../wt/src/a.txt",
        "C:/x",
        "a/.git/config",
    ] {
        let r = crate::plugins::files::read(&f.base.join("plugin"), &json!({ "path": path }));
        assert_eq!(r.err().map(|e| e.0), Some(FS_REFUSED), "{path}");
    }
}

// ---- `net/*` (`plugins/wasm.rs`) ------------------------------------------------------------------------------------

/// The addresses a plugin never reaches, whatever name resolves to them.
#[test]
#[ignore = "audit 2026-11"]
fn holds_the_addresses_of_the_machine_and_of_the_local_network_are_forbidden() {
    for ip in [
        "127.0.0.1",
        "127.255.255.254",
        "0.0.0.0",
        "0.1.2.3",
        "10.0.0.1",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "224.0.0.1",
        "255.255.255.255",
        "240.0.0.1",
        "::1",
        "::",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::ffff:169.254.169.254",
        "64:ff9b::7f00:1",
        "64:ff9b::a00:1",
        "::127.0.0.1",
        "fe80::1",
        "fc00::1",
        "fd12:3456::1",
        "fec0::1",
        "ff02::1",
    ] {
        assert!(wasm::forbidden(ip.parse().unwrap()), "{ip} is reachable");
    }
    for ip in ["1.1.1.1", "8.8.8.8", "172.32.0.1", "2606:4700::1111"] {
        assert!(!wasm::forbidden(ip.parse().unwrap()), "{ip} is refused");
    }
}

/// Noted in the report (SEC-09, of no known effect): an IPv4 address carried in a 6to4 or a Teredo address is not read.
#[test]
#[ignore = "audit 2026-11"]
fn finding_an_ipv4_carried_in_6to4_or_teredo_is_not_judged() {
    // 2002:7f00:0001:: is 127.0.0.1 behind a 6to4 relay; 2002:c0a8:0101:: is 192.168.1.1.
    for ip in ["2002:7f00:1::", "2002:c0a8:101::", "2001:0:7f00:1::"] {
        assert!(!wasm::forbidden(ip.parse().unwrap()), "{ip} is refused now");
    }
}

fn to(addresses: &'static [&'static str]) -> Option<Dns> {
    Some(Arc::new(move |_| {
        addresses.iter().map(|a| a.parse().unwrap()).collect()
    }))
}

/// A URL of a plugin is refused unless it is https to a declared host: no address written, in any spelling, no host that
/// only looks like a declared one, and no declared name that resolves to the machine.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn holds_net_fetch_reaches_declared_hosts_alone() {
    let net = Net {
        dns: to(&["127.0.0.1"]),
        ..Net::default()
    };
    let declared = ["slack.com".to_string(), "*.exemple.dev".to_string()];
    let f = Fetcher::new("dev.exemple.audit", &declared, &net);
    for url in [
        "http://slack.com/",
        "ftp://slack.com/",
        "file:///C:/Windows/win.ini",
        "https://evilslack.com/",
        "https://slack.com.evil.example/",
        "https://evil.example/slack.com",
        "https://evil.example/?h=slack.com",
        "https://evil.example#@slack.com",
        "https://slack.com@evil.example/",
        "https://slack.com:pass@evil.example/",
        "https://evil.example\\@slack.com/",
        "https://exemple.dev/",
        "https://127.0.0.1/",
        "https://2130706433/",
        "https://0x7f.0.0.1/",
        "https://0177.0.0.1/",
        "https://127.1/",
        "https://[::1]/",
        "https://[::ffff:127.0.0.1]/",
        "https://localhost/",
        "https://10.0.0.1/",
        "https://169.254.169.254/latest/meta-data/",
        "not a url",
        "",
    ] {
        let got = f.fetch(&json!({ "url": url })).await;
        assert!(got.is_err(), "{url}: {got:?}");
    }
    // Declared, but the name leads to this machine: refused at the connection, each time it is made.
    for url in ["https://slack.com/api", "https://a.b.exemple.dev/"] {
        let (code, why) = f.fetch(&json!({ "url": url })).await.unwrap_err();
        assert_eq!(code, NET_REFUSED, "{url}: {why}");
        assert!(why.contains("127.0.0.1"), "{why}");
    }
}

/// A name that resolves to an address outside and one of the machine (a DNS rebinding answer): the connection never
/// goes to the machine's. A local server on every port the request could reach sees nothing.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn holds_a_rebinding_answer_never_reaches_the_machine() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    // 192.0.2.1 (TEST-NET-1) is routed nowhere: the request fails, and only there.
    let net = Net {
        dns: to(&["127.0.0.1", "192.0.2.1", "10.0.0.1"]),
        ..Net::default()
    };
    let f = Fetcher::new("dev.exemple.audit", &["slack.com".to_string()], &net).limited(
        manifest::Limits {
            fetch_seconds: Some(3),
            ..manifest::Limits::default()
        },
    );
    let got = f
        .fetch(&json!({ "url": format!("https://slack.com:{port}/") }))
        .await;
    assert!(got.is_err(), "{got:?}");
    assert!(
        listener.accept().is_err(),
        "the machine was reached through the name"
    );
}

/// What `permissions.network` may name. Noted in the report (SEC-09): a public suffix is a host like another.
#[test]
#[ignore = "audit 2026-11"]
fn finding_a_whole_top_level_domain_is_a_host_a_plugin_may_declare() {
    let net = Net::default();
    let f = Fetcher::new("dev.exemple.audit", &["com".to_string()], &net);
    assert!(f.covers("anything-at-all.com"));
    let wide = Fetcher::new("dev.exemple.audit", &["*.com".to_string()], &net);
    assert!(wide.covers("anything-at-all.com"));
    // A bare star covers nothing.
    let star = Fetcher::new("dev.exemple.audit", &["*".to_string()], &net);
    assert!(!star.covers("anything-at-all.com"));
    let read = manifest::check(
        &json!({
            "id": "dev.exemple.audit", "name": "Audit", "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.2.0" },
            "permissions": { "network": ["com"] },
        }),
        Hand::Third,
    );
    assert!(read.manifest.is_ok(), "the manifest refuses it now");
}

// ---- The seals of the store (`plugins/store_keys.rs`, `plugins/store.rs`) -----------------------------------------

// A key of the tests (store/tests/fake/minisign.mjs), never the store's: what `store_keys::tests` checks with.
const KEY: &str = "RWSdD12p0w147Es0or2M3l5uzT4vcBno4LQIrfq7geBz2SWuUFka1+MD";
const SIGNED: &str = r#"{"format":1,"id":"dev.exemple.slack","version":"1.2.0"}"#;
const SEAL: &str = "untrusted comment: signature from minisign secret key\nRUSdD12p0w147Bgh0ETL38EK152D4z9+O8rxLdMPtAIFnqKKT9dil/krYuno8cC1zRL7VNJ+ACvVdZBfX1ItjS0KKPBCimtTywQ=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.2.0 2026-10-29\nI5M6mYLq/SLrR885/61tWzWk1oQc5hO5wZxNVwiXsTonCGw5DbV8Tg3m/jtBT5DitE82TMqgHeR6Q/tnNp0zBQ==\n";

fn sealed(json: &[u8], seal: &str) -> Result<(), Refused> {
    store_keys::verified_with(&[KEY], json, seal, "dev.exemple.slack", "1.2.0")
}

/// One bit changed anywhere in what was signed, in the signature or in its trusted comment: no seal.
#[test]
#[ignore = "audit 2026-11"]
fn holds_one_bit_changed_in_a_version_or_its_signature_breaks_the_seal() {
    assert_eq!(sealed(SIGNED.as_bytes(), SEAL), Ok(()));
    let mut tried = 0;
    for at in 0..SIGNED.len() {
        for bit in 0..8 {
            let mut bytes = SIGNED.as_bytes().to_vec();
            bytes[at] ^= 1 << bit;
            assert_eq!(sealed(&bytes, SEAL), Err(Refused::NotSigned), "byte {at}");
            tried += 1;
        }
    }
    // One byte more, one less, nothing at all.
    for bytes in [
        &SIGNED.as_bytes()[1..],
        &SIGNED.as_bytes()[..SIGNED.len() - 1],
        b"",
    ] {
        assert_eq!(sealed(bytes, SEAL), Err(Refused::NotSigned));
    }
    let mut longer = SIGNED.as_bytes().to_vec();
    longer.push(b' ');
    assert_eq!(sealed(&longer, SEAL), Err(Refused::NotSigned));
    // The signature: every character of its file replaced in turn.
    for at in 0..SEAL.len() {
        if !SEAL.is_char_boundary(at) || SEAL.as_bytes()[at] == b'\n' {
            continue;
        }
        // The first line is a comment nobody signs.
        if at < SEAL.find('\n').unwrap() {
            continue;
        }
        for with in ['A', 'B', '0', ' ', '@'] {
            if SEAL.as_bytes()[at] == with as u8 {
                continue;
            }
            let mut seal = SEAL.to_string();
            seal.replace_range(at..=at, &with.to_string());
            assert!(sealed(SIGNED.as_bytes(), &seal).is_err(), "character {at}");
            tried += 1;
        }
    }
    // A seal of this key for another plugin or another version is no seal of this one.
    for (id, version) in [
        ("dev.exemple.slack", "1.2.1"),
        ("dev.exemple.slack", "1.2"),
        ("dev.exemple.slac", "1.2.0"),
        ("dev.escouade.slack", "1.2.0"),
    ] {
        assert_eq!(
            store_keys::verified_with(&[KEY], SIGNED.as_bytes(), SEAL, id, version),
            Err(Refused::OtherVersion)
        );
    }
    eprintln!("AUDIT seal: {tried} changes of one bit or one character, none sealed");
}

/// A key the revocation list names seals nothing any more, and the app's own keys are the two of `store/keys`.
#[test]
#[ignore = "audit 2026-11"]
fn holds_a_revoked_key_seals_nothing() {
    let keys = vec![KEY.to_string()];
    let list: RevocationList = serde_json::from_value(json!({
        "serial": 7, "issued_at": "2026-11-12T00:00:00Z", "entries": [],
        "keys_revoked": [store_keys::key_id(KEY)],
    }))
    .unwrap();
    assert!(list.trusted(&keys).is_empty());
    let none: Vec<&str> = Vec::new();
    assert_eq!(
        store_keys::verified_with(&none, SIGNED.as_bytes(), SEAL, "dev.exemple.slack", "1.2.0"),
        Err(Refused::NoKey)
    );
    assert_eq!(store_keys::STORE_PUBLIC_KEYS.len(), 2);
    // A list that is not signed by a key of the app, or of another serial, is no list.
    let json = serde_json::to_vec(&list).unwrap();
    assert!(store::check_list(&keys, &json, SEAL, 0).is_err());
    assert!(store::check_list(&keys, &json, "nope", 0).is_err());
    assert!(store::check_list(&keys, b"{}", SEAL, 0).is_err());
}

/// What the store names is what installs: `version.json` by its SHA-256, of this plugin and this version.
#[test]
#[ignore = "audit 2026-11"]
fn holds_a_version_json_changed_or_of_another_version_installs_nothing() {
    let json = br#"{"format":1,"id":"dev.exemple.slack","version":"1.2.0","repository":"https://github.com/o/r","tag":"v1.2.0","archive":{"sha256":"","size":0}}"#;
    let sum = store::sha256(json);
    // As it is named: the check passes, and the seed goes on to its archive, which this one has not.
    let said = taken_from_a_seed(json, &sum, "dev.exemple.slack", "1.2.0");
    assert!(said.contains("archive.tar.gz"), "{said}");
    let mut changed = json.to_vec();
    changed[20] ^= 1;
    let said = taken_from_a_seed(&changed, &sum, "dev.exemple.slack", "1.2.0");
    assert!(said.contains("not the one the catalog names"), "{said}");
    // A downgrade: the bytes of an older version, served under the newer one's name.
    let said = taken_from_a_seed(json, &sum, "dev.exemple.slack", "1.3.0");
    assert!(said.contains("another version's"), "{said}");
    let said = taken_from_a_seed(json, &sum, "dev.exemple.other", "1.2.0");
    assert!(said.contains("another version's"), "{said}");
}

/// What a seed whose list names `id` at `version` by the fingerprint `expected` says of the `version.json` `json`: the
/// check of a `version.json` (`store::check_version_json`, the store's and the seed's alike) is the first thing
/// `Seed::take` does, and the door a test of the app has to it, the host of the plugins being a crate of its own.
/// Nothing is installed: the folder holds that file alone.
fn taken_from_a_seed(json: &[u8], expected: &str, id: &str, version: &str) -> String {
    let dir = dir("seed");
    let folder = dir.join(id);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("version.json"), json).unwrap();
    let entry = SeedEntry {
        id: id.into(),
        version: version.into(),
        sha256: expected.into(),
    };
    let seed = Seed {
        dir: Some(dir.clone()),
        entries: vec![entry.clone()],
        unsigned: true,
        counts: false,
    };
    let said = match seed.take(&entry, None, crate::i18n::Lang::En) {
        Ok(_) => "taken".to_string(),
        Err(why) => why,
    };
    let _ = std::fs::remove_dir_all(&dir);
    said
}

// ---- The readers of what third parties write: generated inputs ---------------------------------------------------

/// The link `escouade://`: whatever it holds, it opens a plugin's page by a well-formed id, or nothing.
#[test]
#[ignore = "audit 2026-11"]
fn fuzz_the_deep_link_gives_a_plugin_id_or_nothing() {
    use crate::deeplink::{parse, DeepLink};
    for url in [
        "escouade://plugin/dev.exemple.slack",
        "ESCOUADE://PLUGIN/dev.exemple.slack/",
        "escouade:plugin/dev.exemple.slack",
        "escouade://plugin/dev%2Eexemple%2Eslack",
    ] {
        assert!(matches!(parse(url), DeepLink::Plugin { .. }), "{url}");
    }
    for url in [
        "escouade://plugin/../../etc/passwd",
        "escouade://plugin/dev.exemple.slack/../x",
        "escouade://plugin/dev.exemple.slack?install=1",
        "escouade://plugin/dev.exemple.slack#x",
        "escouade://plugin/dev.exemple.slack\" --data-dir \"C:\\x",
        "escouade://plugin/dev.exemple.slack%00",
        "escouade://plugin/%2e%2e%2f%2e%2e%2f",
        "escouade://install/dev.exemple.slack",
        "escouade://settings",
        "escouade://plugin/",
        "escouade://",
        "file:///C:/Windows/win.ini",
        "https://escouade.dev/",
        "--data-dir=C:\\x",
    ] {
        assert!(matches!(parse(url), DeepLink::Bad { .. }), "{url}");
    }
    let seeds: &[&[u8]] = &[
        b"escouade://plugin/dev.exemple.slack",
        b"escouade:plugin/a.b",
        b"escouade://plugin/dev%2Eexemple%2Eslack/",
    ];
    fuzz("deeplink::parse", |rng| {
        let seed = *rng.pick(seeds);
        let bytes = mutated(rng, seed);
        let url = String::from_utf8_lossy(&bytes);
        match parse(&url) {
            DeepLink::Plugin { id } => {
                assert!(manifest::is_id(&id), "{url:?} gave the id {id:?}");
                assert!(!id.contains(['/', '\\', ':', '%', ' ', '"']), "{id:?}");
            }
            DeepLink::Bad { url: shown } => {
                assert!(shown.chars().count() <= 121 && !shown.chars().any(char::is_control));
            }
        }
    });
}

fn junk(rng: &mut Rng, depth: usize) -> Value {
    const TEXTS: &[&str] = &[
        "",
        "*",
        "..",
        "../../x",
        "C:\\Windows\\System32\\cmd.exe",
        "/bin/sh",
        "dev.escouade.claude",
        "dev.exemple.audit",
        "{escouade}/bin/x",
        "127.0.0.1",
        "localhost",
        "*.com",
        "git *",
        "* *",
        "\0",
        "a\nb",
        "https://hooks.exemple.dev/{{secrets.token}}",
        "1.0.0",
        "99999999999999999999.0.0",
        "agent",
        "write",
        "<img src=x onerror=alert(1)>",
    ];
    match rng.below(if depth > 3 { 5 } else { 8 }) {
        0 => Value::Null,
        1 => Value::Bool(rng.below(2) == 0),
        2 => json!(rng.next() as i64),
        3 => json!(rng.next() as f64 / 7.0),
        4 => json!(*rng.pick(TEXTS)),
        5 => Value::Array((0..rng.below(4)).map(|_| junk(rng, depth + 1)).collect()),
        _ => {
            let mut m = serde_json::Map::new();
            for _ in 0..rng.below(4) {
                m.insert(rng.pick(TEXTS).to_string(), junk(rng, depth + 1));
            }
            Value::Object(m)
        }
    }
}

/// Puts junk at a place of `v` chosen at random.
fn spoil(rng: &mut Rng, v: &mut Value, depth: usize) {
    match v {
        Value::Object(m) if !m.is_empty() && rng.below(4) != 0 => {
            let key = m.keys().nth(rng.below(m.len())).unwrap().clone();
            match rng.below(6) {
                0 => {
                    m.remove(&key);
                }
                1 => {
                    m.insert(key, junk(rng, depth));
                }
                _ => spoil(rng, m.get_mut(&key).unwrap(), depth + 1),
            }
        }
        Value::Array(a) if !a.is_empty() && rng.below(4) != 0 => {
            let at = rng.below(a.len());
            match rng.below(4) {
                0 => a.push(junk(rng, depth)),
                1 => a[at] = junk(rng, depth),
                _ => spoil(rng, &mut a[at], depth + 1),
            }
        }
        other => *other = junk(rng, depth),
    }
}

/// A manifest of a third party, whatever it holds: read without a panic, and when it is taken, it took nothing a third
/// party may not have.
#[test]
#[ignore = "audit 2026-11"]
fn fuzz_a_manifest_taken_holds_nothing_a_third_party_may_not_have() {
    let seed = json!({
        "id": "dev.exemple.audit",
        "name": { "fr": "Audit", "en": "Audit" },
        "version": "1.0.0",
        "description": "x",
        "author": { "name": "A", "url": "https://exemple.dev" },
        "license": "MIT",
        "repository": "https://github.com/o/r",
        "escouade": { "api": "1", "min": "2.2.0" },
        "run": { "js": "dist/plugin.mjs" },
        "activation": ["event:ticket.column"],
        "permissions": {
            "events": ["ticket.column"],
            "network": ["slack.com", "*.exemple.dev"],
            "secrets": ["token"],
            "commands": ["git status", "git log *"],
            "files": [{ "root": "agent", "access": "read" }],
        },
        "limits": { "memoryBytes": 1024, "turnSeconds": 3 },
        "contributes": {
            "secrets": [{ "key": "token", "title": "Token" }],
            "settings": [{ "key": "channel", "type": "string", "title": "Channel" }],
            "harnesses": [{
                "id": "audit", "name": "Audit",
                "locate": { "names": ["audit"], "program": "C:\\x\\audit.exe" },
                "acp": { "args": ["acp"], "env": { "A": "b" } },
            }],
        },
    });
    if let Err(problems) = manifest::check(&seed, Hand::Third).manifest {
        panic!("the seed of the fuzz is refused: {problems:?}");
    }
    let mut taken = 0u64;
    fuzz("manifest::check", |rng| {
        let mut v = seed.clone();
        for _ in 0..=rng.below(4) {
            spoil(rng, &mut v, 0);
        }
        let Ok(m) = manifest::check(&v, Hand::Third).manifest else {
            return;
        };
        taken += 1;
        assert!(manifest::is_id(&m.id), "{}", m.id);
        assert!(
            !m.id.starts_with("dev.escouade."),
            "a third party took the id {}",
            m.id
        );
        for c in &m.permissions.commands {
            assert!(manifest::command_holds(c), "command {c:?}");
        }
        for f in &m.permissions.files {
            assert!(
                ["plugin", "temp", "agent", "agentRepo", "agentHome"].contains(&f.root.as_str()),
                "root {:?}",
                f.root
            );
        }
        // The same text read as a file gives the same manifest.
        let text = serde_json::to_string(&v).unwrap();
        assert!(manifest::read(&text, Hand::Third).manifest.is_ok());
    });
    eprintln!(
        "AUDIT-FUZZ manifest::check: {taken} manifests taken, each within a third party's rights"
    );
}

/// An archive of the store, whatever it holds: read in memory without a panic, and what it gives are plain relative
/// paths.
#[test]
#[ignore = "audit 2026-11"]
fn fuzz_an_archive_gives_plain_relative_paths_or_nothing() {
    fn tarball(entries: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
        let mut ar = tar::Builder::new(Vec::new());
        for (path, kind, bytes) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(*kind);
            h.set_size(bytes.len() as u64);
            h.set_mode(0o644);
            // Written as bytes: `set_path` itself refuses `..`.
            let name = path.as_bytes();
            h.as_old_mut().name[..name.len()].copy_from_slice(name);
            if kind.is_symlink() || kind.is_hard_link() {
                h.set_link_name("../../outside").unwrap();
            }
            h.set_cksum();
            ar.append(&h, *bytes).unwrap();
        }
        let tar = ar.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, &tar).unwrap();
        gz.finish().unwrap()
    }
    use tar::EntryType as T;
    let good = tarball(&[
        ("top/escouade-plugin.json", T::Regular, b"{}"),
        ("top/dist/plugin.mjs", T::Regular, b"export {}"),
    ]);
    assert_eq!(store::read_archive(&good).unwrap().len(), 2);
    for bad in [
        tarball(&[("top/../../outside.txt", T::Regular, b"x")]),
        tarball(&[("/etc/passwd", T::Regular, b"x")]),
        tarball(&[("C:/Windows/x", T::Regular, b"x")]),
        tarball(&[("top\\..\\..\\outside.txt", T::Regular, b"x")]),
        tarball(&[("top/link", T::Symlink, b"")]),
        tarball(&[("top/hard", T::Link, b"")]),
        tarball(&[("top/fifo", T::Fifo, b"")]),
        tarball(&[("top/a", T::Regular, b"1"), ("top/a", T::Regular, b"2")]),
    ] {
        assert!(store::read_archive(&bad).is_err());
    }
    let plain_tar = {
        let mut d = flate2::read::GzDecoder::new(good.as_slice());
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut d, &mut out).unwrap();
        out
    };
    fuzz("store::read_archive", |rng| {
        // Changed inside the tar (then packed again), or in the packed bytes themselves.
        let gz = if rng.below(2) == 0 {
            let tar = mutated(rng, &plain_tar);
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            std::io::Write::write_all(&mut gz, &tar).unwrap();
            gz.finish().unwrap()
        } else {
            mutated(rng, &good)
        };
        let Ok(files) = store::read_archive(&gz) else {
            return;
        };
        for (path, _) in files {
            let p = Path::new(&path);
            assert!(
                !p.is_absolute()
                    && !path.starts_with(['/', '\\'])
                    && !path.split(['/', '\\']).any(|part| part == "..")
                    && !path.contains('\0'),
                "the archive gave the path {path:?}"
            );
        }
    });
}

/// A seal, a revocation list or a `version.json` made of anything: refused, never a panic.
#[test]
#[ignore = "audit 2026-11"]
fn fuzz_the_seals_and_the_lists_of_the_store_never_hold_by_chance() {
    let keys = vec![KEY.to_string()];
    let list = br#"{"serial":3,"issued_at":"2026-11-12T00:00:00Z","entries":[{"id":"a.b","versions":"*","action":"disable","reason":{"fr":"x","en":"x"},"date":"2026-11-12"}]}"#;
    let mut n = 0u64;
    fuzz("store seals and lists", |rng| {
        let seal = String::from_utf8_lossy(&mutated(rng, SEAL.as_bytes())).into_owned();
        let json = mutated(rng, SIGNED.as_bytes());
        if seal != SEAL || json != SIGNED.as_bytes() {
            // Whatever the signature's file became, it seals nothing but the bytes that were signed.
            assert!(
                sealed(&json, &seal).is_err() || json == SIGNED.as_bytes(),
                "{seal:?} seals {:?}",
                String::from_utf8_lossy(&json)
            );
        }
        let changed = mutated(rng, list);
        assert!(store::check_list(&keys, &changed, &seal, 0).is_err());
        // A `version.json` made of anything, by the door of the seed (`taken_from_a_seed`, which writes a file): one
        // input in 512, under the fingerprint of other bytes then under its own.
        n += 1;
        if n.is_multiple_of(512) {
            let said = taken_from_a_seed(&changed, "", "a.b", "1.0.0");
            assert!(said.contains("not the one the catalog names"), "{said}");
            let said = taken_from_a_seed(&changed, &store::sha256(&changed), "a.b", "1.0.0");
            assert_ne!(said, "taken");
        }
    });
}

/// The frames of a harness (`Content-Length`), with lengths of this size at most: read or refused without a panic.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn fuzz_the_frames_of_a_harness_are_read_or_refused() {
    let seeds: &[&[u8]] = &[
        b"Content-Length: 2\r\n\r\n{}",
        b"content-length:4\n\ntrue",
        b"X: y\r\nContent-Length: 0\r\n\r\n",
    ];
    let (start, mut rng, mut n) = (Instant::now(), Rng(7), 0u64);
    while start.elapsed() < fuzz_time() {
        let seed = *rng.pick(seeds);
        let mut bytes = mutated(&mut rng, seed);
        // A length a harness may write but this test will not allocate (SEC-08 is the length itself).
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let huge = text.lines().any(|l| {
            l.split_once(':').is_some_and(|(k, v)| {
                k.trim().eq_ignore_ascii_case("content-length")
                    && v.trim().parse::<u64>().is_ok_and(|n| n > 1 << 20)
            })
        });
        if huge {
            continue;
        }
        bytes.extend_from_slice(b"\r\n");
        let mut reader = tokio::io::BufReader::new(bytes.as_slice());
        while let Ok(Some(frame)) = read_frame(&mut reader).await {
            assert!(frame.len() <= 1 << 20);
        }
        n += 1;
    }
    eprintln!(
        "AUDIT-FUZZ harness::read_frame: {n} inputs in {:.1} s, no panic",
        start.elapsed().as_secs_f64()
    );
}

/// SEC-08. A frame's length is taken on the word of the harness: the buffer is made before a byte of it is read. Shown
/// with 256 MiB, which a machine has; a length past its memory ends the app (an allocation that fails aborts).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn finding_a_frame_is_allocated_on_the_word_of_its_length() {
    const ASKED: usize = 256 << 20;
    let header = format!("Content-Length: {ASKED}\r\n\r\n");
    // Twelve bytes follow, not 256 MiB.
    let bytes = [header.as_bytes(), b"only twelve!"].concat();
    let mut reader = tokio::io::BufReader::new(bytes.as_slice());
    let before = Instant::now();
    let got = read_frame(&mut reader).await;
    // It fails at the end of the input, once the 256 MiB were allocated and zeroed.
    assert!(got.is_err(), "{:?}", got.map(|f| f.map(|f| f.len())));
    eprintln!(
        "AUDIT read_frame: a header of {} bytes made the reader allocate {ASKED} bytes ({:.0} ms)",
        header.len(),
        before.elapsed().as_secs_f64() * 1000.0
    );
}

/// SEC-08, to its end: a length past the machine's memory. The allocation fails, which aborts the process: neither an
/// error nor a panic the app could catch. Played in a process of its own (this test binary, asked for the test below),
/// whose end is read.
#[test]
#[ignore = "audit 2026-11"]
fn finding_a_frame_longer_than_the_memory_ends_the_process() {
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "plugins::audit_2_2_0::inner_a_frame_longer_than_the_memory",
            "--nocapture",
        ])
        .env("AUDIT_INNER_HUGE_FRAME", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&child.stderr);
    eprintln!(
        "AUDIT read_frame: the process that read the frame ended with {:?}; it said: {}",
        child.status.code(),
        stderr
            .lines()
            .find(|l| l.contains("memory allocation"))
            .unwrap_or("(nothing about its memory)")
    );
    assert!(!child.status.success(), "the process went on");
    assert!(stderr.contains("memory allocation of"), "{stderr}");
}

/// What `finding_a_frame_longer_than_the_memory_ends_the_process` plays in its own process; nothing otherwise.
#[tokio::test]
#[ignore = "audit 2026-11"]
async fn inner_a_frame_longer_than_the_memory() {
    if std::env::var_os("AUDIT_INNER_HUGE_FRAME").is_none() {
        return;
    }
    let bytes = format!("Content-Length: {}\r\n\r\n{{}}", 1u64 << 62).into_bytes();
    let mut reader = tokio::io::BufReader::new(bytes.as_slice());
    let _ = read_frame(&mut reader).await;
}

/// `.escouade.yml` and a shared ticket are files of a repository anyone may have written: read without a panic.
#[test]
#[ignore = "audit 2026-11"]
fn fuzz_the_files_of_a_repository_are_read_without_a_panic() {
    let yml: &[u8] = b"version: 1\nname: Demo\ncommands:\n  test: npm test\n  launch:\n    - name: web\n      command: npm run dev\n      url: http://localhost:5173\nkanban:\n  maxParallel: 2\n  mode: auto\nplugins:\n  recommended:\n    - dev.exemple.slack\n";
    let ticket: &[u8] = b"id: 0a1b2c3d\ntitle: Un ticket\nstatus: todo\ndescription: |\n  Des lignes.\ncriteria:\n  - premier\nauthor:\n  name: A\n";
    fuzz("projectfile::read and sharing::read", |rng| {
        let text = String::from_utf8_lossy(&mutated(rng, yml)).into_owned();
        let _ = crate::projectfile::read(&text);
        let _ = crate::projectfile::tree(&text);
        let text = String::from_utf8_lossy(&mutated(rng, ticket)).into_owned();
        let _ = crate::sharing::read(&text);
        let _ = crate::sharing::has_conflict(&text);
    });
}

// ---- The guards, and the ways around them ------------------------------------------------------------------------
//
// `holds_*`: what a guard refuses, under every spelling tried.
// `limit_*`: what a guard holds, and what it lets through; what passes is printed, and said in the report.

/// The variables by which a command a plugin may run becomes another (git's configuration, a loader's, an
/// interpreter's options): none passes undeclared, and declared, all are refused but the two that belong to no refused
/// family (a harness's own folder and its endpoint).
#[test]
#[ignore = "audit 2026-11"]
fn holds_the_variables_that_turn_a_command_into_another_are_refused_even_declared() {
    let names = [
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
        "GIT_CONFIG_GLOBAL",
        "GIT_SSH_COMMAND",
        "GIT_EXEC_PATH",
        "GIT_EXTERNAL_DIFF",
        "GIT_ASKPASS",
        "PYTHONPATH",
        "PYTHONSTARTUP",
        "RUBYOPT",
        "PERL5OPT",
        "JAVA_TOOL_OPTIONS",
        "NODE_PATH",
        "npm_config_script_shell",
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "XDG_CONFIG_HOME",
        "CLAUDE_CONFIG_DIR",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "SSL_CERT_FILE",
        "NODE_EXTRA_CA_CERTS",
        "NODE_TLS_REJECT_UNAUTHORIZED",
        "GIT_SSL_NO_VERIFY",
        "ANTHROPIC_BASE_URL",
        "EDITOR",
        "PAGER",
        "BROWSER",
    ];
    let declared: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let mut pass = Vec::new();
    for name in names {
        assert!(
            var_refused(Some(&[]), name, false).is_some(),
            "{name} passes undeclared"
        );
        assert!(
            var_refused(None, name, false).is_some(),
            "{name} passes without permissions.env"
        );
        if var_refused(Some(&declared), name, false).is_none() {
            pass.push(name);
        }
        // The launch of a harness is judged the same.
        let vars = Vars {
            first_hand: false,
            declared: Some(declared.clone()),
            given: Vec::new(),
        };
        assert_eq!(
            vars.refusal("poc", name).is_none(),
            var_refused(Some(&declared), name, false).is_none(),
            "{name}: a launch and a program are not judged alike"
        );
    }
    eprintln!(
        "AUDIT of these variables, those that pass once declared ({}): {}",
        pass.len(),
        pass.join(" ")
    );
    assert_eq!(pass, ["CLAUDE_CONFIG_DIR", "ANTHROPIC_BASE_URL"]);
}

/// The same, shown: `git status` with the configuration by variables is refused, declared or not, and the command of
/// the variable never runs.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn holds_git_status_with_a_configuration_by_variables_runs_nothing() {
    let env = json!({
        "GIT_CONFIG_COUNT": "1",
        "GIT_CONFIG_KEY_0": "core.fsmonitor",
        "GIT_CONFIG_VALUE_0": "echo AUDIT-POC > audit-marker.txt; false",
    });
    let lower = json!({
        "git_config_count": "1",
        "git_config_key_0": "core.fsmonitor",
        "git_config_value_0": "echo AUDIT-POC > audit-marker.txt; false",
    });
    let all = [
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
        "git_config_count",
        "git_config_key_0",
        "git_config_value_0",
    ];
    for declared in [None, Some(&[][..]), Some(&all[..])] {
        let b = bed_with(&["git status"], declared);
        assert!(std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&b.repo)
            .status()
            .unwrap()
            .success());
        for env in [&env, &lower] {
            let spawned = b.procs.serve(
                "proc/spawn",
                &json!({ "command": "git", "args": ["status"], "dir": "agent", "agent": "a", "env": env }),
            );
            assert_eq!(
                spawned.as_ref().err().map(|e| e.0),
                Some(PROC_REFUSED),
                "declared {declared:?}: {spawned:?}"
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(
            !b.repo.join("audit-marker.txt").exists(),
            "the command of the variable ran"
        );
        // The same program without them still runs: the guard is on the variables alone.
        let plain = b.procs.serve(
            "proc/spawn",
            &json!({ "command": "git", "args": ["status"], "dir": "agent", "agent": "a" }),
        );
        assert!(plain.is_ok(), "{plain:?}");
    }
}

/// A refused variable under another spelling is refused too: its case (Windows reads `path` as `PATH`), a space, a sign.
/// And a variable declared in a case is not set in another.
#[test]
#[ignore = "audit 2026-11"]
fn holds_a_refused_variable_under_another_spelling_is_refused() {
    for name in [
        "git_config_count",
        "Git_Ssh_Command",
        "path",
        "Path",
        "pAtH",
        "home",
        "Home",
        "node_options",
        "Node_Options",
        "ld_preload",
        "dyld_insert_libraries",
        "escouade_data_dir",
        "https_proxy",
        "Http_Proxy",
        "no_proxy",
        "PATH ",
        " PATH",
        "PATH\t",
        "PA\0TH",
        "ＰＡＴＨ",
        "PATH=x",
        "GIT_CONFIG_COUNT\n",
        "%PATH%",
        "$PATH",
        "1PATH",
        "A-B",
        "",
    ] {
        let declared = vec![name.to_string()];
        assert!(
            var_refused(Some(&declared), name, false).is_some(),
            "{name:?} passes"
        );
        // Its manifest does not hold either: nothing of it is installed.
        assert!(manifest::env_refused(name, Hand::Third), "{name:?}");
    }
    let declared = vec!["AUDIT_NOTE".to_string()];
    assert!(var_refused(Some(&declared), "AUDIT_NOTE", false).is_none());
    assert!(var_refused(Some(&declared), "audit_note", false).is_some());
    assert!(var_refused(Some(&declared), "AUDIT_NOTE2", false).is_some());
    // Who writes a commit, of `GIT_*`, and nothing else of it.
    for name in ["GIT_AUTHOR_NAME", "GIT_COMMITTER_EMAIL", "GIT_AUTHOR_DATE"] {
        assert!(!manifest::env_refused(name, Hand::Third), "{name}");
    }
    for name in [
        "GIT_AUTHOR",
        "GIT_COMMITTERS_HOOK",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
    ] {
        assert!(manifest::env_refused(name, Hand::Third), "{name}");
    }
}

/// What the families do not name: variables by which other programs load code, another configuration or another
/// server. Declared, they pass; the declaration, shown before the installation, is then the guard, with the review.
#[test]
#[ignore = "audit 2026-11"]
fn limit_variables_of_no_refused_family_pass_once_declared() {
    let names = [
        // A program run in place of another, or code loaded at a start.
        "SSH_ASKPASS",
        "LESSOPEN",
        "MANPAGER",
        "RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "DOTNET_STARTUP_HOOKS",
        "COR_PROFILER_PATH",
        "CORECLR_PROFILER_PATH",
        "GOFLAGS",
        "MAVEN_OPTS",
        "GRADLE_OPTS",
        "CLASSPATH",
        "BUNDLE_GEMFILE",
        "GEM_HOME",
        "OPENSSL_CONF",
        "ELECTRON_RUN_AS_NODE",
        "PSModulePath",
        // Where a program finds its files, its packages, its server.
        "TMP",
        "TEMP",
        "TMPDIR",
        "SystemRoot",
        "PIP_INDEX_URL",
        "KUBECONFIG",
        "DOCKER_HOST",
        // A harness's own home and endpoint.
        "CLAUDE_CONFIG_DIR",
        "CODEX_HOME",
        "COPILOT_HOME",
        "ANTHROPIC_BASE_URL",
        "OPENAI_BASE_URL",
    ];
    let declared: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let pass: Vec<&str> = names
        .into_iter()
        .filter(|n| var_refused(Some(&declared), n, false).is_none())
        .collect();
    eprintln!(
        "AUDIT variables of no refused family that pass once declared ({} of {}): {}",
        pass.len(),
        declared.len(),
        pass.join(" ")
    );
    // Undeclared, none of them.
    for name in &declared {
        assert!(var_refused(Some(&[]), name, false).is_some(), "{name}");
    }
    assert!(pass.contains(&"SSH_ASKPASS") && pass.contains(&"DOTNET_STARTUP_HOOKS"));
}

/// The harness anyone's plugin describes is no shell nor interpreter; what the list does not name passes (SEC-01): a
/// launcher of an interpreter, a runner of packages, a build tool.
#[test]
#[ignore = "audit 2026-11"]
fn limit_a_described_harness_is_no_shell_but_other_runners_pass() {
    for held in [
        "sh",
        "bash",
        "BASH.EXE",
        "zsh",
        "cmd",
        "cmd.exe",
        "Cmd.Com",
        "powershell",
        "PowerShell.exe",
        "pwsh",
        "pwsh-7",
        "python",
        "python3",
        "python3.12",
        "Python.exe",
        "pypy3",
        "perl5.36",
        "ruby3.3",
        "node",
        "node.exe",
        "nodejs",
        "deno",
        "bun",
        "wsl",
        "env",
        "mshta",
        "rundll32",
        "wscript",
        "cscript",
        "osascript",
        "busybox",
    ] {
        assert!(manifest::interpreter(held), "{held} passes");
    }
    let others = [
        "py",
        "pythonw",
        "python3w",
        "pwsh-preview",
        "powershell_ise",
        "node22",
        "php8.3",
        "lua5.4",
        "npx",
        "npm",
        "pnpm",
        "yarn",
        "bunx",
        "uv",
        "uvx",
        "pipx",
        "tsx",
        "ts-node",
        "java",
        "jshell",
        "dotnet",
        "go",
        "cargo",
        "make",
        "msbuild",
        "git",
        "ssh",
        "awk",
        "find",
        "xargs",
        "tclsh",
        "Rscript",
        "julia",
        "nu",
        "xonsh",
        "rbash",
        "mksh",
        "sh.bat",
        "bash.cmd",
        "cmd.bat",
        "conhost",
        "forfiles",
        "regsvr32",
        "msiexec",
        "explorer",
        "schtasks",
        "certutil",
        "curl",
        "docker",
    ];
    let pass: Vec<&str> = others
        .into_iter()
        .filter(|n| !manifest::interpreter(n))
        .collect();
    eprintln!(
        "AUDIT programs a described harness may name, that run what their arguments say ({} of {}): {}",
        pass.len(),
        others.len(),
        pass.join(" ")
    );
    for known in ["py", "pythonw", "pwsh-preview", "npx", "uvx", "dotnet"] {
        assert!(pass.contains(&known), "{known} is refused now");
    }
    // The manifest is judged the same: a third party's harness named `npx` holds, one named `bash` does not.
    let third = |name: &str| {
        manifest::check(
            &json!({
                "id": "dev.exemple.audit",
                "name": "Audit",
                "version": "1.0.0",
                "escouade": { "api": "1", "min": "2.2.0" },
                "contributes": { "harnesses": [{
                    "id": "audit",
                    "name": "Audit",
                    "locate": { "names": [name] },
                    "acp": { "args": ["-y", "audit-poc-package"] },
                }] },
            }),
            Hand::Third,
        )
    };
    let problems = |name: &str| match third(name).manifest {
        Ok(_) => "holds".to_string(),
        Err(problems) => format!("{problems:?}"),
    };
    assert!(
        problems("bash").contains("Interpreter"),
        "{}",
        problems("bash")
    );
    eprintln!(
        "AUDIT the manifest of a third party's harness named npx: {}",
        problems("npx")
    );
    assert!(!problems("npx").contains("Interpreter"));
}

/// « Autoriser le mode Bypass » off: the arguments known to open the mode of no question leave a
/// launch, in each of their forms. Other spellings stay: the short ones, and the options that give as much by another
/// name. None is run here; what each means is its harness's own help.
#[test]
#[ignore = "audit 2026-11"]
fn limit_the_bypass_arguments_known_are_held_and_other_spellings_pass() {
    let held = |args: &[&str]| {
        let mut a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        crate::bypass::hold(&mut a, false);
        a
    };
    let known: [&[&str]; 18] = [
        &["--dangerously-skip-permissions"],
        &["--allow-dangerously-skip-permissions"],
        &["--permission-mode", "bypassPermissions"],
        &["--permission-mode=bypassPermissions"],
        &["--dangerously-bypass-approvals-and-sandbox"],
        &["--yolo"],
        &["--yolo=true"],
        &["--approval-mode", "yolo"],
        &["--approval-mode=yolo"],
        &["--allow-all"],
        &["--allow-all-tools"],
        &["--allow-all-paths"],
        &["--allow-all-urls"],
        &["--force"],
        &["--agent", "auto-approve"],
        &["--agent=auto-approve"],
        &["--mode", "yolo"],
        &["--mode=bypassPermissions"],
    ];
    for args in known {
        assert!(held(args).is_empty(), "{args:?} stays");
        // Between others, the others stay.
        let mut among = vec!["--model", "x"];
        among.extend(args);
        among.push("--resume");
        assert_eq!(held(&among), ["--model", "x", "--resume"], "{args:?}");
        // And with the setting on, all of it.
        let mut a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        crate::bypass::hold(&mut a, true);
        assert_eq!(a, args);
    }
    // A mode that asks is never taken for one that does not.
    for args in [
        &["--permission-mode", "acceptEdits"][..],
        &["--mode", "plan"],
        &["--approval-mode", "auto_edit"],
        &["--agent", "default"],
    ] {
        assert_eq!(held(args), args);
    }
    let other: [(&str, &[&str]); 12] = [
        ("-y (the short of --yolo)", &["-y"]),
        ("-f (the short of --force)", &["-f"]),
        (
            "--sandbox danger-full-access --ask-for-approval never",
            &[
                "--sandbox",
                "danger-full-access",
                "--ask-for-approval",
                "never",
            ],
        ),
        (
            "-s danger-full-access -a never",
            &["-s", "danger-full-access", "-a", "never"],
        ),
        (
            "-c sandbox_mode=… -c approval_policy=… (a configuration by arguments)",
            &[
                "-c",
                "sandbox_mode=\"danger-full-access\"",
                "-c",
                "approval_policy=\"never\"",
            ],
        ),
        (
            "--settings {\"permissions\":{\"defaultMode\":\"bypassPermissions\"}}",
            &[
                "--settings",
                "{\"permissions\":{\"defaultMode\":\"bypassPermissions\"}}",
            ],
        ),
        (
            "--allow-tool=shell --allow-tool=write (one by one)",
            &["--allow-tool=shell", "--allow-tool=write"],
        ),
        ("--YOLO (another case)", &["--YOLO"]),
        (
            "--approval-mode YOLO (the value in another case)",
            &["--approval-mode", "YOLO"],
        ),
        (
            "--approval-mode  yolo (the value after an empty argument)",
            &["--approval-mode", "", "yolo"],
        ),
        (
            "--dangerously-skip-perm (an abbreviation)",
            &["--dangerously-skip-perm"],
        ),
        (
            "--permission-mode:bypassPermissions (another separator)",
            &["--permission-mode:bypassPermissions"],
        ),
    ];
    let pass: Vec<&str> = other
        .iter()
        .filter(|(_, args)| held(args) == *args)
        .map(|(what, _)| *what)
        .collect();
    eprintln!(
        "AUDIT spellings the guard of the Bypass arguments lets through ({} of {}):\n  {}",
        pass.len(),
        other.len(),
        pass.join("\n  ")
    );
    assert!(pass.iter().any(|w| w.starts_with("-y")));
    // The mode itself is refused before any argument: no agent starts nor goes into it without the setting.
    assert!(crate::bypass::refusal("bypassPermissions", false).is_some());
    assert!(crate::bypass::refusal("bypassPermissions", true).is_none());
    for mode in [
        "BypassPermissions",
        "bypasspermissions",
        " bypassPermissions",
        "bypassPermissions ",
    ] {
        eprintln!(
            "AUDIT mode {mode:?} without the setting: {}",
            if crate::bypass::refusal(mode, false).is_some() {
                "refused"
            } else {
                "not taken for Bypass"
            }
        );
    }
}

/// « Ignorer la vérification des certificats TLS » holds through the proxy alone: without a proxy, and for a
/// host of the exclusions, a certificate is checked.
#[test]
#[ignore = "audit 2026-11"]
fn holds_the_unchecked_tls_is_for_the_proxy_alone() {
    use crate::tls::{proxy_for, Net};
    let net = |proxy: &str, no_proxy: &str| Net {
        proxy: proxy.into(),
        no_proxy: no_proxy.into(),
        insecure: true,
        extra_ca: String::new(),
    };
    assert!(!net("", "").unchecked_by_proxy());
    assert!(
        !net("   ", "").unchecked_by_proxy() || proxy_for(&net("   ", ""), "example.com").is_none()
    );
    assert!(net("http://127.0.0.1:9000", "").unchecked_by_proxy());
    let n = net(
        "http://127.0.0.1:9000",
        "internal.example, .corp.example;10.1.2.3",
    );
    for around in [
        "internal.example",
        "INTERNAL.example",
        "a.internal.example",
        "x.corp.example",
        "10.1.2.3",
    ] {
        assert!(
            proxy_for(&n, around).is_none(),
            "{around} is taken through the proxy"
        );
    }
    for through in [
        "example.com",
        "notinternal.example",
        "internal.example.evil.test",
    ] {
        assert!(
            proxy_for(&n, through).is_some(),
            "{through} is taken around the proxy"
        );
    }
    // Exclusions written as other tools read them: the one reading
    // takes each of them, so none is « through the proxy » for the check and « around it » for the route.
    let odd = [
        (
            "a range of addresses (10.0.0.0/8), for 10.1.2.3",
            "10.0.0.0/8",
            "10.1.2.3",
        ),
        (
            "a range of addresses (127.0.0.0/8), for 127.0.0.1",
            "127.0.0.0/8",
            "127.0.0.1",
        ),
        (
            "a range of IPv6 addresses (fd00::/8), for fd12::1",
            "fd00::/8",
            "fd12::1",
        ),
        (
            "a wildcard (*.corp.example), for x.corp.example",
            "*.corp.example",
            "x.corp.example",
        ),
        (
            "a host with its port (internal.example:8443), for internal.example",
            "internal.example:8443",
            "internal.example",
        ),
        (
            "an address with its port (10.1.2.3:8080), for 10.1.2.3",
            "10.1.2.3:8080",
            "10.1.2.3",
        ),
        (
            "a name ending with a dot, for internal.example.",
            "internal.example",
            "internal.example.",
        ),
        ("an IPv6 in brackets ([::1]), for ::1", "[::1]", "::1"),
        (
            "an IPv6 in brackets with its port ([::1]:8443), for [::1]",
            "[::1]:8443",
            "[::1]",
        ),
        ("every host (*)", "*", "anything.example"),
        (
            "exclusions separated by a tab",
            "a.example\tinternal.example",
            "internal.example",
        ),
    ];
    let unchecked: Vec<&str> = odd
        .iter()
        .filter(|(_, no_proxy, host)| {
            proxy_for(&net("http://127.0.0.1:9000", no_proxy), host).is_some()
        })
        .map(|(what, _, _)| *what)
        .collect();
    eprintln!(
        "AUDIT exclusions this reading does not take ({} of {}):\n  {}",
        unchecked.len(),
        odd.len(),
        unchecked.join("\n  ")
    );
    assert!(
        unchecked.is_empty(),
        "these exclusions are not read: {unchecked:?}"
    );
    // And what excludes another host excludes not this one: a range that does not hold it, a range that is none, a
    // name that only ends or starts like it.
    for (no_proxy, host) in [
        ("10.0.0.0/8", "11.0.0.1"),
        ("127.0.0.0/8", "128.0.0.1"),
        ("10.0.0.0/33", "10.1.2.3"),
        ("10.0.0.0/x", "10.1.2.3"),
        ("/8", "10.1.2.3"),
        ("10.1.2.3", "10.1.2.30"),
        ("10.0.0.0/8", "ten.example"),
        ("fd00::/8", "fe80::1"),
        ("corp.example", "evilcorp.example"),
        ("*.corp.example", "corp.example.evil.test"),
        ("internal.example:8443", "internal.example.evil.test"),
    ] {
        assert!(
            proxy_for(&net("http://127.0.0.1:9000", no_proxy), host).is_some(),
            "{host} is taken around the proxy by {no_proxy:?}"
        );
    }
}

/// The same on the wire, the setting on and a proxy set: a server of the
/// machine excluded from the proxy, however the exclusion is written, is reached directly and its certificate is
/// checked; one that is not excluded goes through the proxy, its certificate unchecked, as the setting says.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn holds_a_host_excluded_by_any_spelling_goes_direct_and_its_certificate_is_checked() {
    use crate::integrations::fake::FakeServer;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    // A proxy that opens the tunnels it is asked for, and counts them.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let opened = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let count = opened.clone();
    tokio::spawn(async move {
        while let Ok((mut client, _)) = listener.accept().await {
            let count = count.clone();
            tokio::spawn(async move {
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if client.read(&mut byte).await.unwrap_or(0) == 0 {
                        return;
                    }
                    head.push(byte[0]);
                }
                let head = String::from_utf8_lossy(&head).to_string();
                let Some(target) = head
                    .strip_prefix("CONNECT ")
                    .and_then(|r| r.split(' ').next())
                else {
                    return;
                };
                let Ok(mut server) = tokio::net::TcpStream::connect(target).await else {
                    return;
                };
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = client
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await;
                let _ = tokio::io::copy_bidirectional(&mut client, &mut server).await;
            });
        }
    });
    // A server whose certificate no authority signed, asked by its address and by its name.
    let server = FakeServer::start_tls().await;
    server.on("GET", "/ok", 200, json!({}));
    let by_address = format!("{}/ok", server.url);
    assert!(by_address.contains("127.0.0.1"), "{by_address}");
    let by_name = by_address.replace("127.0.0.1", "localhost");
    let port = server.url.rsplit(':').next().unwrap().to_string();
    let status = |url: &str, no_proxy: &str| {
        let url = url.to_string();
        let settings = crate::model::Settings {
            proxy_url: proxy.clone(),
            no_proxy: no_proxy.into(),
            insecure_tls: true,
            ..crate::model::Settings::default()
        };
        async move {
            let client = crate::tls::app_builder(&settings).unwrap().build().unwrap();
            client
                .get(&url)
                .send()
                .await
                .ok()
                .map(|r| r.status().as_u16())
        }
    };
    let through = || opened.load(std::sync::atomic::Ordering::SeqCst);
    // Excluded: around the proxy, and checked (refused: nobody signed that certificate).
    let excluded: Vec<(&str, String)> = vec![
        (&by_address, "127.0.0.1".into()),
        (&by_address, "127.0.0.0/8".into()),
        (&by_address, "127.0.0.1/32".into()),
        (&by_address, "0.0.0.0/0".into()),
        (&by_address, format!("127.0.0.1:{port}")),
        (&by_address, "*".into()),
        (&by_name, "localhost".into()),
        (&by_name, ".localhost".into()),
        (&by_name, "*.localhost".into()),
        (&by_name, "LOCALHOST".into()),
        (&by_name, "localhost:8443".into()),
        (&by_name, "localhost.".into()),
    ];
    for (url, no_proxy) in &excluded {
        let before = through();
        let got = status(url, no_proxy).await;
        let tunnels = through() - before;
        assert_eq!(
            (got, tunnels),
            (None, 0),
            "excluded by {no_proxy:?}: answered {got:?} with {tunnels} tunnel(s) through the proxy"
        );
    }
    // Not excluded: through the proxy, unchecked.
    let not_excluded = [
        (&by_address, ""),
        (&by_address, "127.0.0.2"),
        (&by_address, "128.0.0.0/8"),
        (&by_name, "notlocalhost"),
        (&by_name, "ocalhost"),
    ];
    for (url, no_proxy) in not_excluded {
        let before = through();
        let got = status(url, no_proxy).await;
        let tunnels = through() - before;
        assert!(
            got == Some(200) && tunnels >= 1,
            "not excluded by {no_proxy:?}: answered {got:?} with {tunnels} tunnel(s) through the proxy"
        );
    }
    eprintln!(
        "AUDIT a server excluded by {} spellings of an exclusion, the setting on: each time reached directly and refused for its certificate; by {} that exclude another host: through the proxy",
        excluded.len(),
        not_excluded.len()
    );
}

/// Escouade's own hand is an origin and an id together. A folder
/// in development, a folder copied or a repository cloned is of anyone's hand whatever its id, and its manifest, read as
/// anyone's, is refused the id `dev.escouade.*` and the variables of the refused families.
#[test]
#[ignore = "audit 2026-11"]
fn holds_a_folder_in_development_is_of_anyones_hand_whatever_its_id() {
    use super::{hand_of, Origin};
    let (ours, theirs) = ("dev.escouade.audit-poc", "dev.audit.malveillant");
    let origins = [
        (
            "a folder in development",
            Origin::Dev {
                path: "C:/work/plugin".into(),
            },
        ),
        (
            "a folder copied",
            Origin::Folder {
                path: "C:/work/plugin".into(),
            },
        ),
        (
            "a repository cloned",
            Origin::Git {
                url: "https://example.com/plugin.git".into(),
                tag: "v1.0.0".into(),
            },
        ),
    ];
    for (what, origin) in &origins {
        for id in [ours, theirs] {
            assert!(
                hand_of(origin, id) == Hand::Third,
                "{what} named {id} is of Escouade's own hand"
            );
        }
    }
    let read = |id: &str, env: &[&str]| {
        manifest::check(
            &json!({
                "id": id,
                "name": "Audit",
                "version": "1.0.0",
                "escouade": { "api": "1", "min": "2.2.0" },
                "permissions": { "commands": ["git status"], "env": env },
            }),
            Hand::Third,
        )
        .manifest
    };
    assert!(read(theirs, &["AUDIT_NOTE"]).is_ok());
    let reserved = format!("{:?}", read(ours, &["AUDIT_NOTE"]).err());
    assert!(reserved.contains("Reserved"), "{reserved}");
    let variable = format!("{:?}", read(theirs, &["GIT_CONFIG_COUNT"]).err());
    assert!(variable.contains("Variable"), "{variable}");
}

/// The refusals of Copilot CLI's view are arguments its plugin writes for Copilot CLI to hold
/// (`plugins/copilot/src/terminal.rs`): asked here as the core asks them (`terminal/launch`), and
/// checked by it with the rest of the launch. What they are for a folder, as it is written: whether Copilot CLI holds
/// them for another spelling of the same folder (a short name, another case, a path through `..`) is its own, and no
/// harness is run in this audit.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn holds_the_refusals_of_the_copilot_view_are_written_for_the_folder_as_spelled() {
    let copilot = crate::plugins::builtin::COPILOT;
    let rt = crate::plugins::builtin::test_runtime("audit-copilot-view");
    let harness = super::harness::BUILTIN_HARNESSES
        .iter()
        .find(|h| h.plugin == copilot)
        .unwrap();
    super::harness::sync_builtin(&rt, harness, true);
    let engine = super::harness::plugged(copilot)
        .expect("the Copilot plugin of the workspace (scripts/plugins-wasm.sh)");
    let home = dir("copilot-view");
    let cwd = home.join("demo (copie)");
    std::fs::create_dir_all(&cwd).unwrap();
    let params = |mode: &str, read_only: bool, fence: &[String]| proto::TerminalParams {
        agent: "a1".into(),
        cwd: plain(&cwd),
        mode: mode.into(),
        read_only,
        fence: fence.to_vec(),
        hook_url: "http://127.0.0.1:1/hook".into(),
        hook_url_var: "ESCOUADE_HOOK_URL".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        private_file: plain(&home.join("tui").join("a1.json")),
        lang: "en".into(),
        ..proto::TerminalParams::default()
    };
    // The mode of no question, « Autoriser le mode Bypass » on: the refusals are there beside `--allow-all`.
    let told = engine
        .terminal_launch(
            params("bypassPermissions", false, &[]),
            AgentPlace::default(),
            false,
            true,
        )
        .await
        .unwrap();
    let d: Vec<&String> = told
        .launch
        .args
        .iter()
        .filter(|a| a.starts_with("--deny-tool"))
        .collect();
    eprintln!(
        "AUDIT refusals of the Copilot view of an agent in <cwd>:\n  {}",
        d.iter()
            .map(|a| a.replace(&plain(&cwd), "<cwd>"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    assert!(told.launch.args.iter().any(|a| a == "--allow-all"));
    for name in [".git", ".copilot"] {
        assert!(
            d.iter()
                .any(|a| **a == format!("--deny-tool=write({name})")),
            "{name}"
        );
    }
    assert!(d.iter().any(|a| a.contains("hooks")));
    // No refusal names a command: in the mode of no question, a command writes where a write is refused.
    assert!(!d.iter().any(|a| a.contains("shell")));
    // The setting off: the core takes what opens the mode of no question out of what the plugin described.
    let held = engine
        .terminal_launch(
            params("bypassPermissions", false, &[]),
            AgentPlace::default(),
            false,
            false,
        )
        .await
        .unwrap();
    assert!(!held.launch.args.iter().any(|a| a == "--allow-all"));
    // A discussion writes nothing and runs nothing.
    let read_only = engine
        .terminal_launch(
            params("default", true, &[]),
            AgentPlace::default(),
            false,
            true,
        )
        .await
        .unwrap()
        .launch;
    assert!(read_only.read_only);
    assert!(read_only.args.contains(&"--deny-tool=write".to_string()));
    assert!(read_only.args.contains(&"--deny-tool=shell".to_string()));
    // A fenced agent has no view of Copilot CLI: nothing in it would keep the fence.
    let fenced = engine
        .terminal_launch(
            params("default", false, &[plain(&home.join("private"))]),
            AgentPlace::default(),
            false,
            true,
        )
        .await;
    let why = fenced.err().map(|e| format!("{e:#}")).unwrap_or_default();
    eprintln!("AUDIT the Copilot view of a fenced agent: {why}");
    assert!(!why.is_empty(), "a fenced agent got a view of Copilot CLI");
    rt.shutdown();
    let _ = std::fs::remove_dir_all(&home);
}

/// `escouade-hook` is run by its whole path, or by its name with its folder first on the `PATH` and
/// `NoDefaultCurrentDirectoryInExePath` when a line cannot hold the path: never a program of the agent's
/// folder. The hooks of a view are written by its plugin, on `escouade_acp::terminal` (the crate the plugins of
/// Mistral Vibe, Gemini CLI and Cursor CLI are built on): its launch is read here, then, under Windows, `cmd` is run as
/// a hook runs it, in a folder that holds a decoy named as the hook.
#[test]
#[ignore = "audit 2026-11"]
fn holds_a_hook_never_runs_a_program_of_the_agents_folder() {
    use escouade_acp::terminal::{path_first, vibe, vibe_hooks, NO_CURRENT_DIR};
    let system = r"C:\Windows\System32";
    let params = |command: &str| proto::TerminalParams {
        agent: "a1".into(),
        cwd: r"C:\work\demo".into(),
        home: Some(r"C:\data\agents\a1".into()),
        hook_command: Some(command.into()),
        hook_url: "http://127.0.0.1:1/hook".into(),
        hook_url_var: "ESCOUADE_HOOK_URL".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        path: Some(system.into()),
        private_file: r"C:\data\tui\a1.json".into(),
        ..proto::TerminalParams::default()
    };
    // A path a line can hold: the hook is called by it, whole.
    let whole = vibe_hooks(r"C:\Apps\Escouade\escouade-hook.exe");
    assert!(
        whole.contains("C:/Apps/Escouade/escouade-hook.exe"),
        "{whole}"
    );
    let mut by_name = 0;
    for odd in [
        r"C:\Program Files\Escouade\escouade-hook.exe",
        r"C:\a&calc\escouade-hook.exe",
        r"C:\a;b\escouade-hook.exe",
        r"C:\a`b\escouade-hook.exe",
        r"C:\a$(x)\escouade-hook.exe",
        r"C:\a^b\escouade-hook.exe",
        r"C:\a|b\escouade-hook.exe",
        "C:\\a\nb\\escouade-hook.exe",
        r"C:\a b\escouade-hook.exe",
        r"C:\é\escouade-hook.exe",
    ] {
        // Called by its name alone: nothing of the path is in the line a shell reads.
        let hooks = vibe_hooks(odd);
        assert!(
            hooks.contains("command = \"escouade-hook\""),
            "{odd:?}: {hooks}"
        );
        let env = vibe(&params(odd)).env;
        assert!(
            env.iter()
                .any(|v| v.name == NO_CURRENT_DIR && v.value == "1"),
            "{odd:?}"
        );
        // Its folder first on the PATH, unless it would cut the PATH in two (a `;`): the PATH is then left as it is.
        let path = env
            .iter()
            .find(|v| v.name == "PATH")
            .map(|v| v.value.clone());
        assert_eq!(path, path_first(odd, system), "{odd:?}");
        match &path {
            Some(p) => assert!(
                p.ends_with(&format!(";{system}")) && !p.starts_with(system),
                "{p}"
            ),
            None => assert!(odd.contains(';'), "{odd:?}"),
        }
        by_name += 1;
    }
    // A folder that is not whole is never put first: it would name a current folder.
    for not_whole in [
        r"C:escouade-hook.exe",
        r"escouade-hook.exe",
        r"..\escouade-hook.exe",
        r"\\?\C:\a b\escouade-hook.exe",
    ] {
        assert_eq!(path_first(not_whole, system), None, "{not_whole:?}");
    }
    eprintln!("AUDIT hooks called by their name, the variable set: {by_name} of {by_name} paths a line cannot hold");
    // Under Windows, the same on the wire: `cmd /C escouade-hook` in a folder that holds a decoy.
    #[cfg(windows)]
    {
        let root = dir("hook-decoy");
        let (app, agent) = (
            root.join("Program Files").join("Escouade"),
            root.join("agent"),
        );
        std::fs::create_dir_all(&app).unwrap();
        std::fs::create_dir_all(&agent).unwrap();
        // Each writes its name in the folder `cmd` runs in.
        std::fs::write(app.join("escouade-hook.cmd"), "@echo real> ran.txt\r\n").unwrap();
        std::fs::write(agent.join("escouade-hook.cmd"), "@echo decoy> ran.txt\r\n").unwrap();
        let command = plain(&app.join("escouade-hook.cmd"));
        let system = std::env::var("SystemRoot")
            .map(|r| format!(r"{r}\System32"))
            .unwrap_or_default();
        let run = |guarded: bool| {
            let _ = std::fs::remove_file(agent.join("ran.txt"));
            let mut cmd = std::process::Command::new("cmd");
            cmd.args(["/C", "escouade-hook"])
                .current_dir(&agent)
                .env_remove(NO_CURRENT_DIR)
                .env("PATH", path_first(&command, &system).unwrap());
            if guarded {
                cmd.env(NO_CURRENT_DIR, "1");
            }
            assert!(cmd.status().unwrap().success());
            std::fs::read_to_string(agent.join("ran.txt"))
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let (with, without) = (run(true), run(false));
        eprintln!(
            "AUDIT cmd /C escouade-hook in a folder that holds a decoy: with the variable it ran « {with} », without it « {without} »"
        );
        assert_eq!(with, "real", "the decoy of the agent's folder ran");
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// What an interface's plugin may write beside its private file: the file itself, and names that start with its own.
#[test]
#[ignore = "audit 2026-11"]
fn holds_beside_takes_the_private_file_and_names_that_start_with_it_alone() {
    let base = dir("beside");
    let private = base.join("agents").join("a1").join("settings.json");
    let at = |name: &str| plain(&base.join("agents").join("a1").join(name));
    for ok in ["settings.json", "settings.json.lock", "settings.local.json"] {
        assert!(super::harness::beside(&private, &at(ok)), "{ok}");
    }
    for no in [
        "other.json",
        "settingsx.json",
        "xsettings.json",
        "settings",
        ".settings.json",
    ] {
        assert!(!super::harness::beside(&private, &at(no)), "{no}");
    }
    let dir_of = plain(private.parent().unwrap());
    for no in [
        format!("{dir_of}/../a2/settings.json"),
        format!("{dir_of}/settings.json/../../x"),
        plain(&base.join("agents").join("a2").join("settings.json")),
        plain(&base.join("settings.json")),
        "settings.json".to_string(),
        String::new(),
    ] {
        assert!(!super::harness::beside(&private, &no), "{no}");
    }
    // Noted in the report (SEC-06): a name that starts with the stem may be a program's.
    assert!(super::harness::beside(&private, &at("settings.cmd")));
}

/// What the manifest of anyone's plugin says of the harness it describes,
/// beyond its program: the variables it takes off are judged as those it sets, at the manifest and again at each launch;
/// the commands « that install it » and « that update it » and the variables taken off are shown by the installation; and
/// the app says those commands are its plugin's, not its own. Every part is tried before any is judged, so that a version
/// that lacks one says all it lacks. The limit, printed: at a launch the rule is a list of refusals, and a variable of
/// no refused family is still taken off. Nothing is installed nor run but a `set` of the system.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "audit 2026-11"]
async fn limit_what_a_described_harness_words_is_shown_and_judged() {
    let install = "AUDIT-POC: a line to paste in a terminal";
    let update = "AUDIT-POC: another line to paste";
    let taken_off = [
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "NO_PROXY",
        "NODE_EXTRA_CA_CERTS",
        "SSL_CERT_FILE",
    ];
    let manifest_of = |acp: Value| {
        json!({
            "id": "dev.exemple.audit",
            "name": "Audit",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.2.0" },
            "contributes": { "harnesses": [{
                "id": "audit",
                "name": "Audit",
                "locate": { "names": ["audit-poc-harness-that-is-not-installed"] },
                "tested": { "oldest": "9.0.0", "newest": "9.0.0" },
                "install": { "windows": install, "unix": install },
                "updateCommand": update,
                "acp": acp,
            }] },
        })
    };
    let mut lacking: Vec<String> = Vec::new();
    // 1. The manifest of a third party that takes the proxy or the authorities off is refused, each variable as the
    // same variable set is.
    let with = manifest_of(json!({ "args": ["acp"], "removeEnv": taken_off }));
    let said = format!("{:?}", manifest::check(&with, Hand::Third).manifest.err());
    for (i, name) in taken_off.iter().enumerate() {
        if !said.contains(&format!("acp.removeEnv[{i}]")) {
            lacking.push(format!("the manifest may take {name} off its harness"));
        }
    }
    // A variable of no refused family may be taken off: that manifest holds, and is what the rest is tried on.
    let harmless = manifest_of(json!({ "args": ["acp"], "removeEnv": ["AUDIT_NOTE"] }));
    let m = manifest::check(&harmless, Hand::Third)
        .manifest
        .unwrap_or_else(|p| panic!("the manifest is refused: {p:?}"));
    // 2. What the installation shows says the two commands and the variable taken off.
    let shown = super::asks(&m).join("\n  ");
    eprintln!("AUDIT what the installation shows of the plugin:\n  {shown}");
    for (what, word) in [
        ("the command that installs it", install),
        ("the command that updates it", update),
        ("the variable it takes off", "AUDIT_NOTE"),
    ] {
        if !shown.contains(word) {
            lacking.push(format!("the installation does not show {what}"));
        }
    }
    // 3. Where the app says the command that updates it, it says whose it is.
    let decl = super::described::harnesses_of(&m.contributes).remove(0);
    let rt = crate::plugins::builtin::test_runtime("audit-described-words");
    let engine = super::harness::ProcessEngine::described(rt.clone(), "dev.exemple.audit", decl);
    let too_old = crate::engine::registry::too_old(crate::i18n::Lang::En, &engine, "1.0.0");
    eprintln!("AUDIT the app's words for a version older than the manifest asks: {too_old}");
    assert!(too_old.contains(update));
    if !too_old.contains("dev.exemple.audit") {
        lacking.push("the app says the command as its own, with no word of the plugin".into());
    }
    // 4. A launch: whatever a description says (one no manifest check saw, the generic plugin handed it), the core keeps
    // what it gives. The next program of an agent is judged as its first is.
    let mut names: Vec<String> = taken_off.iter().map(|v| v.to_string()).collect();
    names.extend(
        [
            "https_proxy",
            "Https_Proxy",
            "all_proxy",
            "SSL_CERT_DIR",
            "CURL_CA_BUNDLE",
            "REQUESTS_CA_BUNDLE",
            "CODEX_CA_CERTIFICATE",
            "node_extra_ca_certs",
            "GIT_SSL_CAINFO",
            "ESCOUADE_HARNESS_HOME",
            "ESCOUADE_MCP_TOKEN",
            "PATH",
            "HOME",
        ]
        .map(String::from),
    );
    let kept_from_it = names.len();
    // Those no family names, whatever they are for: they go.
    let others = [
        "AUDIT_NOTE",
        "TERM",
        "LANG",
        "NoDefaultCurrentDirectoryInExePath",
        "AWS_CA_BUNDLE",
        "DENO_CERT",
        "PIP_CERT",
        "CARGO_HTTP_CAINFO",
        "PROXY_URL",
        "GRPC_DEFAULT_SSL_ROOTS_FILE_PATH",
    ];
    names.extend(others.map(String::from));
    let launch = proto::Launch {
        remove_env: names,
        ..proto::Launch::default()
    };
    let next = super::harness::next_launch("Audit", &engine.vars(), &launch, false).unwrap();
    rt.shutdown();
    let still: Vec<&String> = next
        .remove_env
        .iter()
        .filter(|v| !others.contains(&v.as_str()))
        .collect();
    eprintln!(
        "AUDIT of {kept_from_it} variables of a refused family or given by the core, a launch of anyone's still takes off {}: {still:?}",
        still.len()
    );
    eprintln!(
        "AUDIT variables of no refused family a launch of anyone's takes off ({} of {}): {}",
        next.remove_env
            .iter()
            .filter(|v| others.contains(&v.as_str()))
            .count(),
        others.len(),
        next.remove_env.join(" ")
    );
    if !still.is_empty() {
        lacking.push(format!("a launch takes off what the core gives: {still:?}"));
    }
    // On the wire: the system's own `set`, started as a harness is with the proxy of the settings among its
    // variables, and what that launch may still take off.
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd.exe", &["/C", "set"])
    } else {
        ("/usr/bin/env", &[])
    };
    let program = match std::env::var("SystemRoot") {
        Ok(root) if cfg!(windows) => PathBuf::from(root).join("System32").join(program),
        _ => PathBuf::from(program),
    };
    let program = escouade_engines::program::Program::Native(program);
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let given = vec![(
        "HTTPS_PROXY".to_string(),
        "http://proxy.invalid:3128".to_string(),
    )];
    let cwd = plain(&std::env::temp_dir());
    let out = super::harness::command_of(&program, &cwd, &args, &given, &next.remove_env)
        .unwrap()
        .output()
        .await
        .unwrap();
    let has_proxy = String::from_utf8_lossy(&out.stdout)
        .to_uppercase()
        .contains("HTTPS_PROXY=HTTP://PROXY.INVALID:3128");
    eprintln!("AUDIT the proxy of the settings in the variables of that launch: {has_proxy}");
    if !has_proxy {
        lacking.push("the harness runs without the proxy of the settings".into());
    }
    assert!(
        lacking.is_empty(),
        "a described harness says more than is shown or judged:\n  {}",
        lacking.join("\n  ")
    );
}

/// The short name Windows gives the folder `path` (`FENCED~1`), when its volume keeps them and it differs from its name.
#[cfg(windows)]
fn short_name(path: &Path) -> Option<String> {
    let out = std::process::Command::new("cmd")
        .args(["/C", "for", "%I", "in", "(.)", "do", "@echo", "%~snxI"])
        .current_dir(path)
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let long = path.file_name()?.to_string_lossy().into_owned();
    (!name.is_empty() && !name.eq_ignore_ascii_case(&long)).then_some(name)
}

#[cfg(not(windows))]
fn short_name(_: &Path) -> Option<String> {
    None
}

/// The refusals by path against the other spellings Windows has of one folder: its short name, a dot or a space after
/// its name, a stream of its directory, another case. A fenced folder is not read by any of them, and a file of a
/// launch is not taken for one of the agent's folder by any of them.
#[test]
#[ignore = "audit 2026-11"]
fn holds_another_spelling_of_windows_does_not_get_around_a_refusal_by_path() {
    if !cfg!(windows) {
        return;
    }
    let long = "fenced-folder-of-the-audit";
    let f = Files::fenced(&["private", &format!("wt/{long}")]);
    let fenced = f.base.join("wt").join(long);
    std::fs::create_dir_all(&fenced).unwrap();
    std::fs::write(fenced.join("d.txt"), "privé").unwrap();
    let read = |path: &str| json!({ "agent": "a", "root": "agent", "path": path });
    f.refused("fs/read", read(&format!("{long}/d.txt")));
    let mut spellings = vec![
        format!("{long}./d.txt"),
        format!("{long} /d.txt"),
        format!("{long}::$INDEX_ALLOCATION/d.txt"),
        format!("{long}:$I30:$INDEX_ALLOCATION/d.txt"),
        format!("{long}/d.txt::$DATA"),
        format!("{long}/d.txt."),
        format!("{long}/d.txt "),
        format!("{}/D.TXT", long.to_uppercase()),
        format!("./{long}/./d.txt"),
        format!("{long}//d.txt"),
        format!("{long}\\d.txt"),
    ];
    let short = short_name(&fenced);
    match &short {
        Some(s) => {
            spellings.push(format!("{s}/d.txt"));
            spellings.push(format!("{}/D.TXT", s.to_lowercase()));
        }
        None => eprintln!("AUDIT short names: this volume keeps none, that spelling is not tried"),
    }
    let passed: Vec<&String> = spellings
        .iter()
        .filter(|path| {
            f.call("fs/read", read(path))
                .is_ok_and(|v| v["text"] == "privé")
        })
        .collect();
    eprintln!(
        "AUDIT a fenced folder asked by {} other spellings (short name: {}): {} read",
        spellings.len(),
        if short.is_some() {
            "tried"
        } else {
            "none on this volume"
        },
        passed.len()
    );
    assert!(passed.is_empty(), "the fence was passed by {passed:?}");
    // The same for a write.
    for path in &spellings {
        let _ = f.call(
            "fs/write",
            json!({ "agent": "a", "root": "agent", "path": path, "text": "AUDIT-POC" }),
        );
    }
    assert_eq!(
        std::fs::read_to_string(fenced.join("d.txt")).unwrap(),
        "privé",
        "a write reached the fenced folder"
    );

    // The files of a launch: a path is of the agent's folder by its letter alone, so another spelling of that folder
    // is refused (the safe side), and none of them leads out of it.
    let home = f.base.join("home-of-the-agent-in-full");
    std::fs::create_dir_all(&home).unwrap();
    let h = plain(&home);
    assert_eq!(judged(&home, vec![file(home.join("x.txt"), "")]), Ok(()));
    let mut others = vec![
        format!("{h}.\\x.txt"),
        format!("{h} \\x.txt"),
        format!("{h}::$INDEX_ALLOCATION\\x.txt"),
        format!("{h}\\sub\\..\\..\\x.txt"),
        format!("{h}\\sub.\\..\\..\\x.txt"),
        format!(
            "{h}\\..\\{}\\..\\x.txt",
            home.file_name().unwrap().to_string_lossy()
        ),
    ];
    if let Some(s) = short_name(&home) {
        others.push(plain(&home.with_file_name(s).join("x.txt")));
    }
    for path in others {
        assert!(
            judged(&home, vec![file(&path, "")]).is_err(),
            "{path:?} was accepted"
        );
    }
}
