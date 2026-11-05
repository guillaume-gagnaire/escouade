//! `escouade-plugin`, the command of the plugins' authors (PLG-3, ESC-175). Here, what needs no host (ESC-328):
//! `validate` reads a plugin's manifest; `build` compiles a `run.js` plugin's bundle (ESC-322) or a `run.wasm` plugin's
//! crate (ESC-335) to its WASM module and says its SHA-256: what the store and the review compile. None needs Node.
//!
//! `test` and `dev` run the plugin, on the app's host: the app gives them through `more` (`plugins::bench::cli`). The
//! store's program has none. A command that needs no host goes in the `match` of [`main`], and both programs have it.

use crate::i18n::Lang;
use crate::manifest::{self, Manifest};
use crate::{canon, rust, wasm};
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn usage(lang: Lang) -> String {
    tr_in!(
        lang,
        "escouade-plugin : tester un plugin d'Escouade sans l'app.

  escouade-plugin test <dossier> [--json] [--measure [n]] [--wasm <module>] [--offline] [--account <adresse>] [--escouade <version>]
      Joue le banc de conformité : initialisation, version refusée, délais, un fait reçu,
      une décision, un secret, plantage, fermeture. Code de sortie 1 si un scénario échoue.
      --wasm joue ce module (celui du store) au lieu de compiler le plugin ;
      --offline refuse tout fetch, quoi que dise permissions.network ;
      --account donne une adresse à {{account}}, comme l'aurait saisie l'utilisateur.
  escouade-plugin validate <dossier> [--escouade <version>]
      Lit le manifeste (escouade-plugin.json) et dit ce qui ne va pas, champ par champ ; pour un
      plugin `run.js`, que son bundle peut tourner en WASM ; pour un `run.wasm`, que ses sources
      seraient compilées, et que son plugin.wasm, s'il est là, a la forme d'un réacteur.
  escouade-plugin build <dossier> [--out <dossier>] [--json]
      Compile un plugin `run.js` ou `run.wasm` en WASM, comme le store : packs/<id>-<version>.wasm
      et son SHA-256, le même sur tous les systèmes.
  escouade-plugin fetch <dossier>
      Télécharge les dépendances d'un plugin `run.wasm` depuis crates.io, sans rien lancer du
      plugin : `build` les trouve ensuite hors ligne, comme au store.
  escouade-plugin dev <dossier> [--secret <clé>=<valeur>]... [--escouade <version>]
      Le faux Escouade : lance le plugin, puis envoie ce que tu tapes :
        fact <nom> [json]       un fait (events/fact), ex. fact ticket.column
        decide [json]           une demande d'autorisation (permissions/decide)
        call <méthode> [json]   un appel, et sa réponse
        notify <méthode> [json] une notification
        restart                 tue le plugin et le relance
        quit                    shutdown, puis la fin

Un plugin `run.js` tourne en WASM, sans Node : la Javy épinglée ({javy}) est téléchargée à son
premier besoin et vérifiée par son SHA-256 (ESCOUADE_JAVY en nomme une autre).
Un plugin `run.wasm` se compile en Rust par une chaîne d'outils privée (rustc {rust}, {target}),
téléchargée à son premier besoin dans le dossier des outils d'Escouade et vérifiée à chaque usage
(ESCOUADE_RUST_TOOLCHAIN en nomme une autre). Il lui faut un Cargo.lock.

Options : --lang fr|en ; --escouade <version> joue le plugin comme cette version d'Escouade, pour écrire pour une
  version à venir : la plage `escouade` du manifeste et ce que voit `initialize` se font contre elle (celle de
  ce binaire par défaut) ; --first-party pour un plugin d'Escouade (plugins/<id>/ de son dépôt,
  identifiants dev.escouade.*)
",
        "escouade-plugin: test an Escouade plugin without the app.

  escouade-plugin test <folder> [--json] [--measure [n]] [--wasm <module>] [--offline] [--account <address>] [--escouade <version>]
      Plays the conformance bench: initialization, version refused, delays, a fact received,
      a decision, a secret, crash, shutdown. Exit code 1 when a scenario fails.
      --wasm plays this module (the store's) instead of compiling the plugin;
      --offline refuses every fetch, whatever permissions.network says;
      --account gives {{account}} an address, as the user would have typed it.
  escouade-plugin validate <folder> [--escouade <version>]
      Reads the manifest (escouade-plugin.json) and says what is wrong, field by field; for a
      `run.js` plugin, that its bundle can run in WASM; for a `run.wasm` one, that its sources would
      be compiled, and that its plugin.wasm, when it is there, has the form of a reactor.
  escouade-plugin build <folder> [--out <folder>] [--json]
      Compiles a `run.js` or `run.wasm` plugin to WASM, as the store does: packs/<id>-<version>.wasm
      and its SHA-256, the same on every system.
  escouade-plugin fetch <folder>
      Downloads the dependencies of a `run.wasm` plugin from crates.io, running nothing of the
      plugin: `build` then finds them offline, as at the store.
  escouade-plugin dev <folder> [--secret <key>=<value>]... [--escouade <version>]
      The fake Escouade: starts the plugin, then sends what you type:
        fact <name> [json]       a fact (events/fact), e.g. fact ticket.column
        decide [json]            a permission request (permissions/decide)
        call <method> [json]     a call, and its answer
        notify <method> [json]   a notification
        restart                  kills the plugin and starts it again
        quit                     shutdown, then the end

A `run.js` plugin runs in WASM, without Node: the pinned Javy ({javy}) is downloaded the first
time it is needed and checked by its SHA-256 (ESCOUADE_JAVY names another).
A `run.wasm` plugin compiles in Rust by a private toolchain (rustc {rust}, {target}), downloaded the
first time it is needed into Escouade's tools folder and checked at each use
(ESCOUADE_RUST_TOOLCHAIN names another). It needs a Cargo.lock.

Options: --lang fr|en; --escouade <version> plays the plugin as this version of Escouade, to write for a
  version to come: the manifest's `escouade` range and what `initialize` shows go by it (this
  build's by default); --first-party for a plugin of Escouade's own (plugins/<id>/ of its repository,
  ids dev.escouade.*)
",
        javy = wasm::JAVY_VERSION,
        rust = rust::VERSION,
        target = rust::TARGET
    )
}

