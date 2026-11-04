fn main() {
    #[allow(unused_mut)] // only changed on Windows
    let mut attributes = tauri_build::Attributes::new();
    // Embed the Common-Controls v6 manifest in every binary, test harnesses included:
    // without it, test binaries that link the dialog/tray code fail to start on Windows
    // (STATUS_ENTRYPOINT_NOT_FOUND, TaskDialogIndirect).
    #[cfg(windows)]
    {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = std::env::current_dir()
            .expect("cwd")
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rustc-link-arg=/WX");
    }
    tauri_build::try_build(attributes).expect("tauri build script");
}
