//! Tests of « Plugins › Installés › Depuis un dossier… » on the folder of a plugin as its author has it (issue #43, ESC-652):
//! its copy leaves out what is not the plugin (`target/`, `.git/`, `node_modules/`, what Git ignores) and keeps of
//! `packs/` the module of the manifest's version alone; a `run.wasm` built there installs and starts, its journal naming
//! the file it loaded; a folder without a module is refused with what to do; a link that goes out is never copied.

use crate::i18n::Lang;
use crate::journal::Kind;
use crate::paths::DataDir;
use crate::plugins_reactor_tests::fixture;
use crate::runtime::{Limits, Runtime};
use crate::secrets::MemoryStore;
use crate::wasm;
use crate::Plugins;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ID: &str = "dev.test.reactor";
/// What `escouade-plugin build` writes for the version of the fixture's manifest.
const BUILT: &str = "packs/dev.test.reactor-1.0.0.wasm";

fn base(name: &str) -> PathBuf {
    let b = std::env::temp_dir().join(format!(
        "plugin-folder-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&b).unwrap();
    b
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The folder of a `run.wasm` plugin as its author has it after `escouade-plugin build`: its manifest, its Rust, no
/// `plugin.wasm` beside the manifest, its module in `packs/` with the one of an older version, Rust's compilation
/// (`target/`, a big file), npm's packages, and what its `.gitignore` names.
fn authored(at: &Path) -> PathBuf {
    let dir = at.join("reactor");
    for f in [
        "escouade-plugin.json",
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
    ] {
        write(&dir.join(f), &std::fs::read(fixture().join(f)).unwrap());
    }
    let module = std::fs::read(fixture().join(wasm::MODULE_FILE)).unwrap();
    write(&dir.join(BUILT), &module);
    write(&dir.join("packs/dev.test.reactor-0.9.0.wasm"), &module);
    write(
        &dir.join("packs/dev.test.reactor-1.0.0.tar.gz"),
        b"an archive",
    );
    write(
        &dir.join("target/wasm32-wasip1/release/big.rlib"),
        &vec![0u8; 8 << 20],
    );
    write(
        &dir.join("node_modules/left-pad/index.js"),
        b"module.exports = 1;",
    );
    write(&dir.join("sub/node_modules/x.js"), b"1");
    write(&dir.join("sub/kept.txt"), b"kept");
    write(&dir.join(".gitignore"), b"*.log\nbuild/\npacks/\n");
    write(&dir.join("debug.log"), b"a log");
    write(&dir.join("build/out.txt"), b"an output");
    dir
}

/// The files of `dir`, by their path in it, sorted.
fn files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, at: &str, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let rel = format!("{at}{}", e.file_name().to_string_lossy());
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), &format!("{rel}/"), out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

fn plugins(at: &Path) -> Arc<Plugins> {
    Plugins::load(DataDir::new(at.join("data")), "2.2.0")
}

#[test]
fn a_folder_s_copy_keeps_the_plugin_and_the_module_of_its_version_alone() {
    let at = base("sift");
    let src = authored(&at);
    git(&src, &["init", "-q"]);
    let plugins = plugins(&at);
    let preview = plugins.inspect_folder(&src, Lang::En).unwrap();
    // The preview says which module runs, covered by the fingerprint it shows.
    assert_eq!(preview.module.as_deref(), Some(BUILT));
    let view = plugins.install(&preview.token, Lang::En).unwrap();
    let copied = PathBuf::from(&view.folder);
    assert_eq!(
        files(&copied),
        [
            ".gitignore",
            "Cargo.lock",
            "Cargo.toml",
            "escouade-plugin.json",
            BUILT,
            "src/lib.rs",
            "sub/kept.txt",
        ],
        "nothing of target/, .git/, node_modules/, what Git ignores nor the other packs"
    );
    assert_eq!(
        preview.fingerprint,
        format!("sha256:{}", crate::digest(&copied).unwrap()),
        "the fingerprint shown is that of what was copied, its module with it"
    );
    assert_eq!(view.state, crate::State::Ready);
}

#[test]
fn what_git_ignores_is_read_from_the_repository_the_folder_is_in() {
    // The repository is the folder's parent: its rules hold for the folder, its paths read from there.
    let at = base("parent");
    let src = authored(&at);
    git(&at, &["init", "-q"]);
    write(&at.join(".gitignore"), b"notes/\n");
    write(&src.join("notes/draft.md"), b"a draft");
    let plugins = plugins(&at);
    let view = plugins.install_folder(&src, Lang::En).unwrap();
    let copied = files(Path::new(&view.folder));
    assert!(
        !copied.iter().any(|f| f.starts_with("notes/")),
        "{copied:?}"
    );
    assert!(!copied.contains(&"debug.log".to_string()), "{copied:?}");
    assert!(copied.contains(&BUILT.to_string()), "{copied:?}");
}

#[test]
fn out_of_a_repository_the_fixed_list_alone_is_left_out() {
    let at = base("norepo");
    let src = authored(&at);
    let plugins = plugins(&at);
    let view = plugins.install_folder(&src, Lang::En).unwrap();
    let copied = files(Path::new(&view.folder));
    // No repository: its .gitignore is a file like another, not read.
    assert!(copied.contains(&"debug.log".to_string()), "{copied:?}");
    assert!(copied.contains(&"build/out.txt".to_string()), "{copied:?}");
    assert!(!copied.iter().any(|f| f.starts_with("target/")
        || f.contains("node_modules/")
        || f == "packs/dev.test.reactor-0.9.0.wasm"
        || f == "packs/dev.test.reactor-1.0.0.tar.gz"));
    assert!(copied.contains(&BUILT.to_string()), "{copied:?}");
}

#[test]
fn a_folder_without_a_built_module_is_refused_and_says_what_to_do() {
    let at = base("unbuilt");
    let src = authored(&at);
    std::fs::remove_dir_all(src.join("packs")).unwrap();
    let plugins = plugins(&at);
    let fr = plugins.inspect_folder(&src, Lang::Fr).unwrap_err().message;
    assert!(
        fr.contains("Ce dossier n'a pas de module construit : lance `escouade-plugin build`, ou charge-le depuis En développement › Charger un dossier…"),
        "{fr}"
    );
    let en = plugins.inspect_folder(&src, Lang::En).unwrap_err().message;
    assert!(
        en.contains("This folder has no built module: run `escouade-plugin build`, or load it from In development › Load a folder…"),
        "{en}"
    );
    // Only the module of another version: refused too.
    let module = std::fs::read(fixture().join(wasm::MODULE_FILE)).unwrap();
    write(&src.join("packs/dev.test.reactor-0.9.0.wasm"), &module);
    assert!(plugins.inspect_folder(&src, Lang::En).is_err());
    // Nothing left aside.
    let left: Vec<_> = std::fs::read_dir(at.join("data/plugins"))
        .map(|d| d.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
    // Its plugin.wasm beside the manifest, as before: taken.
    write(&src.join(wasm::MODULE_FILE), &module);
    let p = plugins.inspect_folder(&src, Lang::En).unwrap();
    assert_eq!(p.module.as_deref(), Some(wasm::MODULE_FILE));
}

fn journal(rt: &Runtime) -> Vec<String> {
    rt.journal(ID)
        .into_iter()
        .filter(|e| e.kind == Kind::Start)
        .map(|e| e.text)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_wasm_built_in_packs_installs_from_a_folder_starts_and_its_journal_names_its_module()
{
    let at = base("start");
    let src = authored(&at);
    let data = DataDir::new(at.join("data"));
    let plugins = Plugins::load(data.clone(), "2.2.0");
    let view = plugins.install_folder(&src, Lang::En).unwrap();
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        "2.2.0",
        Limits::default(),
    );
    let r = rt.call(ID, "probe/turns", json!({})).await.unwrap();
    assert_eq!(r["boot"], false, "{r}");
    let module = Path::new(&view.folder)
        .join(wasm::PACKS)
        .join("dev.test.reactor-1.0.0.wasm");
    let started = journal(&rt);
    assert!(
        started.iter().any(|t| t.starts_with(&format!(
            "started {} in WASM, isolated: its module precompiled",
            module.display()
        ))),
        "{started:?}"
    );
    assert!(
        !started.iter().any(|t| t.contains(wasm::MODULE_FILE)),
        "{started:?}"
    );
    // Started again: from the cache, and said so with the file it was made of.
    rt.shutdown();
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        "2.2.0",
        Limits::default(),
    );
    rt.call(ID, "probe/turns", json!({})).await.unwrap();
    let started = journal(&rt);
    assert!(
        started
            .iter()
            .any(|t| t.contains(&module.display().to_string())
                && t.contains("loaded from the cache")),
        "{started:?}"
    );
    rt.shutdown();
}

/// A link of the folder is never followed nor copied, whether it names a file or a folder out of it: a `packs/` that
/// is a link to a module elsewhere leaves the folder without one, refused.
#[test]
fn a_link_out_of_the_folder_is_never_copied() {
    let at = base("links");
    let src = authored(&at);
    let outside = at.join("outside");
    write(&outside.join("secret.txt"), b"a secret");
    let module = std::fs::read(fixture().join(wasm::MODULE_FILE)).unwrap();
    write(&outside.join("packs/dev.test.reactor-1.0.0.wasm"), &module);
    std::fs::remove_dir_all(src.join("packs")).unwrap();
    if !link_dir(&outside.join("packs"), &src.join("packs")) {
        eprintln!("no link can be made here: skipped");
        return;
    }
    let _ = link_file(&outside.join("secret.txt"), &src.join("secret.txt"));
    let plugins = plugins(&at);
    let e = plugins.inspect_folder(&src, Lang::En).unwrap_err().message;
    assert!(e.contains("no built module"), "{e}");
    // With a module of its own, it installs, and nothing of the links is in its copy.
    write(&src.join(wasm::MODULE_FILE), &module);
    let view = plugins.install_folder(&src, Lang::En).unwrap();
    let copied = files(Path::new(&view.folder));
    assert!(
        !copied
            .iter()
            .any(|f| f.starts_with("packs") || f == "secret.txt"),
        "{copied:?}"
    );
}

#[cfg(unix)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

/// A junction: no privilege needed, and Rust reads it as a link.
#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}