/// What compiles a plugin here: this escouade-plugin; for `run.js` the pinned Javy, escouade-js and the prelude; for
/// `run.wasm` the pinned rustc, its target and the wrapper.
fn version() -> String {
    format!(
        "escouade-plugin {}
javy {}
escouade-js {}
prelude {}
wasm-since {}
rust {}
rust-target {}
rustc-canon {}",
        env!("CARGO_PKG_VERSION"),
        wasm::JAVY_VERSION,
        match wasm::escouade_js() {
            Ok(_) => wasm::escouade_js_sha256().to_string(),
            Err(_) => "absent".into(),
        },
        wasm::sha256(wasm::PRELUDE.as_bytes()),
        wasm::SERVED_SINCE,
        rust::VERSION,
        rust::TARGET,
        canon::sha256()
    )
}

/// A command for the app to run, as `main` read it.
pub struct Call<'a> {
    pub command: &'a str,
    pub folder: &'a Path,
    pub lang: Lang,
    pub hand: manifest::Hand,
    /// `--escouade <version>`: the version of Escouade the plugin is judged for (this build's by default).
    pub app_version: &'a str,
    pub args: &'a [String],
}

impl Call<'_> {
    pub fn flag(&self, name: &str) -> bool {
        self.args.iter().any(|a| a == name)
    }

    /// The word after the option `name`.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.args
            .iter()
            .position(|a| a == name)
            .and_then(|i| self.args.get(i + 1))
            .map(String::as_str)
    }
}

/// What runs `test` and `dev`: the app's host.
pub type More<'a> = &'a dyn Fn(&Call) -> i32;

