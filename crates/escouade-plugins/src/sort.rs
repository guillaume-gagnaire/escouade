//! The tables of the sort of the seed (the app's `plugins::sort` reads the configuration with them) (ESC-373, docs/PLUGINS-DESIGN.md §8.7): of the plugins of the first hand the app ships, those the user's
//! configuration uses, read in their settings, their projects, their agents and their accounts, never on the network. At
//! the start, `Plugins::sync_seed` installs these and nothing else; the others stay in the seed, proposed by the view
//! Plugins, and install with no network on the gesture that needs one (`Core::need_plugin`).
//!
//! The rule never makes a function of 2.1 lost: what does not read surely (a settings file that does not parse, a
//! project's folder or git configuration out of reach) wants every plugin it could have needed, and an id of the seed the
//! rule does not know is installed.

use super::builtin;

/// The plugin « Goose », a harness its manifest describes, served by `dev.escouade.acp`.
pub const GOOSE: &str = "dev.escouade.goose";
/// The plugin « isola », the launch environment of a worktree with an `.isola.toml`.
pub const ISOLA: &str = "dev.escouade.isola";

/// The plugins that install at the start only when they are installed already: their use is the plugin itself.
pub const ON_REQUEST: &[&str] = &[
    "dev.escouade.discord",
    "dev.escouade.slack",
    "dev.escouade.high-contrast",
    "dev.escouade.team-rules",
];

/// Whether `id` is a plugin of the first hand the seed holds though the registry has it not (`builtin::seeds`): Goose and
/// those on request.
pub fn seeds_beyond_the_registry(id: &str) -> bool {
    id == GOOSE || ON_REQUEST.contains(&id)
}

/// The name of a plugin on request in `lang`, for the view while it is not installed: they are not in the registry, whose
/// manifests name the others (`builtin::name`).
pub fn name_on_request(id: &str, lang: crate::i18n::Lang) -> Option<&'static str> {
    let fr = matches!(lang, crate::i18n::Lang::Fr);
    Some(match id {
        GOOSE => "Goose",
        "dev.escouade.discord" => "Discord",
        "dev.escouade.slack" => "Slack",
        "dev.escouade.high-contrast" if fr => "Contraste élevé",
        "dev.escouade.high-contrast" => "High contrast",
        "dev.escouade.team-rules" if fr => "Règles de l'équipe",
        "dev.escouade.team-rules" => "Team rules",
        _ => return None,
    })
}

/// The voice services of the first hand, by their id in the settings: their plugin is `dev.escouade.<id>`.
pub const VOICES: &[&str] = &["openai", "groq", "mistral", "elevenlabs", "deepgram"];

/// The code hosts of the first hand, by their kind (`forge::Provider::kind`), and their plugin.
pub const FORGES: &[(&str, &str)] = &[
    ("github", builtin::GITHUB),
    ("gitlab", builtin::GITLAB),
    ("bitbucket", builtin::BITBUCKET),
    ("gitea", builtin::GITEA),
    ("azure", builtin::AZURE_DEVOPS),
];

/// The plugins of the harness `engine` (an `EngineId`'s text): its own, and `dev.escouade.acp` for a harness a manifest
/// describes (Goose, a third party's). None for an empty id.
pub fn of_engine(engine: &str) -> Vec<&'static str> {
    let own = match engine {
        "" => return Vec::new(),
        "claude" => builtin::CLAUDE,
        "codex" => builtin::CODEX,
        "copilot" => builtin::COPILOT,
        "cursor" => builtin::CURSOR,
        "opencode" => builtin::OPENCODE,
        "vibe" => builtin::MISTRAL,
        "agy" => builtin::ANTIGRAVITY,
        "gemini" => builtin::GEMINI,
        "goose" => return vec![GOOSE, builtin::ACP],
        _ => return vec![builtin::ACP],
    };
    vec![own]
}

/// The plugin of the source of tickets `service` (a `Service`'s text).
pub fn of_service(service: &str) -> Option<&'static str> {
    Some(match service {
        "jira" => builtin::JIRA,
        "trello" => builtin::TRELLO,
        "github" => builtin::GITHUB,
        "linear" => builtin::LINEAR,
        "clickup" => builtin::CLICKUP,
        "gitlab" => builtin::GITLAB,
        "azure" => builtin::AZURE_DEVOPS,
        _ => return None,
    })
}

/// The plugin of the code host of kind `kind`.
pub fn of_forge(kind: &str) -> Option<&'static str> {
    FORGES.iter().find(|(k, _)| *k == kind).map(|(_, p)| *p)
}

/// The plugin of the voice service `id`.
pub fn of_voice(id: &str) -> Option<String> {
    VOICES.contains(&id).then(|| format!("dev.escouade.{id}"))
}

/// Whether the rule knows `id`: the plugins of harnesses, sources, code hosts, voices, isola and those on request. An
/// id of the seed it does not know installs, as in 2.1.
pub fn known(id: &str) -> bool {
    let harness = [
        builtin::CLAUDE,
        builtin::CODEX,
        builtin::COPILOT,
        builtin::CURSOR,
        builtin::OPENCODE,
        builtin::MISTRAL,
        builtin::ANTIGRAVITY,
        builtin::GEMINI,
        builtin::ACP,
        GOOSE,
    ];
    harness.contains(&id)
        || [
            builtin::JIRA,
            builtin::TRELLO,
            builtin::LINEAR,
            builtin::CLICKUP,
        ]
        .contains(&id)
        || FORGES.iter().any(|(_, p)| *p == id)
        || VOICES
            .iter()
            .any(|v| id.strip_prefix("dev.escouade.") == Some(v))
        || id == ISOLA
        || ON_REQUEST.contains(&id)
}
