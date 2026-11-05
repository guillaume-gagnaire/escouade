//! Tests of `escouade-plugin build` on a `run.wasm` plugin (ESC-335, docs/PLUGINS.md §8.5): each refusal, said in French
//! and in English; the closed list; the toolchain's checks; and, by the real program with the real toolchain, the fixture
//! tests/fixtures/plugins/rust-build compiled twice to the hash of goldens.json, whatever its folder and its target, then
//! played by the bench. That hash is the same on every system: this test proves it on each one of CI, and
//! `scripts/plugin-build-linux.sh` under Linux from any machine with a container engine.

use crate::i18n::Lang;
use crate::paths::test_dir;
use crate::plugins::bench::cli;
use escouade_plugin_cli::rust::{self, toml, Native, Refusal, Runs};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CDYLIB: &str = "[package]\nname = \"p\"\nversion = \"1.0.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[workspace]\n";
const LOCK: &str = "version = 4\n\n[[package]]\nname = \"p\"\nversion = \"1.0.0\"\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A folder holding `files`, the sources of a plugin in Rust with its lock unless they say otherwise.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = test_dir(&format!("rust-build-{name}"));
    let mut all = vec![("Cargo.toml", CDYLIB), ("Cargo.lock", LOCK)];
    all.extend_from_slice(files);
    for (file, text) in all {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn manifest(more: &str) -> String {
    format!("{CDYLIB}{more}\n")
}

fn refused(name: &str, files: &[(&str, &str)]) -> Refusal {
    rust::check(&folder(name, files), "Cargo.toml").expect_err(name)
}

#[test]
fn what_cargo_fetch_says_of_a_lock_is_the_plugins_fault_and_a_network_is_not() {
    for said in [
        "error: the lock file /w/Cargo.lock needs to be updated but --locked was passed",
        "error: failed to parse lock file at: /w/Cargo.lock",
        "error: failed to select a version for the requirement `serde = \"=1.0.999\"`",
        "error: checksum for `serde v1.0.228` changed between lock files",
    ] {
        assert!(rust::lock_at_fault(said), "{said}");
    }
    assert!(!rust::lock_at_fault(
        "error: failed to download from `https://static.crates.io/crates/serde/serde-1.0.228.crate`"
    ));
    let why = Refusal::StaleLock("--locked".into());
    assert!(why.message(Lang::Fr).contains("ne correspond pas"));
    assert!(why.message(Lang::En).contains("does not match"));
}

#[test]
fn a_plugin_without_its_lock_is_refused_with_the_reason() {
    let dir = folder("no-lock", &[]);
    std::fs::remove_file(dir.join("Cargo.lock")).unwrap();
    let why = rust::check(&dir, "Cargo.toml").unwrap_err();
    assert_eq!(why, Refusal::NoLock("Cargo.lock".into()));
    assert!(why.message(Lang::Fr).contains("commite-le"));
    assert!(why.message(Lang::En).contains("commit it"));
}

#[test]
fn a_patch_or_a_replace_is_refused() {
    for (table, text) in [
        ("patch", "[patch.crates-io]\nserde = { path = \"x\" }"),
        ("replace", "[replace]\n\"serde:1.0.0\" = { path = \"x\" }"),
    ] {
        let m = manifest(text);
        let why = refused(table, &[("Cargo.toml", &m)]);
        assert_eq!(why, Refusal::Patch(table.into()));
        assert!(why.message(Lang::Fr).contains("n'est pas permis"));
        assert!(why.message(Lang::En).contains("is not allowed"));
    }
}

#[test]
fn a_path_that_leaves_the_folder_is_refused_in_every_table() {
    for (name, text) in [
        ("deps", "[dependencies]\nkit = { path = \"../kit\" }"),
        ("deep", "[dependencies]\nkit = { path = \"a/../../kit\" }"),
        ("build", "[build-dependencies]\nkit = { path = \"../kit\" }"),
        ("dev", "[dev-dependencies]\nkit = { path = \"../kit\" }"),
        (
            "target",
            "[target.'cfg(windows)'.dependencies]\nkit = { path = \"../kit\" }",
        ),
        (
            "workspace",
            "[workspace.dependencies]\nkit = { path = \"../kit\" }",
        ),
        ("absolute", "[dependencies]\nkit = { path = \"/etc/kit\" }"),
    ] {
        let m = manifest(text);
        let why = refused(name, &[("Cargo.toml", &m)]);
        assert!(
            matches!(&why, Refusal::PathOutside(kit, _) if kit == "kit"),
            "{name}: {why:?}"
        );
        assert!(why.message(Lang::Fr).contains("sort du dossier"));
        assert!(why.message(Lang::En).contains("leaves the plugin's folder"));
    }
    let members = format!("{}members = [\"../other\"]\n", CDYLIB);
    assert_eq!(
        refused("members", &[("Cargo.toml", &members)]),
        Refusal::PathOutside("workspace.members".into(), "../other".into())
    );
}

#[test]
fn a_path_inside_the_folder_goes_through() {
    let m = manifest("[dependencies]\ninner = { path = \"crates/inner\" }");
    let dir = folder("inside", &[("Cargo.toml", &m)]);
    assert!(rust::check(&dir, "Cargo.toml").is_ok());
    // Named from a crate in a folder: its `..` stays in the plugin's.
    let nested = manifest("[dependencies]\ninner = { path = \"../inner\" }");
    let dir = folder(
        "nested",
        &[("rust/Cargo.toml", &nested), ("rust/Cargo.lock", LOCK)],
    );
    assert!(rust::check(&dir, "rust/Cargo.toml").is_ok());
}

#[test]
fn what_does_not_come_from_crates_io_is_refused() {
    for (name, text) in [
        (
            "git",
            "[dependencies]\nserde = { git = \"https://example.dev/s\" }",
        ),
        (
            "registry",
            "[dependencies]\nserde = { version = \"1\", registry = \"mine\" }",
        ),
    ] {
        let m = manifest(text);
        let why = refused(name, &[("Cargo.toml", &m)]);
        assert_eq!(why, Refusal::NotCratesIo("serde".into()));
        assert!(why.message(Lang::Fr).contains("crates.io"));
        assert!(why.message(Lang::En).contains("crates.io"));
    }
    let lock = format!("{LOCK}\n[[package]]\nname = \"q\"\nversion = \"1.0.0\"\nsource = \"git+https://example.dev/q\"\n");
    let why = refused("lock-git", &[("Cargo.lock", &lock)]);
    assert_eq!(
        why,
        Refusal::LockSource("q".into(), "git+https://example.dev/q".into())
    );
    assert!(why.message(Lang::Fr).contains("pas de crates.io"));
    assert!(why.message(Lang::En).contains("not from crates.io"));
    // What crates.io gives, by either protocol, is no refusal.
    for source in [
        "registry+https://github.com/rust-lang/crates.io-index",
        "sparse+https://index.crates.io/",
    ] {
        let lock = format!(
            "{LOCK}\n[[package]]\nname = \"q\"\nversion = \"1.0.0\"\nsource = \"{source}\"\n"
        );
        assert!(rust::check(&folder("lock-ok", &[("Cargo.lock", &lock)]), "Cargo.toml").is_ok());
    }
}

#[test]
fn a_target_other_than_a_cdylib_is_refused() {
    let package = "[package]\nname = \"p\"\nversion = \"1.0.0\"\n\n[workspace]\n";
    for (name, text) in [
        ("no-lib", package.to_string()),
        ("rlib", format!("{package}[lib]\ncrate-type = [\"rlib\"]\n")),
        (
            "staticlib",
            format!("{package}[lib]\ncrate-type = [\"cdylib\", \"staticlib\"]\n"),
        ),
        ("bin", manifest("[[bin]]\nname = \"b\"\npath = \"b.rs\"")),
    ] {
        let why = refused(name, &[("Cargo.toml", &text)]);
        assert!(matches!(why, Refusal::NotCdylib(_)), "{name}: {why:?}");
        assert!(
            why.message(Lang::Fr)
                .contains("crate-type = [\\\"cdylib\\\"]")
                || why.message(Lang::Fr).contains("crate-type = [\"cdylib\"]")
        );
        assert!(why.message(Lang::En).contains("a library"));
    }
    assert_eq!(
        refused("main", &[("src/main.rs", "fn main() {}")]),
        Refusal::NotCdylib("src/main.rs".into())
    );
    assert_eq!(
        refused("src-bin", &[("src/bin/b.rs", "fn main() {}")]),
        Refusal::NotCdylib("src/bin".into())
    );
    // Beside a cdylib, an rlib changes nothing: until Z3, the crates of the first hand are also linked natively.
    let both = format!("{package}[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n");
    assert!(rust::check(&folder("both", &[("Cargo.toml", &both)]), "Cargo.toml").is_ok());
}

#[test]
fn a_script_or_a_macro_of_the_plugin_itself_is_refused() {
    let why = refused("script", &[("build.rs", "fn main() {}")]);
    assert_eq!(why, Refusal::OwnBuildScript("p".into()));
    assert!(why.message(Lang::Fr).contains("en natif"));
    assert!(why.message(Lang::En).contains("in native"));
    let named = CDYLIB.replace("edition", "build = \"gen.rs\"\nedition");
    assert_eq!(
        refused("named", &[("Cargo.toml", &named)]),
        Refusal::OwnBuildScript("p".into())
    );
    let none = CDYLIB.replace("edition", "build = false\nedition");
    assert!(rust::check(
        &folder(
            "no-script",
            &[("Cargo.toml", &none), ("build.rs", "unused")]
        ),
        "Cargo.toml"
    )
    .is_ok());
    let proc_macro = "[package]\nname = \"p\"\nversion = \"1.0.0\"\n\n[lib]\nproc-macro = true\ncrate-type = [\"cdylib\"]\n\n[workspace]\n";
    let why = refused("macro", &[("Cargo.toml", proc_macro)]);
    assert_eq!(why, Refusal::OwnMacro("p".into()));
    assert!(why.message(Lang::En).contains("procedural macro"));
}

#[test]
fn what_would_change_the_compilation_is_refused() {
    for (name, file) in [
        ("config", ".cargo/config.toml"),
        ("config-old", ".cargo/config"),
        ("nested-config", "crates/inner/.cargo/config.toml"),
        ("toolchain", "rust-toolchain.toml"),
        ("toolchain-old", "rust-toolchain"),
    ] {
        let why = refused(name, &[(file, "")]);
        assert!(matches!(why, Refusal::Config(_)), "{name}: {why:?}");
        assert!(why.message(Lang::Fr).contains("changerait la compilation"));
        assert!(why
            .message(Lang::En)
            .contains("would change the compilation"));
    }
    for (key, text) in [
        ("package", "[profile.release.package.serde]\nopt-level = 0"),
        (
            "build-override",
            "[profile.release.build-override]\nopt-level = 0",
        ),
    ] {
        let m = manifest(text);
        let why = refused(key, &[("Cargo.toml", &m)]);
        assert_eq!(why, Refusal::Profile(format!("profile.release.{key}")));
        assert!(why.message(Lang::En).contains("imposed"));
    }
    // The author's own profile is passed over, not refused.
    let own = manifest("[profile.release]\nopt-level = \"z\"\npanic = \"unwind\"");
    assert!(rust::check(&folder("own", &[("Cargo.toml", &own)]), "Cargo.toml").is_ok());
}

#[test]
fn a_manifest_that_is_not_there_or_does_not_read_is_refused() {
    let dir = folder("manifests", &[]);
    assert_eq!(
        rust::check(&dir, "rust/Cargo.toml").unwrap_err(),
        Refusal::NoManifest("rust/Cargo.toml".into())
    );
    assert_eq!(
        rust::check(&dir, "../Cargo.toml").unwrap_err(),
        Refusal::NoManifest("../Cargo.toml".into())
    );
    let why = refused("broken", &[("Cargo.toml", "[package")]);
    assert!(matches!(why, Refusal::BadManifest(_)));
    let why = refused("broken-lock", &[("Cargo.lock", "[[package")]);
    assert!(matches!(why, Refusal::BadLock(_)));
    assert!(why.message(Lang::En).contains("Cargo.lock does not read"));
}

#[test]
fn the_fixtures_go_through() {
    for fixture in ["reactor", "rust-build"] {
        let dir = root().join("tests/fixtures/plugins").join(fixture);
        assert!(rust::check(&dir, "Cargo.toml").is_ok(), "{fixture}");
    }
}

/// A package of `cargo metadata`, with the kinds of its targets.
fn package(name: &str, source: Option<&str>, kinds: &[&str], root: &Path) -> Value {
    let targets: Vec<Value> = kinds.iter().map(|k| json!({ "kind": [k] })).collect();
    json!({
        "id": name,
        "name": name,
        "version": "1.0.0",
        "source": source,
        "manifest_path": root.join(name).join("Cargo.toml"),
        "targets": targets,
    })
}

/// `cargo metadata` of a plugin `p` over `packages`, each edge a dependant, a dependency and its kind.
fn metadata(root: &Path, packages: &[(&str, &[&str])], edges: &[(&str, &str, Value)]) -> Value {
    let io = "registry+https://github.com/rust-lang/crates.io-index";
    let mut all = vec![package("p", None, &["cdylib"], root)];
    all.extend(packages.iter().map(|(n, k)| package(n, Some(io), k, root)));
    let nodes: Vec<Value> = all
        .iter()
        .map(|p| {
            let id = p["id"].as_str().unwrap();
            let deps: Vec<Value> = edges
                .iter()
                .filter(|(from, _, _)| *from == id)
                .map(|(_, to, kind)| json!({ "pkg": to, "dep_kinds": [{ "kind": kind }] }))
                .collect();
            json!({ "id": id, "deps": deps })
        })
        .collect();
    json!({ "packages": all, "resolve": { "root": "p", "nodes": nodes } })
}

/// A `Cargo.lock` of `names`, the checksum of each its name.
fn lock_of(names: &[&str]) -> toml::Table {
    let packages: Vec<String> = names
        .iter()
        .map(|n| {
            format!("[[package]]\nname = \"{n}\"\nversion = \"1.0.0\"\nchecksum = \"sum-{n}\"\n")
        })
        .collect();
    packages.concat().parse().unwrap()
}

fn listed(names: &[&str]) -> Vec<Native> {
    names
        .iter()
        .map(|n| Native {
            name: (*n).to_string(),
            version: "1.0.0".into(),
            checksum: format!("sum-{n}"),
        })
        .collect()
}

#[test]
fn what_compiles_to_the_module_needs_no_list_what_runs_in_native_does() {
    let dir = Path::new("/plugin");
    let normal = || Value::Null;
    let lock = lock_of(&["plain", "script", "derive", "under", "tool"]);

    // A plain dependency runs in WASM alone.
    let m = metadata(dir, &[("plain", &["lib"])], &[("p", "plain", normal())]);
    assert_eq!(rust::check_packages(&m, &lock, dir, &[]), Ok(()));

    // A build script, by name, version and checksum.
    let m = metadata(
        dir,
        &[("script", &["lib", "custom-build"])],
        &[("p", "script", normal())],
    );
    let why = rust::check_packages(&m, &lock, dir, &[]).unwrap_err();
    assert_eq!(why, Refusal::NotListed("script 1.0.0".into(), Runs::Script));
    assert!(why.message(Lang::Fr).contains("a un build.rs"));
    assert!(why.message(Lang::En).contains("has a build.rs"));
    assert_eq!(
        rust::check_packages(&m, &lock, dir, &listed(&["script"])),
        Ok(())
    );
    let mut other = listed(&["script"]);
    other[0].version = "1.0.1".into();
    assert!(rust::check_packages(&m, &lock, dir, &other).is_err());
    let mut other = listed(&["script"]);
    other[0].checksum = "another".into();
    assert!(rust::check_packages(&m, &lock, dir, &other).is_err());

    // A macro, then what it depends on.
    let m = metadata(
        dir,
        &[("derive", &["proc-macro"]), ("under", &["lib"])],
        &[("p", "derive", normal()), ("derive", "under", normal())],
    );
    let why = rust::check_packages(&m, &lock, dir, &listed(&["under"])).unwrap_err();
    assert_eq!(why, Refusal::NotListed("derive 1.0.0".into(), Runs::Macro));
    assert!(why.message(Lang::En).contains("is a procedural macro"));
    let why = rust::check_packages(&m, &lock, dir, &listed(&["derive"])).unwrap_err();
    assert_eq!(why, Refusal::NotListed("under 1.0.0".into(), Runs::Under));
    assert!(why.message(Lang::Fr).contains("sert un build.rs"));
    assert_eq!(
        rust::check_packages(&m, &lock, dir, &listed(&["derive", "under"])),
        Ok(())
    );

    // What a build script uses runs in native too; what only tests use is not compiled.
    let m = metadata(
        dir,
        &[("script", &["custom-build"]), ("tool", &["lib"])],
        &[
            ("p", "script", normal()),
            ("script", "tool", json!("build")),
        ],
    );
    let why = rust::check_packages(&m, &lock, dir, &listed(&["script"])).unwrap_err();
    assert_eq!(why, Refusal::NotListed("tool 1.0.0".into(), Runs::Under));
    let m = metadata(
        dir,
        &[("script", &["custom-build"])],
        &[("p", "script", json!("dev"))],
    );
    assert_eq!(rust::check_packages(&m, &lock, dir, &[]), Ok(()));
}

#[test]
fn a_crate_of_the_plugin_has_no_script_and_no_macro_and_stays_in_its_folder() {
    let dir = Path::new("/plugin");
    let lock = lock_of(&[]);
    let member = |kinds: &[&str], at: &Path| {
        let mut m = metadata(at, &[], &[]);
        let mut inner = package("inner", None, kinds, at);
        inner["manifest_path"] = json!(at.join("inner/Cargo.toml"));
        m["packages"].as_array_mut().unwrap().push(inner);
        let nodes = m["resolve"]["nodes"].as_array_mut().unwrap();
        nodes[0]["deps"] = json!([{ "pkg": "inner", "dep_kinds": [{ "kind": null }] }]);
        nodes.push(json!({ "id": "inner", "deps": [] }));
        m
    };
    assert_eq!(
        rust::check_packages(&member(&["lib"], dir), &lock, dir, &[]),
        Ok(())
    );
    assert_eq!(
        rust::check_packages(&member(&["custom-build"], dir), &lock, dir, &[]),
        Err(Refusal::OwnBuildScript("inner".into()))
    );
    assert_eq!(
        rust::check_packages(&member(&["proc-macro"], dir), &lock, dir, &[]),
        Err(Refusal::OwnMacro("inner".into()))
    );
    assert!(matches!(
        rust::check_packages(&member(&["lib"], Path::new("/elsewhere")), &lock, dir, &[]),
        Err(Refusal::PathOutside(..))
    ));
}

#[test]
fn the_closed_list_names_each_crate_by_version_and_checksum() {
    let list = rust::native_list();
    assert!(list.iter().any(|n| n.name == "serde_derive"));
    for n in &list {
        assert_eq!(n.checksum.len(), 64, "{} {}", n.name, n.version);
        assert_eq!(list.iter().filter(|o| *o == n).count(), 1);
    }
}

#[test]
fn build_and_validate_say_a_refusal_and_stop() {
    let dir = folder("cli", &[]);
    std::fs::remove_file(dir.join("Cargo.lock")).unwrap();
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string(&json!({
            "id": "dev.test.p",
            "name": "P",
            "version": "1.0.0",
            "description": { "en": "A plugin.", "fr": "Un plugin." },
            "license": "MIT",
            "escouade": { "api": "1", "min": "2.1.0" },
            "run": { "wasm": "Cargo.toml" },
            "activation": ["event:ticket.column"],
            "permissions": { "events": ["ticket.column"] },
        }))
        .unwrap(),
    )
    .unwrap();
    let d = dir.to_string_lossy().to_string();
    for command in ["validate", "build"] {
        let args = vec![command.to_string(), d.clone(), "--lang".into(), "en".into()];
        assert_eq!(cli::main(args), 1, "{command}");
    }
    std::fs::write(dir.join("Cargo.lock"), LOCK).unwrap();
    assert_eq!(cli::main(vec!["validate".into(), d]), 0);
}

