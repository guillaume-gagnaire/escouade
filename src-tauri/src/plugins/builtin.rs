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
/// line here; the CI checks that each folder has its manifest and its README, and that each is in this list.
pub const REGISTRY: &[Entry] = &[Entry {
    id: "dev.escouade.minimal",
    manifest: include_str!("../../../plugins/builtin/dev.escouade.minimal/escouade-plugin.json"),
}];

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

/// The signature of the app's programs, where the system checks it. Compiled in release only: a build of development
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
    /// Signing when its secrets are set), the program must carry a valid one too.
    #[cfg(all(not(debug_assertions), windows))]
    pub fn holds(program: &Path) -> Result<(), String> {
        use std::os::windows::process::CommandExt;
        fn status(path: &Path) -> Result<String, String> {
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg("(Get-AuthenticodeSignature -LiteralPath $args[0]).Status")
                .arg(path)
                .creation_flags(crate::proc::CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("the signature cannot be read: {e}"))?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        let app = std::env::current_exe().map_err(|e| e.to_string())?;
        if status(&app)? != "Valid" {
            // The app is not signed (a build without the release's secrets): nothing to hold the program to.
            return Ok(());
        }
        match status(program)?.as_str() {
            "Valid" => Ok(()),
            other => Err(format!(
                "its Authenticode signature is {other}, the app's is Valid"
            )),
        }
    }
}
