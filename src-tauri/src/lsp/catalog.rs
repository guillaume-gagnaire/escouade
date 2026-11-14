//! Which language servers Escouade knows, for which languages (ESC-446): for each, the programs that are it, its
//! arguments, where else than the `PATH` it is found, and the command that installs it. Nothing is downloaded: a
//! server not found is said, with that command.

/// A place, out of the `PATH` and the project's `node_modules/.bin`, where a tool puts the server it installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standard {
    /// `rustup which rust-analyzer`: the toolchain's component.
    Rustup,
    /// `~/.cargo/bin`.
    CargoBin,
    /// `$(go env GOPATH)/bin`, `~/go/bin` when `go` says nothing.
    GoBin,
    /// The folder of the scripts of `pip install --user` and `pipx` (`~/.local/bin`; on Windows, also
    /// `%APPDATA%\Python\Scripts`).
    PythonUser,
}

/// A language server of the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Server {
    /// Its id: the key of its settings (`Settings::language_servers`, `language_servers.<id>` of `.escouade.yml`).
    pub id: &'static str,
    /// What the settings show.
    pub name: &'static str,
    /// The ids of the languages it serves, as VS Code names them (`typescriptreact`…).
    pub languages: &'static [&'static str],
    /// The names of its program, the first preferred: each looked for everywhere before the next.
    pub programs: &'static [&'static str],
    pub args: &'static [&'static str],
    /// An npm package: also looked for in the `node_modules/.bin` of the worktree and of the project, which only
    /// runs once the project is trusted (`Trust`).
    pub node: bool,
    /// Where its tool installs it.
    pub standard: &'static [Standard],
    /// The command that installs it, said when it is not found.
    pub install: &'static str,
    /// It runs code of the project, even installed on the machine: rust-analyzer its `build.rs` and its procedural
    /// macros (`cargo check`), typescript-language-server the `tsserver` of the project's `node_modules/typescript`,
    /// gopls the toolchain its `go.mod` names and cgo. Started in a project only once the user trusts it, as VS Code
    /// turns them off out of its Workspace Trust.
    pub runs_project_code: bool,
}

pub const SERVERS: &[Server] = &[
    Server {
        id: "typescript",
        name: "TypeScript, JavaScript (typescript-language-server)",
        languages: &[
            "typescript",
            "typescriptreact",
            "javascript",
            "javascriptreact",
        ],
        programs: &["typescript-language-server"],
        args: &["--stdio"],
        node: true,
        standard: &[],
        install: "npm install -g typescript-language-server typescript",
        runs_project_code: true,
    },
    Server {
        id: "rust",
        name: "Rust (rust-analyzer)",
        languages: &["rust"],
        programs: &["rust-analyzer"],
        args: &[],
        node: false,
        standard: &[Standard::Rustup, Standard::CargoBin],
        install: "rustup component add rust-analyzer",
        runs_project_code: true,
    },
    Server {
        id: "python",
        name: "Python (basedpyright, pyright)",
        languages: &["python"],
        programs: &["basedpyright-langserver", "pyright-langserver"],
        args: &["--stdio"],
        node: true,
        standard: &[Standard::PythonUser],
        install: "pip install basedpyright",
        runs_project_code: false,
    },
    Server {
        id: "go",
        name: "Go (gopls)",
        languages: &["go"],
        programs: &["gopls"],
        args: &[],
        node: false,
        standard: &[Standard::GoBin],
        install: "go install golang.org/x/tools/gopls@latest",
        runs_project_code: true,
    },
    Server {
        id: "json",
        name: "JSON (vscode-json-language-server)",
        languages: &["json", "jsonc"],
        programs: &["vscode-json-language-server"],
        args: &["--stdio"],
        node: true,
        standard: &[],
        install: "npm install -g vscode-langservers-extracted",
        runs_project_code: false,
    },
    Server {
        id: "css",
        name: "CSS, SCSS, Less (vscode-css-language-server)",
        languages: &["css", "scss", "less"],
        programs: &["vscode-css-language-server"],
        args: &["--stdio"],
        node: true,
        standard: &[],
        install: "npm install -g vscode-langservers-extracted",
        runs_project_code: false,
    },
    Server {
        id: "html",
        name: "HTML (vscode-html-language-server)",
        languages: &["html"],
        programs: &["vscode-html-language-server"],
        args: &["--stdio"],
        node: true,
        standard: &[],
        install: "npm install -g vscode-langservers-extracted",
        runs_project_code: false,
    },
];

/// The server of id `id`.
pub fn get(id: &str) -> Option<&'static Server> {
    SERVERS.iter().find(|s| s.id == id)
}

/// The server of the language `language` (an id of VS Code's: `typescriptreact`, `rust`…).
pub fn for_language(language: &str) -> Option<&'static Server> {
    SERVERS.iter().find(|s| s.languages.contains(&language))
}