#[test]
fn a_toolchain_changed_since_its_download_is_no_longer_it() {
    let dir = test_dir("rust-build-sums");
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("bin/rustc"), b"the program").unwrap();
    std::fs::write(dir.join("readme"), b"a file").unwrap();
    assert!(!rust::same_as_noted(&dir), "nothing noted yet");
    rust::note_sums(&dir).unwrap();
    assert!(rust::same_as_noted(&dir));
    std::fs::write(dir.join("bin/rustc"), b"the program!").unwrap();
    assert!(!rust::same_as_noted(&dir));
    rust::note_sums(&dir).unwrap();
    std::fs::remove_file(dir.join("readme")).unwrap();
    assert!(!rust::same_as_noted(&dir));
}

/// A `.tar.gz` of `(path, text)`.
fn archive(files: &[(&str, &str)], at: &Path) {
    let gz = flate2::write::GzEncoder::new(
        std::fs::File::create(at).unwrap(),
        flate2::Compression::fast(),
    );
    let mut tar = tar::Builder::new(gz);
    for (path, text) in files {
        let mut head = tar::Header::new_gnu();
        head.set_size(text.len() as u64);
        head.set_mode(0o755);
        // Set by hand: `append_data` itself refuses a `..`, which is what an archive could hold.
        head.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
        head.set_cksum();
        tar.append(&head, text.as_bytes()).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
}

#[test]
fn only_the_programs_and_the_libraries_come_out_of_an_archive() {
    let dir = test_dir("rust-build-unpack");
    let (from, to) = (dir.join("a.tar.gz"), dir.join("toolchain"));
    archive(
        &[
            ("rustc-1.98.0/install.sh", "no"),
            ("rustc-1.98.0/rustc/bin/rustc", "yes"),
            ("rustc-1.98.0/rustc/lib/rustlib/x", "yes"),
            ("rustc-1.98.0/rustc/share/doc/x", "no"),
        ],
        &from,
    );
    rust::unpack(&from, &to).unwrap();
    assert!(to.join("bin/rustc").is_file());
    assert!(to.join("lib/rustlib/x").is_file());
    assert!(!to.join("share").exists());
    assert!(!to.join("install.sh").exists());
    archive(&[("rustc-1.98.0/rustc/bin/../../../../evil", "no")], &from);
    assert!(rust::unpack(&from, &to).is_err());
    assert!(!dir.join("evil").exists());
}

/// `escouade-plugin` beside the tests: the wrapper of a compilation is the program itself, never a test's.
fn program() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap();
    let name = format!("escouade-plugin{}", std::env::consts::EXE_SUFFIX);
    assert!(
        dir.join(&name).is_file(),
        "{name} is not built beside the tests (cargo build --bins)"
    );
    dir.join(name)
}