/// Runs the command of `args` (without the program's name), `test` and `dev` by `more`; gives its exit code.
pub fn main(args: Vec<String>, more: Option<More>) -> i32 {
    // Cargo running this program as `rustc-canon`, the `RUSTC_WRAPPER` of a `run.wasm` compilation.
    if std::env::var_os(canon::ROOT).is_some() {
        return canon::run(args);
    }
    let mut lang = crate::i18n::system();
    if let Some(i) = args.iter().position(|a| a == "--lang") {
        if let Some(l) = args.get(i + 1) {
            lang = crate::i18n::of_locale(l);
        }
    }
    let Some(command) = args.first().map(String::as_str) else {
        eprint!("{}", usage(lang));
        return 2;
    };
    if matches!(command, "-h" | "--help" | "help") {
        print!("{}", usage(lang));
        return 0;
    }
    if matches!(command, "-V" | "--version" | "version") {
        println!("{}", version());
        // Built without escouade-js, it plays no `run.js` plugin: the release checks it is there.
        return i32::from(wasm::escouade_js().is_err());
    }
    let Some(folder) = args
        .get(1)
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from)
    else {
        eprint!("{}", usage(lang));
        return 2;
    };
    let flag = |name: &str| args.iter().any(|a| a == name);
    // `--escouade <version>`: the version of Escouade the plugin is judged for (this build's by default).
    let app_version = args
        .iter()
        .position(|a| a == "--escouade")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    // A plugin of Escouade's own hand (`plugins/<id>/` of its repository, ESC-314) takes the ids
    // `dev.escouade.*`; one the app gives (INT-4), its programs too.
    let hand = if flag("--builtin") {
        manifest::Hand::Builtin
    } else if flag("--first-party") {
        manifest::Hand::First
    } else {
        manifest::Hand::Third
    };
    match command {
        "validate" => validate(&folder, lang, hand, &app_version),
        "build" => {
            let out = args
                .iter()
                .position(|a| a == "--out")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from);
            build(&folder, lang, hand, out, flag("--json"))
        }
        "fetch" => fetch(&folder, lang, hand),
        "test" | "dev" => match more {
            Some(more) => more(&Call {
                command,
                folder: &folder,
                lang,
                hand,
                app_version: &app_version,
                args: &args,
            }),
            None => {
                eprintln!(
                    "{}",
                    tr_in!(
                        lang,
                        "`{command}` lance le plugin : c'est l'`escouade-plugin` livré avec l'app qui le fait",
                        "`{command}` starts the plugin: the `escouade-plugin` shipped with the app does it"
                    )
                );
                2
            }
        },
        _ => {
            eprint!("{}", usage(lang));
            2
        }
    }
}

/// The manifest of `folder`, or what is wrong with it, said on the standard error.
pub fn read_manifest(folder: &Path, lang: Lang, hand: manifest::Hand) -> Option<Manifest> {
    let path = folder.join(manifest::FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return None;
        }
    };
    let read = manifest::read_in(&text, hand, folder);
    for f in &read.ignored {
        eprintln!(
            "{}",
            tr_in!(lang, "champ ignoré : {f}", "ignored field: {f}")
        );
    }
    match read.manifest {
        Ok(m) => Some(m),
        Err(problems) => {
            for p in problems {
                eprintln!("✗ {}", p.message(lang));
            }
            None
        }
    }
}

fn validate(folder: &Path, lang: Lang, hand: manifest::Hand, app_version: &str) -> i32 {
    let Some(m) = read_manifest(folder, lang, hand) else {
        return 1;
    };
    if let Some(why) = m.refusal(app_version, lang) {
        eprintln!("✗ {why}");
        return 1;
    }
    // A `run.js` plugin: its bundle is one that can run in WASM (what the store's compilation refuses, said first).
    if let Some(js) = m.run.as_ref().and_then(|r| r.js.as_ref()) {
        if let Err(why) = bundle_of(folder, js, lang) {
            eprintln!("✗ {why}");
            return 1;
        }
    }
    // A `run.wasm` plugin: what `build` would refuse of its sources without running anything, and its module, when it
    // is there, has the form the app runs.
    if let Some(cargo_file) = m.run.as_ref().and_then(|r| r.wasm.as_ref()) {
        if let Err(why) = rust::check(folder, cargo_file) {
            eprintln!("✗ {}", why.message(lang));
            return 1;
        }
        let path = folder.join(wasm::MODULE_FILE);
        if let Ok(bytes) = std::fs::read(&path) {
            if let Err(why) = wasm::reactor_form(&bytes) {
                eprintln!("✗ {}: {why}", path.display());
                return 1;
            }
        }
    }
    println!(
        "{}",
        tr_in!(
            lang,
            "✓ {} {} : le manifeste est valide",
            "✓ {} {}: the manifest holds",
            m.id,
            m.version
        )
    );
    0
}

/// The bundle `js` of the plugin in `folder`, when it can run in WASM; why not, in words.
fn bundle_of(folder: &Path, js: &str, lang: Lang) -> Result<String, String> {
    let path = folder.join(js);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    wasm::script_of(&text).map_err(|r| r.message(lang))?;
    Ok(text)
}

/// The module of the `run.js` plugin of `m`, and how it was made.
fn module_of_bundle(
    folder: &Path,
    js: &str,
    lang: Lang,
) -> Result<(Vec<u8>, serde_json::Value), String> {
    let bundle = bundle_of(folder, js, lang)?;
    let compiler = wasm::Compiler::of_data(&crate::paths::tools_data_dir());
    let bytes = compiler.build(&bundle).map_err(|e| {
        tr_in!(
            lang,
            "{js} ne se compile pas en WASM : {e}",
            "{js} does not compile to WASM: {e}"
        )
    })?;
    let made = json!({
        "javy": wasm::JAVY_VERSION,
        "engine": wasm::escouade_js_sha256(),
        "prelude": env!("CARGO_PKG_VERSION"),
    });
    Ok((bytes, made))
}

