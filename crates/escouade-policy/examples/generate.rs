//! Writes the files generated from the single list of the policies (`escouade_policy::generate::files`): the ADMX and
//! its ADML, the macOS manifests and example profile, the schema of /etc and its example, the page of the site.
//!
//! `cargo run -p escouade-policy --example generate`, from anywhere in the repository; the test
//! `the_files_of_the_repository_are_what_the_list_generates` fails until it is run after a change of the list.

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (path, content) in escouade_policy::generate::files() {
        let file = root.join(path);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let before = std::fs::read_to_string(&file).unwrap_or_default();
        if before.replace("\r\n", "\n") != content {
            std::fs::write(&file, content)?;
            println!("written: {path}");
        }
    }
    Ok(())
}