/// The pinned toolchain, in the tools of this machine, downloaded by the first test that needs it.
fn toolchain() -> PathBuf {
    rust::toolchain(&escouade_plugin_cli::paths::tools_data_dir(), &|| {}).unwrap()
}

/// `escouade-plugin <args>` with data folder of its own, so no target to reuse, on the toolchain of the machine.
fn play(tools: &Path, data: &Path, args: &[&str]) -> Output {
    Command::new(program())
        .args(args)
        .env("ESCOUADE_DATA_DIR", data)
        .env(rust::TOOLCHAIN_VAR, tools)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

fn said(o: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A copy of the fixture, in a folder of its own.
fn fixture(name: &str) -> PathBuf {
    let to = test_dir(&format!("rust-build-{name}"))
        .join("a folder")
        .join(name);
    copy(&root().join("tests/fixtures/plugins/rust-build"), &to);
    to
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        if e.path().is_dir() {
            copy(&e.path(), &to.join(e.file_name()));
        } else {
            std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

#[test]
fn the_fixture_compiles_to_the_hash_of_the_goldens_from_any_folder() {
    let tools = toolchain();
    let goldens: Value = serde_json::from_slice(
        &std::fs::read(root().join("crates/escouade-plugin-cli/goldens.json")).unwrap(),
    )
    .unwrap();
    let golden = &goldens["rust"];
    assert_eq!(
        golden["canon"],
        escouade_plugin_cli::canon::sha256(),
        "rustc-canon changed: every module changes with it. Compile the fixture again here and under Linux \
         (scripts/plugin-build-linux.sh), and note the same hash in goldens.json"
    );

    let mut sums = Vec::new();
    for name in ["first", "second"] {
        let (dir, data) = (fixture(name), test_dir(&format!("rust-build-data-{name}")));
        let o = play(&tools, &data, &["build", dir.to_str().unwrap(), "--json"]);
        assert!(o.status.success(), "{}", said(&o));
        let report: Value = serde_json::from_slice(&o.stdout).unwrap();
        let file = PathBuf::from(report["file"].as_str().unwrap());
        assert_eq!(file, dir.join("packs/dev.test.rust-build-1.0.0.wasm"));
        let bytes = std::fs::read(&file).unwrap();
        let sha = escouade_plugin_cli::wasm::sha256(&bytes);
        assert_eq!(report["sha256"], sha);
        assert_eq!(report["size"], bytes.len());
        assert_eq!(
            report["compiler"],
            json!({
                "rust": rust::VERSION,
                "target": rust::TARGET,
                "canon": escouade_plugin_cli::canon::sha256(),
                "lock": golden["lock"],
            })
        );
        let noted =
            std::fs::read_to_string(dir.join("packs/dev.test.rust-build-1.0.0.wasm.sha256"))
                .unwrap();
        assert_eq!(noted, format!("{sha}  dev.test.rust-build-1.0.0.wasm\n"));
        // Its plugin.wasm is not written: a module is not a source.
        assert!(!dir.join("plugin.wasm").exists());
        sums.push(sha);
    }
    assert_eq!(sums[0], sums[1]);
    assert_eq!(golden["module"], sums[0]);
}

#[test]
fn the_bench_compiles_a_run_wasm_and_plays_it() {
    let tools = toolchain();
    let (dir, data) = (fixture("bench"), test_dir("rust-build-data-bench"));
    let d = dir.to_str().unwrap();
    let o = play(&tools, &data, &["test", d, "--json", "--escouade", "2.2.0"]);
    assert!(o.status.success(), "{}", said(&o));
    let report: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(report.to_string().contains("initialize"), "{report}");

    // `dev` too, which starts it and ends at the end of what is typed.
    let o = play(&tools, &data, &["dev", d, "--escouade", "2.2.0"]);
    assert!(o.status.success(), "{}", said(&o));
    assert!(said(&o).contains("protocolVersion"), "{}", said(&o));

    // A module given is played as it is, without the sources.
    let module = data.join("module.wasm");
    std::fs::copy(
        root().join("tests/fixtures/plugins/reactor/plugin.wasm"),
        &module,
    )
    .unwrap();
    std::fs::remove_file(dir.join("src/lib.rs")).unwrap();
    let o = play(
        &tools,
        &data,
        &[
            "test",
            d,
            "--wasm",
            module.to_str().unwrap(),
            "--escouade",
            "2.2.0",
        ],
    );
    assert!(o.status.success(), "{}", said(&o));
    // Without it, the sources are what is played: their error says so.
    let o = play(&tools, &data, &["test", d, "--escouade", "2.2.0"]);
    assert!(!o.status.success(), "{}", said(&o));
}

#[test]
fn version_says_the_toolchain_and_a_toolchain_of_another_form_is_refused() {
    let o = Command::new(program()).arg("--version").output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout).replace('\r', "");
    assert!(
        text.contains(&format!("\nrust {}\n", rust::VERSION)),
        "{text}"
    );
    assert!(
        text.contains(&format!("\nrust-target {}\n", rust::TARGET)),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "\nrustc-canon {}",
            escouade_plugin_cli::canon::sha256()
        )),
        "{text}"
    );

    let (dir, data) = (fixture("named"), test_dir("rust-build-data-named"));
    let nothing = test_dir("rust-build-no-toolchain");
    let o = play(&nothing, &data, &["build", dir.to_str().unwrap()]);
    assert!(!o.status.success());
    assert!(said(&o).contains(rust::TOOLCHAIN_VAR), "{}", said(&o));
}

#[test]
fn a_dependency_with_a_script_off_the_list_is_refused_before_it_runs() {
    let tools = toolchain();
    let (dir, data) = (fixture("off-list"), test_dir("rust-build-data-off-list"));
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        text.replace("serde_json = \"1\"", "serde_json = \"=1.0.100\""),
    )
    .unwrap();
    let cargo = tools
        .join("bin")
        .join(format!("cargo{}", std::env::consts::EXE_SUFFIX));
    let locked = Command::new(cargo)
        .arg("update")
        .current_dir(&dir)
        .env("CARGO_HOME", data.join("home"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .output()
        .unwrap();
    assert!(locked.status.success(), "{}", said(&locked));
    let o = play(
        &tools,
        &data,
        &["build", dir.to_str().unwrap(), "--lang", "en"],
    );
    assert!(!o.status.success());
    assert!(
        said(&o).contains("serde_json 1.0.100 has a build.rs and is not on the reviewed list"),
        "{}",
        said(&o)
    );
    assert!(!dir.join("packs").exists());
}
