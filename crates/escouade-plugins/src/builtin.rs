//! The registry of the plugins of Escouade's own hand (docs/PLUGINS-DESIGN.md §8.7): the one list of the ids of the app's seed
//! (`seed`), with their manifests as the repository holds them (`plugins/<id-court>/escouade-plugin.json`, embedded),
//! which name a plugin of the seed that is not installed, and say what a form of its accounts lacks.
//!
//! No plugin runs from here in a release: those of the first hand are installed from the seed or the store, plugins like
//! any other, which turn off and uninstall (ESC-355 took the origin `builtin` away, its `{escouade}/…`, its program
//! `escouade-plugins` and its check). A build of development and the tests run the registry's modules from the workspace
//! (`Origin::Workspace`, `folder`) where the seed has not its files.

use std::path::{Path, PathBuf};

/// A plugin of the registry.
pub struct Entry {
    pub id: &'static str,
    /// Its `escouade-plugin.json`, as the repository holds it (its `version` is replaced by the app's).
    manifest: &'static str,
}

/// The registry, by id. Add the folder `plugins/<id-court>/` (manifest, README, LICENSE, CHANGELOG) and one line here;
/// the test of the registry checks that each folder has them, and that each is in this list.
///
/// The demonstration plugin is in the builds of development and the tests only (a module, `tests/fixtures/plugins/
/// minimal/`), so that nobody sees (nor runs) a plugin that does nothing.
pub const REGISTRY: &[Entry] = &[
    #[cfg(any(debug_assertions, test))]
    Entry {
        id: MINIMAL,
        manifest: include_str!("../../../tests/fixtures/plugins/minimal/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.openai",
        manifest: include_str!("../../../plugins/openai/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.groq",
        manifest: include_str!("../../../plugins/groq/escouade-plugin.json"),
    },
    Entry {
        id: MISTRAL,
        manifest: include_str!("../../../plugins/mistral/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.elevenlabs",
        manifest: include_str!("../../../plugins/elevenlabs/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.deepgram",
        manifest: include_str!("../../../plugins/deepgram/escouade-plugin.json"),
    },
    Entry {
        id: "dev.escouade.isola",
        manifest: include_str!("../../../plugins/isola/escouade-plugin.json"),
    },
    Entry {
        id: GITHUB,
        manifest: include_str!("../../../plugins/github/escouade-plugin.json"),
    },
    Entry {
        id: GITLAB,
        manifest: include_str!("../../../plugins/gitlab/escouade-plugin.json"),
    },
    Entry {
        id: BITBUCKET,
        manifest: include_str!("../../../plugins/bitbucket/escouade-plugin.json"),
    },
    Entry {
        id: GITEA,
        manifest: include_str!("../../../plugins/gitea/escouade-plugin.json"),
    },
    Entry {
        id: TRELLO,
        manifest: include_str!("../../../plugins/trello/escouade-plugin.json"),
    },
    Entry {
        id: AZURE_DEVOPS,
        manifest: include_str!("../../../plugins/azure-devops/escouade-plugin.json"),
    },
    Entry {
        id: ACP,
        manifest: include_str!("../../../plugins/acp/escouade-plugin.json"),
    },
    Entry {
        id: LINEAR,
        manifest: include_str!("../../../plugins/linear/escouade-plugin.json"),
    },
    Entry {
        id: JIRA,
        manifest: include_str!("../../../plugins/jira/escouade-plugin.json"),
    },
    Entry {
        id: CURSOR,
        manifest: include_str!("../../../plugins/cursor/escouade-plugin.json"),
    },
    Entry {
        id: CLICKUP,
        manifest: include_str!("../../../plugins/clickup/escouade-plugin.json"),
    },
    Entry {
        id: GEMINI,
        manifest: include_str!("../../../plugins/gemini/escouade-plugin.json"),
    },
    Entry {
        id: OPENCODE,
        manifest: include_str!("../../../plugins/opencode/escouade-plugin.json"),
    },
    Entry {
        id: COPILOT,
        manifest: include_str!("../../../plugins/copilot/escouade-plugin.json"),
    },
    Entry {
        id: CODEX,
        manifest: include_str!("../../../plugins/codex/escouade-plugin.json"),
    },
    Entry {
        id: ANTIGRAVITY,
        manifest: include_str!("../../../plugins/antigravity/escouade-plugin.json"),
    },
    // Claude Code (INT-10.5): its agents' conversation, by the client of `claude` in its crate.
    Entry {
        id: CLAUDE,
        manifest: include_str!("../../../plugins/claude/escouade-plugin.json"),
    },
];

/// The demonstration plugin, of the builds of development and the tests alone.
#[cfg(any(debug_assertions, test))]
pub const MINIMAL: &str = "dev.escouade.minimal";

/// The plugin « GitHub »: github.com, GHE.com and GitHub Enterprise Server, their pull requests and GitHub Issues.
pub const GITHUB: &str = "dev.escouade.github";

/// The plugin « GitLab »: gitlab.com and the self-managed instances, their merge requests (ESC-56); GitLab Issues is
/// still the core's, on the same accounts, until INT-8.2.
pub const GITLAB: &str = "dev.escouade.gitlab";

/// The plugin « Bitbucket »: Bitbucket Cloud, its pull requests, pipelines and deployments.
pub const BITBUCKET: &str = "dev.escouade.bitbucket";

/// The plugin « Gitea / Forgejo »: Gitea, Forgejo and Codeberg, their pull requests, statuses and Actions.
pub const GITEA: &str = "dev.escouade.gitea";

/// The plugin « Trello »: the Kanban's source of Trello cards (INT-7.1).
pub const TRELLO: &str = "dev.escouade.trello";

/// The plugin « Codex »: OpenAI's Codex as a harness (INT-10.4).
pub const CODEX: &str = "dev.escouade.codex";
/// The plugin Claude Code.
pub const CLAUDE: &str = "dev.escouade.claude-code";

/// The plugin « Antigravity CLI »: Google's agent in a terminal as a harness (INT-10.8).
pub const ANTIGRAVITY: &str = "dev.escouade.antigravity-cli";

/// The plugin « Azure DevOps »: Azure DevOps (dev.azure.com and Server), its pull requests, checks and Azure Boards.
pub const AZURE_DEVOPS: &str = "dev.escouade.azure-devops";

/// The generic ACP plugin, which serves the harnesses a manifest describes (`contributes.harnesses[].acp`, PLG-6).
pub const ACP: &str = "dev.escouade.acp";

/// The plugin « Linear »: the Kanban's source of Linear issues (INT-7.2).
pub const LINEAR: &str = "dev.escouade.linear";

/// The plugin « Jira »: the Kanban's source of Jira Cloud issues (INT-7.3).
pub const JIRA: &str = "dev.escouade.jira";

/// The plugin « Cursor CLI »: Cursor CLI as a harness, over ACP (INT-10.1).
pub const CURSOR: &str = "dev.escouade.cursor-cli";

/// The plugin « Copilot CLI »: its harness in process, on the client of its own server (INT-10.3).
pub const COPILOT: &str = "dev.escouade.copilot-cli";

/// The plugin « ClickUp »: the Kanban's source of ClickUp tasks (INT-7.4).
pub const CLICKUP: &str = "dev.escouade.clickup";

/// The plugin « Mistral », one process and one account: its voice service (INT-13) and Mistral Vibe as a harness, over
/// ACP (INT-10.6 bis).
pub const MISTRAL: &str = "dev.escouade.mistral";

/// The plugin « Gemini CLI »: Gemini CLI as a harness, over ACP (INT-10.7).
pub const GEMINI: &str = "dev.escouade.gemini-cli";

/// The plugin « OpenCode »: OpenCode as a harness, on its `opencode serve` (INT-10.2).
pub const OPENCODE: &str = "dev.escouade.opencode";

/// The plugins of the harnesses that left the core, in the order of the app's `harness::BUILTIN_HARNESSES` (a test of the app
/// keeps them the same): a plugin that describes harnesses over ACP is none of these.
pub const HARNESSES: &[&str] = &[
    CURSOR,
    MISTRAL,
    GEMINI,
    OPENCODE,
    COPILOT,
    CODEX,
    ANTIGRAVITY,
    CLAUDE,
];

/// Whether the seed may hold `id`: a plugin of the registry, less the demonstration, which is no product; and the plugins
/// of the first hand the app never gave, published on the store (ESC-354): Goose, a harness its manifest describes, and
/// those on request (Discord, Slack, High contrast, Team rules), which the seed proposes and never installs unasked.
pub fn seeds(id: &str) -> bool {
    (id != "dev.escouade.minimal" && is_builtin(id)) || super::sort::seeds_beyond_the_registry(id)
}

/// Whether `id` is the id of a plugin of the registry.
pub fn is_builtin(id: &str) -> bool {
    REGISTRY.iter().any(|e| e.id == id)
}

/// Whether `id` is of Escouade's own hand: `dev.escouade.*`, an id the manifest refuses to a third party and that installs
/// signed alone, which holds for those the app does not give (Goose). The one test of it at run time: a plugin holds that
/// id only when it came from the seed or signed from the store (`super::hand_of`, audit SEC-27); a folder in
/// development, copied or cloned, is refused it.
pub fn first_hand(id: &str) -> bool {
    id.starts_with(super::manifest::RESERVED_PREFIX)
}

/// The name of the plugin of the registry `id` in `lang`, as its manifest says it: what the view and the line of a
/// gesture name a plugin of the seed that is not installed (ESC-373).
pub fn name(id: &str, lang: crate::i18n::Lang) -> Option<String> {
    let entry = REGISTRY.iter().find(|e| e.id == id)?;
    let v: serde_json::Value = serde_json::from_str(entry.manifest).ok()?;
    let name = v.get("name")?;
    let key = match lang {
        crate::i18n::Lang::Fr => "fr",
        crate::i18n::Lang::En => "en",
    };
    name.as_str()
        .or_else(|| name.get(key).and_then(|n| n.as_str()))
        .or_else(|| name.get("en").and_then(|n| n.as_str()))
        .map(str::to_string)
}

/// The manifest of the plugin of the registry `id`, at Escouade's version `app_version`: what the workspace of a build of
/// development runs (`Origin::Workspace`).
pub fn manifest_text(id: &str, app_version: &str) -> Option<String> {
    let entry = REGISTRY.iter().find(|e| e.id == id)?;
    let mut v: serde_json::Value = serde_json::from_str(entry.manifest).ok()?;
    v["version"] = app_version.into();
    // Run by this build, it runs on it, whatever a port wrote for the store (`escouade.min`). A version of Escouade
    // there is numbers alone: a pre-release of the app (2.2.0-beta.1) writes its own (2.2.0), or none would be read.
    let numbers = app_version.split(['-', '+']).next().unwrap_or(app_version);
    if let Some(range) = v.get_mut("escouade").and_then(|e| e.as_object_mut()) {
        range.insert("min".to_string(), numbers.into());
    }
    serde_json::to_string_pretty(&v).ok()
}

/// The folder a build of development or a test runs the plugin of the registry `id` from (`Origin::Workspace`): the one
/// `scripts/plugins-wasm.sh` left in `<target>/plugins-wasm/<id-court>/`, beside the app's folder (`app_dir`).
pub fn folder(id: &str, app_dir: &Path) -> PathBuf {
    // The demonstration's module is committed beside its manifest (`sh build.sh`).
    #[cfg(any(debug_assertions, test))]
    if id == MINIMAL {
        return Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/plugins/minimal");
    }
    let base = app_dir.parent().unwrap_or(app_dir).join("plugins-wasm");
    match module_name(id) {
        Some(name) => base.join(name),
        None => base.join(id),
    }
}

/// The folder of `plugins/` of a plugin of the registry that is a module: the last word of its `repository`.
fn module_name(id: &str) -> Option<String> {
    let entry = REGISTRY.iter().find(|e| e.id == id)?;
    let v: serde_json::Value = serde_json::from_str(entry.manifest).ok()?;
    v["run"]["wasm"].as_str()?;
    let repository = v["repository"].as_str()?;
    repository.rsplit('/').next().map(str::to_string)
}

/// What a form of an account of the plugin `plugin` lacks, in a sentence: the first of the fields of its kind of account
/// (`kind`, in `contributes.accounts`) that is empty and has a `missing` text, in the language `lang`. `value` says
/// what the form gave for a field. The texts are the plugin's own (its manifest), so that the core writes none of them.
pub fn missing(
    plugin: &str,
    kind: &str,
    value: impl Fn(&str) -> String,
    lang: crate::i18n::Lang,
) -> Option<String> {
    let entry = REGISTRY.iter().find(|e| e.id == plugin)?;
    let v: serde_json::Value = serde_json::from_str(entry.manifest).ok()?;
    let account = v["contributes"]["accounts"]
        .as_array()?
        .iter()
        .find(|a| a["id"] == kind)?;
    let code = match lang {
        crate::i18n::Lang::Fr => "fr",
        crate::i18n::Lang::En => "en",
    };
    account["fields"].as_array()?.iter().find_map(|f| {
        let key = f["key"].as_str()?;
        if !value(key).trim().is_empty() {
            return None;
        }
        f["missing"][code].as_str().map(str::to_string)
    })
}

/// The runtime of the plugins of the workspace, as a test has it: their modules those the build made beside the tests
/// (`target/plugins-wasm`), its keychain empty. A test lends the tokens
/// (`Runtime::lend`), and says where a module's fake service is (`let_reach`).
#[cfg(any(test, feature = "testing"))]
pub fn test_runtime(name: &str) -> std::sync::Arc<super::runtime::Runtime> {
    let data =
        crate::paths::DataDir::new(crate::paths::test_dir(&format!("builtin-{name}")).join("data"));
    data.ensure().unwrap();
    let plugins = super::Plugins::load_for_core(data.clone(), env!("ESCOUADE_APP_VERSION"));
    super::runtime::Runtime::new(
        plugins,
        std::sync::Arc::new(crate::secrets::MemoryStore::default()),
        &data,
        env!("ESCOUADE_APP_VERSION"),
        super::runtime::Limits::default(),
    )
}

/// What a test lets the modules of `rt` reach beside their hosts: its fake services by their own address, as `account.api`
/// names them, and the hosts they stand in for.
#[cfg(any(test, feature = "testing"))]
pub fn let_reach(rt: &super::runtime::Runtime, routes: &[(&str, &str)]) {
    rt.set_routes(
        routes
            .iter()
            .map(|(host, base)| (host.to_string(), base.to_string()))
            .collect(),
    );
}