/// What the toolchain's first download says, as it takes minutes.
pub fn say_download(lang: Lang) {
    eprintln!(
        "{}",
        tr_in!(
            lang,
            "téléchargement de rustc {} pour {} (quelques centaines de Mo, une seule fois)…",
            "downloading rustc {} for {} (a few hundred MB, once)…",
            rust::VERSION,
            rust::TARGET
        )
    );
}

/// The module of the `run.wasm` plugin of `folder`, with this program as `rustc-canon`; why not, in words.
pub fn module_of_crate(
    folder: &Path,
    cargo_file: &str,
    data: &Path,
    lang: Lang,
) -> Result<rust::Built, String> {
    let exe = this_program().map_err(|e| e.to_string())?;
    rust::build(folder, cargo_file, data, &exe, &|| say_download(lang)).map_err(|f| f.message(lang))
}

/// This program, which serves as `rustc-canon`. The store's sandbox has no `/proc` (ESC-336), where Linux says it: its
/// path is then the absolute one it was started by.
fn this_program() -> std::io::Result<PathBuf> {
    std::env::current_exe().or_else(|e| {
        std::env::args_os()
            .next()
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or(e)
    })
}

/// `fetch`: the dependencies of a `run.wasm` plugin, in the data folder. A line of ✗ is the plugin's fault; without one,
/// the store takes it for its own (a network that fails) and the version waits to be checked again.
fn fetch(folder: &Path, lang: Lang, hand: manifest::Hand) -> i32 {
    let Some(m) = read_manifest(folder, lang, hand) else {
        return 1;
    };
    let Some(cargo_file) = m.run.as_ref().and_then(|r| r.wasm.as_ref()) else {
        eprintln!(
            "✗ {}",
            tr_in!(
                lang,
                "seul un plugin `run.wasm` a des dépendances à télécharger",
                "only a `run.wasm` plugin has dependencies to download"
            )
        );
        return 1;
    };
    let exe = match this_program() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let data = crate::paths::tools_data_dir();
    match rust::fetch(folder, cargo_file, &data, &exe, &|| say_download(lang)) {
        Ok(()) => 0,
        Err(rust::Failure::Refused(why)) => {
            eprintln!("✗ {}", why.message(lang));
            1
        }
        Err(rust::Failure::Tools(why)) => {
            eprintln!("{why}");
            1
        }
    }
}

/// `build`: the module of a `run.js` or `run.wasm` plugin, written with its SHA-256 (`<sha256>  <name>`, as `pack` writes
/// its own).
fn build(folder: &Path, lang: Lang, hand: manifest::Hand, out: Option<PathBuf>, json: bool) -> i32 {
    let Some(m) = read_manifest(folder, lang, hand) else {
        return 1;
    };
    let run = m.run.as_ref();
    let made = if let Some(js) = run.and_then(|r| r.js.as_ref()) {
        module_of_bundle(folder, js, lang)
    } else if let Some(cargo_file) = run.and_then(|r| r.wasm.as_ref()) {
        module_of_crate(folder, cargo_file, &crate::paths::tools_data_dir(), lang)
            .map(|b| (b.bytes, rust::compiler(&b.lock)))
    } else {
        Err(tr_in!(
            lang,
            "seul un plugin `run.js` ou `run.wasm` se compile en WASM",
            "only a `run.js` or `run.wasm` plugin compiles to WASM"
        ))
    };
    let (bytes, made) = match made {
        Ok(m) => m,
        Err(why) => {
            eprintln!("✗ {why}");
            return 1;
        }
    };
    let sha256 = wasm::sha256(&bytes);
    let out = out.unwrap_or_else(|| folder.join("packs"));
    let name = format!("{}-{}.wasm", m.id, m.version);
    let path = out.join(&name);
    let written = std::fs::create_dir_all(&out)
        .and_then(|()| std::fs::write(&path, &bytes))
        .and_then(|()| {
            std::fs::write(
                out.join(format!("{name}.sha256")),
                format!("{sha256}  {name}\n"),
            )
        });
    if let Err(e) = written {
        eprintln!("{}: {e}", path.display());
        return 1;
    }
    if json {
        let report = json!({
            "file": path,
            "size": bytes.len(),
            "sha256": sha256,
            "compiler": made,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        println!("{}", path.display());
        println!("{}", tr_in!(lang, "  {} octets", "  {} bytes", bytes.len()));
        println!("  sha256 {sha256}");
    }
    0
}
