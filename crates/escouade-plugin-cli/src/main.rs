//! `escouade-plugin` without the app: what the store's image runs, under that name (store/Dockerfile).
//! `validate`, `build` and `--version`, the same as the app's; `test` and `dev` need the app's host, and say so.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_plugin_cli::cli::main(args, None));
}
