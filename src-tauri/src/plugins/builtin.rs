//! The plugins the app gives (INT-4, docs/PLUGINS.md §4.3): the registry of those of Escouade's own hand that are
//! shipped with the app and not installed, their origin `builtin`, and the check that the program they run is the
//! app's own.
//!
//! Each is a process of its own, started by `runtime` like any plugin's, but its manifest is read from the app
//! (`plugins/builtin/<id>/escouade-plugin.json` of the repository, embedded) and its program is `escouade-plugins`
//! beside the app, which the release signs with it. Their version is Escouade's: there is no update of their own, a fix
//! comes with a release, and no revocation list for them. The user may turn them off for the app or for a project, never
//! remove them; a plugin of the store with the same id is refused: the one the app gives wins, and is never replaced by
//! a copy that could be older or not the app's.

use std::path::Path;

/// A plugin the app gives.
pub struct Entry {
    pub id: &'static str,
    /// Its `escouade-plugin.json`, as the repository holds it (its `version` is replaced by the app's).
    manifest: &'static str,
}

/// The registry: the plugins the app gives, by id. Add the folder `plugins/builtin/<id>/` (manifest, README) and one
/// line here; the test of the registry checks that each folder has its manifest and its README, and that each is in
/// this list.
///
/// The demonstration plugin is in the builds of development and the tests only: a release's registry holds the real ones
/// (the five voice services, INT-13; isola, INT-14; GitHub, INT-8.1, its code host and GitHub Issues; the others come
/// with INT-7 and INT-8), so that nobody sees (nor runs) a plugin that does nothing. `escouade-plugins` itself is
/// packaged and signed all the same.
pub const REGISTRY: &[Entry] = &[
    #[cfg(any(debug_assertions, test))]
    Entry {
        id: "dev.escouade.minimal",
        manifest: include_str!(
            "../../../plugins/builtin/dev.escouade.minimal/escouade-plugin.json"
        ),
    },
    Entry {
        id: "dev.escouade.openai",
        manifest: include_str!("../../../plugins/builtin/dev.escouade.openai/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.groq",
        manifest: include_str!("../../../plugins/builtin/dev.escouade.groq/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.mistral",
        manifest: include_str!(
            "../../../plugins/builtin/dev.escouade.mistral/escouade-plugin.json"
        ),
    },
    Entry {
        id: "dev.escouade.elevenlabs",
        manifest: include_str!(
            "../../../plugins/builtin/dev.escouade.elevenlabs/escouade-plugin.json"
        ),
    },
    Entry {
        id: "dev.escouade.deepgram",
        manifest: include_str!(
            "../../../plugins/builtin/dev.escouade.deepgram/escouade-plugin.json"
        ),
    },
    Entry {
        id: "dev.escouade.isola",
        manifest: include_str!("../../../plugins/builtin/dev.escouade.isola/escouade-plugin.json"),
    },
    Entry {
        id: GITHUB,
        manifest: include_str!("../../../plugins/builtin/dev.escouade.github/escouade-plugin.json"),
    },
];

/// The plugin « GitHub »: github.com, GHE.com and GitHub Enterprise Server, their pull requests and GitHub Issues.
pub const GITHUB: &str = "dev.escouade.github";

/// Whether `id` is the id of a plugin the app gives.
pub fn is_builtin(id: &str) -> bool {
    REGISTRY.iter().any(|e| e.id == id)
}

/// The manifest of the builtin plugin `id`, at Escouade's version `app_version`.
pub fn manifest_text(id: &str, app_version: &str) -> Option<String> {
    let entry = REGISTRY.iter().find(|e| e.id == id)?;
    let mut v: serde_json::Value = serde_json::from_str(entry.manifest).ok()?;
    v["version"] = app_version.into();
    serde_json::to_string_pretty(&v).ok()
}

/// What pins a builtin plugin: the app's version.
pub fn fingerprint(app_version: &str) -> String {
    format!("builtin:{app_version}")
}

/// The name of the program of the plugins the app gives, beside the app.
pub fn program_name() -> &'static str {
    if cfg!(windows) {
        "escouade-plugins.exe"
    } else {
        "escouade-plugins"
    }
}

/// Whether `program` is the app's own `escouade-plugins`: that very file, in the folder of the app (`app_dir`, links
/// followed), and where the system checks signatures, a signature as good as the app's. Else why not.
pub fn check_program(program: &Path, app_dir: &Path) -> Result<(), String> {
    // Windows starts `escouade-plugins` for `escouade-plugins.exe`: the manifest names it without.
    let program = if cfg!(windows) && program.extension().is_none() {
        program.with_extension("exe")
    } else {
        program.to_path_buf()
    };
    let real = std::fs::canonicalize(&program)
        .map_err(|e| format!("{} cannot be read: {e}", program.display()))?;
    let dir = std::fs::canonicalize(app_dir)
        .map_err(|e| format!("{} cannot be read: {e}", app_dir.display()))?;
    let name_holds = real.file_name().and_then(|n| n.to_str()) == Some(program_name());
    if !name_holds || real.parent() != Some(dir.as_path()) {
        return Err(format!(
            "{} is not the {} of the app's folder ({})",
            program.display(),
            program_name(),
            dir.display()
        ));
    }
    signature::holds(&real)
}

/// The signature of the app's programs, where the system checks it. The rule is the release's: a build of development
/// is not signed.
mod signature {
    use std::path::Path;

    #[cfg(any(debug_assertions, not(any(windows, target_os = "macos"))))]
    pub fn holds(_: &Path) -> Result<(), String> {
        Ok(())
    }

    /// macOS: the code signature of the file holds (an ad hoc one too: the release signs with the Developer ID when
    /// its secrets are set, and notarizes it).
    #[cfg(all(not(debug_assertions), target_os = "macos"))]
    pub fn holds(program: &Path) -> Result<(), String> {
        let out = std::process::Command::new("/usr/bin/codesign")
            .args(["--verify", "--strict"])
            .arg(program)
            .output()
            .map_err(|e| format!("codesign cannot run: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "its code signature does not hold: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    /// Windows: when the app itself carries a valid Authenticode signature (the release signs with Azure Artifact
    /// Signing when its secrets are set), the program must carry a valid one too. Fails closed: a status that cannot
    /// be read is no signature.
    #[cfg(all(not(debug_assertions), windows))]
    pub fn holds(program: &Path) -> Result<(), String> {
        let app = std::env::current_exe().map_err(|e| e.to_string())?;
        if authenticode_status(&app)? != "Valid" {
            // The app is not signed (a build without the release's secrets): nothing to hold the program to.
            return Ok(());
        }
        match authenticode_status(program)?.as_str() {
            "Valid" => Ok(()),
            other => Err(format!(
                "its Authenticode signature is {other}, the app's is Valid"
            )),
        }
    }

    /// What the system says of a file's Authenticode signature (`Valid`, `NotSigned`, `HashMismatch`…), as
    /// `Get-AuthenticodeSignature` names it. The path goes by the environment, never into the script. Kept for the
    /// session by the file (path, size, modification date): each start of a plugin the app gives would else run
    /// PowerShell again, for the app and for the program.
    #[cfg(windows)]
    // Read by the release's rule and by the tests; a build of development has no use of it.
    #[cfg_attr(debug_assertions, allow(dead_code))]
    pub fn authenticode_status(path: &Path) -> Result<String, String> {
        use std::collections::HashMap;
        use std::os::windows::process::CommandExt;
        use std::sync::{LazyLock, Mutex};
        use std::time::SystemTime;
        type Key = (std::path::PathBuf, u64, Option<SystemTime>);
        static KEPT: LazyLock<Mutex<HashMap<Key, String>>> = LazyLock::new(Default::default);
        let meta = std::fs::metadata(path)
            .map_err(|e| format!("{} cannot be read: {e}", path.display()))?;
        let key: Key = (path.to_path_buf(), meta.len(), meta.modified().ok());
        if let Some(kept) = KEPT.lock().map_err(|e| e.to_string())?.get(&key) {
            return Ok(kept.clone());
        }
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg("(Get-AuthenticodeSignature -LiteralPath $env:ESCOUADE_SIGNED_PATH).Status")
            .env("ESCOUADE_SIGNED_PATH", path)
            .creation_flags(crate::proc::CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("the signature cannot be read: {e}"))?;
        let status = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() || status.is_empty() {
            return Err(format!(
                "the signature of {} cannot be read: {}",
                path.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        KEPT.lock()
            .map_err(|e| e.to_string())?
            .insert(key, status.clone());
        Ok(status)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::signature::authenticode_status;
    use std::path::Path;

    /// The reading itself (compiled in every profile; the rule that uses it is the release's).
    #[test]
    fn the_system_s_own_program_is_signed_and_an_unsigned_copy_is_not() {
        let notepad = Path::new(r"C:\Windows\System32\notepad.exe");
        assert_eq!(authenticode_status(notepad).unwrap(), "Valid");
        let dir =
            std::env::temp_dir().join(format!("authenticode-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let unsigned = dir.join("tool.exe");
        std::fs::copy(std::env::current_exe().unwrap(), &unsigned).unwrap();
        let status = authenticode_status(&unsigned).unwrap();
        assert_ne!(status, "Valid");
        // Kept: the second reading is the first's.
        assert_eq!(authenticode_status(&unsigned).unwrap(), status);
        // A file that is not there is no status.
        assert!(authenticode_status(&dir.join("nope.exe")).is_err());
    }
}

/// The runtime of the plugins the app gives, as a test has it: their programs those the build made beside the tests
/// (`target/debug/escouade-plugins`), its keychain empty. A test lends the tokens (`Runtime::lend`).
#[cfg(test)]
pub fn test_runtime(name: &str) -> std::sync::Arc<super::runtime::Runtime> {
    let data =
        crate::paths::DataDir::new(crate::paths::test_dir(&format!("builtin-{name}")).join("data"));
    data.ensure().unwrap();
    let plugins = super::Plugins::load_for_core(data.clone(), env!("CARGO_PKG_VERSION"));
    super::runtime::Runtime::new(
        plugins,
        std::sync::Arc::new(crate::integrations::secrets::MemoryStore::default()),
        &data,
        env!("CARGO_PKG_VERSION"),
        super::runtime::Limits::default(),
    )
}

/// A client of the code host `kind` for the account `creds`, served by the plugin the app gives for it (`plugin`), its
/// token lent: what the core does for an account of the first hand.
#[cfg(test)]
pub fn served_forge(
    plugin: &str,
    kind: &str,
    creds: &crate::forge::Credentials,
) -> Box<dyn crate::forge::Forge> {
    let rt = test_runtime(kind);
    rt.lend(
        plugin,
        &format!("account:{}", crate::forge::host_of(&creds.url)),
        &creds.token,
    );
    Box::new(super::adapters::ProcessForge::new(rt, plugin, kind, creds))
}
