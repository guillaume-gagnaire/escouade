//! The single list of the policies (docs/TEAM-SERVER.md §6.3 and §6.4, TS-12): every key a managed device may impose,
//! with its type, its sort and what it locks. The core's table of locks is this list (`locks::KEYS`), and the ADMX,
//! its ADML, the macOS manifests, the example profile, the JSON schema of `/etc` and the page of the site are generated
//! from it ([`crate::generate`]); a test checks the files in the repository are what it generates.

use serde_json::Value;

/// What a key may do, when an administrator imposes it (§6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    /// It turns a function off or narrows a list: applied at once. The worst a pirate gets of it is a denial of service.
    Restrict,
    /// A value (a target branch, a prefix, a server's address): applied and shown.
    Value,
    /// A proxy's address, a file of authorities: waits for each member's consent when it is the team's (TS-13); the
    /// managed device's applies, its MDM having every power on the machine already.
    Sensitive,
    /// A command, a hook, a plugin to install: never imposed by the team; the managed device alone may.
    Executable,
}

/// The shape of a key's value, and how the ranks that set it meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// On or off. For a key that restricts, only `restricted` may be imposed (« Statistiques » off, never on).
    Flag { restricted: Option<bool> },
    /// One of `choices`; for a key that restricts, only `restricted`.
    Choice {
        choices: &'static [&'static str],
        restricted: Option<&'static str>,
    },
    /// One of `choices`, from the one that allows the least to the one that allows the most: a ceiling. Between two
    /// ranks, the lower one holds.
    Ceiling { choices: &'static [&'static str] },
    /// A list of what is allowed: crossed between the ranks that set it. `universe`: what it may name (none: any name,
    /// as the harnesses the plugins bring).
    Bound {
        universe: Option<&'static [&'static str]>,
    },
    /// A list of what is denied or added (file patterns): united between the ranks that set it.
    Union,
    /// A string (an address, a path, a fingerprint).
    Text,
    /// A whole number within `min` and `max`.
    Number { min: u32, max: u32 },
    /// A JSON object, written as a JSON string in the registry.
    Json,
}

/// The type of a key's value, as the ADMX, the profile and the schema write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Type {
    Boolean,
    List,
    String,
    Number,
    Json,
}

impl Form {
    pub fn kind(&self) -> Type {
        match self {
            Form::Flag { .. } => Type::Boolean,
            Form::Choice { .. } | Form::Ceiling { .. } | Form::Text => Type::String,
            Form::Bound { .. } | Form::Union => Type::List,
            Form::Number { .. } => Type::Number,
            Form::Json => Type::Json,
        }
    }

    /// The values it may take, when they are a closed list.
    pub fn choices(&self) -> Option<&'static [&'static str]> {
        match self {
            Form::Choice { choices, .. } | Form::Ceiling { choices } => Some(choices),
            Form::Bound { universe } => *universe,
            _ => None,
        }
    }
}

/// The groups of the policies: an ADMX category, a section of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Permissions,
    Harnesses,
    Plugins,
    Projects,
    Network,
    Privacy,
    Remote,
    Updates,
    Servers,
    Mcp,
    Enrollment,
}

impl Category {
    pub const ALL: [Category; 11] = [
        Category::Permissions,
        Category::Harnesses,
        Category::Plugins,
        Category::Projects,
        Category::Network,
        Category::Privacy,
        Category::Remote,
        Category::Updates,
        Category::Servers,
        Category::Mcp,
        Category::Enrollment,
    ];

    /// Its id in the ADMX.
    pub fn id(self) -> &'static str {
        match self {
            Category::Permissions => "permissions",
            Category::Harnesses => "harnesses",
            Category::Plugins => "plugins",
            Category::Projects => "projects",
            Category::Network => "network",
            Category::Privacy => "privacy",
            Category::Remote => "remote",
            Category::Updates => "updates",
            Category::Servers => "servers",
            Category::Mcp => "mcp",
            Category::Enrollment => "enrollment",
        }
    }

    /// Its name, in French then in English.
    pub fn name(self, lang: Lang) -> &'static str {
        let (fr, en) = match self {
            Category::Permissions => ("Modes de permission", "Permission modes"),
            Category::Harnesses => ("Harness et modèles", "Harnesses and models"),
            Category::Plugins => ("Plugins", "Plugins"),
            Category::Projects => ("Projets", "Projects"),
            Category::Network => ("Proxy et certificats", "Proxy and certificates"),
            Category::Privacy => ("Télémétrie et voix", "Telemetry and voice"),
            Category::Remote => ("Remote Control", "Remote Control"),
            Category::Updates => ("Mises à jour", "Updates"),
            Category::Servers => ("Serveurs", "Servers"),
            Category::Mcp => ("Clients MCP extérieurs", "Outside MCP clients"),
            Category::Enrollment => ("Enrôlement", "Enrollment"),
        };
        lang.pick(fr, en)
    }
}

/// A language of the generated files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    pub fn pick<'a>(self, fr: &'a str, en: &'a str) -> &'a str {
        match self {
            Lang::Fr => fr,
            Lang::En => en,
        }
    }
}

/// What a key says, in one language.
#[derive(Clone, Copy, Debug)]
pub struct Text {
    /// Its name, as the ADMX shows it: phrased as what it imposes (« Interdire le mode Bypass »).
    pub title: &'static str,
    /// What it does, and the values it takes.
    pub help: &'static str,
    /// What it locks on the device: the setting grayed, what is refused.
    pub locks: &'static str,
}

/// A policy of the list.
#[derive(Clone, Copy, Debug)]
pub struct Def {
    /// As the policies name it (`permissions.allowBypass`): the registry value's name, the profile's key, the JSON key.
    pub key: &'static str,
    pub sort: Sort,
    pub form: Form,
    /// The rows of the app's register of settings it locks, or whose choices it bounds (`SETTINGS_ROWS` ids,
    /// src/lib/settings-pages.ts). A key may lock none: what it refuses happens elsewhere (an install, a connection).
    pub rows: &'static [&'static str],
    /// The bound a value of it stays within (`permissions.defaultMode` within `permissions.modes`).
    pub within: Option<&'static str>,
    pub category: Category,
    /// What it holds comes with a later piece (the team server, the spaces of the server mode): read and kept, its lock
    /// computed, nothing of the app to bring within it yet.
    pub later: bool,
    /// A value of it, as JSON: the example profile, the example file of `/etc` and the page show it.
    pub example: &'static str,
    pub fr: Text,
    pub en: Text,
}

impl Def {
    pub fn kind(&self) -> Type {
        self.form.kind()
    }

    pub fn text(&self, lang: Lang) -> &Text {
        match lang {
            Lang::Fr => &self.fr,
            Lang::En => &self.en,
        }
    }

    /// Its id in the ADMX: the key, its dots made underscores.
    pub fn id(&self) -> String {
        self.key.replace('.', "_")
    }

    /// Its example, parsed.
    pub fn example_value(&self) -> Value {
        serde_json::from_str(self.example).expect("the list's examples are JSON")
    }
}

/// Escouade's modes, from the one that allows the least to the one that allows the most (`handover::MODES`).
pub const MODES: &[&str] = &[
    "plan",
    "default",
    "acceptEdits",
    "auto",
    "bypassPermissions",
];
/// The efforts a harness may be given, from the least to the most.
pub const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
/// Where a plugin may come from.
pub const PLUGIN_ORIGINS: &[&str] = &[
    "store-verified",
    "store",
    "team-store",
    "git",
    "folder",
    "dev",
];
/// What Claude outside Escouade may do through its MCP server, from the least to the most.
pub const MCP_CLIENTS: &[&str] = &["denied", "readOnly", "allowed"];

pub const ALLOW_BYPASS: &str = "permissions.allowBypass";
pub const MODES_KEY: &str = "permissions.modes";
pub const DEFAULT_MODE: &str = "permissions.defaultMode";
pub const HARNESSES: &str = "harnesses.allowed";
pub const MODELS: &str = "harnesses.allowedModels";
pub const MAX_EFFORT: &str = "harnesses.maxEffort";
pub const PLUGIN_ORIGINS_KEY: &str = "plugins.allowedOrigins";
pub const PLUGINS_ALLOWED: &str = "plugins.allowed";
pub const PLUGINS_DENIED: &str = "plugins.denied";
pub const VERIFIED_ONLY: &str = "plugins.verifiedOnly";
pub const FORCE_PRIVATE: &str = "projects.forcePrivate";
pub const DENIED_FILES: &str = "projects.deniedFiles";
pub const PROXY_URL: &str = "proxy.url";
pub const PROXY_EXCLUSIONS: &str = "proxy.exclusions";
pub const DENY_INSECURE_TLS: &str = "proxy.denyInsecureTls";
pub const AUTHORITIES: &str = "proxy.authorities";
pub const TELEMETRY: &str = "telemetry.enabled";
pub const VOICE_ONLINE: &str = "voice.onlineAllowed";
pub const DICTATION: &str = "voice.dictation";
pub const REMOTE: &str = "remoteControl.enabled";
pub const CHANNEL: &str = "updates.channel";
pub const AUTO_UPDATE: &str = "updates.automatic";
pub const SERVERS: &str = "servers.allowed";
pub const PERSONAL_SPACE: &str = "servers.personalSpace";
pub const AGGREGATE_VIEW: &str = "servers.aggregateView";
pub const CONTENTLESS: &str = "servers.contentlessNotifications";
pub const MCP_EXTERNAL: &str = "mcp.externalClients";
pub const TEAM_SERVER: &str = "team.server";
pub const TEAM_FINGERPRINT: &str = "team.fingerprint";
pub const ONLY_THIS_SERVER: &str = "team.onlyThisServer";

const fn off() -> Form {
    Form::Flag {
        restricted: Some(false),
    }
}

const fn on() -> Form {
    Form::Flag {
        restricted: Some(true),
    }
}

/// The policies, in the order of the page and of the ADMX.
pub const POLICIES: &[Def] = &[
    // ---------- permissions ----------
    Def {
        key: ALLOW_BYPASS,
        sort: Sort::Restrict,
        form: off(),
        rows: &["allowBypass"],
        within: None,
        category: Category::Permissions,
        later: false,
        example: "false",
        fr: Text {
            title: "Interdire le mode Bypass",
            help: "Le mode où un harness ne demande rien (bypassPermissions) n’est plus permis à aucun agent. Seule la valeur false (0) s’impose.",
            locks: "Réglages › Agents › « Autoriser le mode Bypass », éteint et grisé ; le mode Bypass refusé partout.",
        },
        en: Text {
            title: "Deny the Bypass mode",
            help: "The mode where a harness asks nothing (bypassPermissions) is no longer allowed to any agent. Only the value false (0) is imposed.",
            locks: "Settings › Agents › “Allow Bypass mode”, off and grayed; the Bypass mode refused everywhere.",
        },
    },
    Def {
        key: MODES_KEY,
        sort: Sort::Restrict,
        form: Form::Bound {
            universe: Some(MODES),
        },
        rows: &["agentMode", "mode"],
        within: None,
        category: Category::Permissions,
        later: false,
        example: r#"["plan", "default", "acceptEdits"]"#,
        fr: Text {
            title: "Modes de permission permis",
            help: "La liste des modes que les agents peuvent prendre : plan, default, acceptEdits, auto, bypassPermissions. Un agent dans un autre mode finit son tour, puis passe au mode permis le plus proche.",
            locks: "Les modes non permis grisés dans « Mode par défaut des agents », le mode du Kanban et le mode d’un agent.",
        },
        en: Text {
            title: "Allowed permission modes",
            help: "The list of the modes agents may take: plan, default, acceptEdits, auto, bypassPermissions. An agent in another mode ends its turn, then goes to the nearest allowed mode.",
            locks: "The modes not allowed grayed in “Default mode of agents”, the board’s mode and an agent’s mode.",
        },
    },
    Def {
        key: DEFAULT_MODE,
        sort: Sort::Value,
        form: Form::Choice {
            choices: &["plan", "default", "acceptEdits", "auto"],
            restricted: None,
        },
        rows: &["agentMode"],
        within: Some(MODES_KEY),
        category: Category::Permissions,
        later: false,
        example: r#""default""#,
        fr: Text {
            title: "Mode par défaut des agents",
            help: "Le mode des nouveaux agents : plan, default, acceptEdits ou auto. Il reste dans les modes permis : hors d’eux, c’est le plus proche permis.",
            locks: "Réglages › Agents › « Mode par défaut des agents », grisé.",
        },
        en: Text {
            title: "Default agent mode",
            help: "The mode of new agents: plan, default, acceptEdits or auto. It stays within the allowed modes: outside them, the nearest allowed one.",
            locks: "Settings › Agents › “Default mode of agents”, grayed.",
        },
    },
    // ---------- harnesses ----------
    Def {
        key: HARNESSES,
        sort: Sort::Restrict,
        form: Form::Bound { universe: None },
        rows: &["projectEngine"],
        within: None,
        category: Category::Harnesses,
        later: false,
        example: r#"["claude", "codex"]"#,
        fr: Text {
            title: "Harness permis",
            help: "La liste des harness que les agents peuvent utiliser, par leur id (claude, codex, copilot, cursor, opencode, gemini…). Un agent d’un harness devenu interdit s’arrête à la fin de son tour.",
            locks: "Le harness d’un projet, d’un agent, d’un ticket : un harness non permis est grisé et refusé.",
        },
        en: Text {
            title: "Allowed harnesses",
            help: "The list of the harnesses agents may use, by their id (claude, codex, copilot, cursor, opencode, gemini…). An agent of a harness no longer allowed stops at the end of its turn.",
            locks: "The harness of a project, an agent, a ticket: a harness not allowed is grayed and refused.",
        },
    },
    Def {
        key: MODELS,
        sort: Sort::Restrict,
        form: Form::Json,
        rows: &[],
        within: None,
        category: Category::Harnesses,
        later: false,
        example: r#"{"claude": ["claude-sonnet-5-5", "claude-opus-5-5"]}"#,
        fr: Text {
            title: "Modèles permis par harness",
            help: "Un objet JSON qui donne, pour chaque harness, la liste de ses modèles permis. Un harness absent de l’objet garde tous ses modèles.",
            locks: "Un agent ou un ticket sur un modèle non permis est refusé à son démarrage.",
        },
        en: Text {
            title: "Allowed models by harness",
            help: "A JSON object that gives, for each harness, the list of its allowed models. A harness absent from the object keeps all its models.",
            locks: "An agent or a ticket on a model not allowed is refused at its start.",
        },
    },
    Def {
        key: MAX_EFFORT,
        sort: Sort::Restrict,
        form: Form::Ceiling { choices: EFFORTS },
        rows: &[],
        within: None,
        category: Category::Harnesses,
        later: false,
        example: r#""high""#,
        fr: Text {
            title: "Effort au plus",
            help: "L’effort le plus haut qu’un agent peut recevoir : low, medium, high, xhigh ou max.",
            locks: "L’effort des agents du Kanban ramené à ce plafond ; un effort plus haut refusé.",
        },
        en: Text {
            title: "Maximum effort",
            help: "The highest effort an agent may be given: low, medium, high, xhigh or max.",
            locks: "The effort of the board’s agents brought down to this ceiling; a higher effort refused.",
        },
    },
    // ---------- plugins ----------
    Def {
        key: PLUGIN_ORIGINS_KEY,
        sort: Sort::Restrict,
        form: Form::Bound {
            universe: Some(PLUGIN_ORIGINS),
        },
        rows: &[],
        within: None,
        category: Category::Plugins,
        later: false,
        example: r#"["store-verified"]"#,
        fr: Text {
            title: "Origines des plugins permises",
            help: "D’où un plugin peut s’installer : store-verified (le store, versions vérifiées), store (le store, toute version), team-store (le store de l’équipe), git, folder (un dossier), dev (un dossier en développement).",
            locks: "L’installation d’un plugin d’une autre origine est refusée et dite.",
        },
        en: Text {
            title: "Allowed plugin origins",
            help: "Where a plugin may be installed from: store-verified (the store, verified versions), store (the store, any version), team-store (the team’s store), git, folder (a folder), dev (a folder in development).",
            locks: "Installing a plugin of another origin is refused and said.",
        },
    },
    Def {
        key: PLUGINS_ALLOWED,
        sort: Sort::Restrict,
        form: Form::Bound { universe: None },
        rows: &[],
        within: None,
        category: Category::Plugins,
        later: false,
        example: r#"["dev.escouade.github", "dev.escouade.gitlab"]"#,
        fr: Text {
            title: "Plugins permis",
            help: "La liste des ids de plugins qui peuvent s’installer. Absente, tous le peuvent.",
            locks: "L’installation d’un plugin hors de la liste est refusée et dite.",
        },
        en: Text {
            title: "Allowed plugins",
            help: "The list of the plugin ids that may be installed. Absent, all may.",
            locks: "Installing a plugin outside the list is refused and said.",
        },
    },
    Def {
        key: PLUGINS_DENIED,
        sort: Sort::Restrict,
        form: Form::Union,
        rows: &[],
        within: None,
        category: Category::Plugins,
        later: false,
        example: r#"["com.example.unwanted"]"#,
        fr: Text {
            title: "Plugins interdits",
            help: "La liste des ids de plugins qui ne s’installent jamais, même permis par ailleurs.",
            locks: "L’installation d’un plugin de la liste est refusée et dite.",
        },
        en: Text {
            title: "Denied plugins",
            help: "The list of the plugin ids that are never installed, even when allowed elsewhere.",
            locks: "Installing a plugin of the list is refused and said.",
        },
    },
    Def {
        key: VERIFIED_ONLY,
        sort: Sort::Restrict,
        form: on(),
        rows: &["pluginsVerifiedOnly"],
        within: None,
        category: Category::Plugins,
        later: false,
        example: "true",
        fr: Text {
            title: "N’installer que des versions vérifiées",
            help: "Une version du store sans le sceau « Vérifié par Escouade » est refusée, à l’installation comme à la mise à jour. Seule la valeur true (1) s’impose.",
            locks: "Plugins › « N’installer que des versions vérifiées », allumé et grisé.",
        },
        en: Text {
            title: "Install verified versions only",
            help: "A version of the store without the “Verified by Escouade” seal is refused, at an install as at an update. Only the value true (1) is imposed.",
            locks: "Plugins › “Install verified versions only”, on and grayed.",
        },
    },
    // ---------- projects ----------
    Def {
        key: FORCE_PRIVATE,
        sort: Sort::Restrict,
        form: on(),
        rows: &["privateProjects", "private"],
        within: None,
        category: Category::Projects,
        later: false,
        example: "true",
        fr: Text {
            title: "Projets privés d’office",
            help: "Chaque projet est privé : ses agents ne voient aucun autre projet, et aucun autre projet ne le voit. Seule la valeur true (1) s’impose.",
            locks: "« Nouveaux projets privés » et « Projet privé » de chaque projet, allumés et grisés.",
        },
        en: Text {
            title: "Private projects by default",
            help: "Every project is private: its agents see no other project, and no other project sees it. Only the value true (1) is imposed.",
            locks: "“New projects are private” and each project’s “Private project”, on and grayed.",
        },
    },
    Def {
        key: DENIED_FILES,
        sort: Sort::Restrict,
        form: Form::Union,
        rows: &[],
        within: None,
        category: Category::Projects,
        later: true,
        example: r#"["**/.env", "**/*.pem"]"#,
        fr: Text {
            title: "Motifs de fichiers interdits",
            help: "Des motifs de fichiers (à la manière de .gitignore) que les agents ne lisent ni n’écrivent, ajoutés à ceux des projets.",
            locks: "Lu et gardé ; appliqué aux agents avec la clôture des fichiers d’un projet (à venir).",
        },
        en: Text {
            title: "Denied file patterns",
            help: "File patterns (the .gitignore way) agents neither read nor write, added to those of the projects.",
            locks: "Read and kept; applied to the agents with a project’s fence of files (to come).",
        },
    },
    // ---------- network ----------
    Def {
        key: PROXY_URL,
        sort: Sort::Sensitive,
        form: Form::Text,
        rows: &["proxyUrl"],
        within: None,
        category: Category::Network,
        later: false,
        example: r#""http://proxy.example.com:3128""#,
        fr: Text {
            title: "Adresse du proxy",
            help: "Le proxy HTTP(S) de l’app, des plugins et des agents, par exemple http://proxy.example.com:3128.",
            locks: "Réglages › Avancé › Réseau, l’adresse du proxy grisée.",
        },
        en: Text {
            title: "Proxy address",
            help: "The HTTP(S) proxy of the app, the plugins and the agents, for example http://proxy.example.com:3128.",
            locks: "Settings › Advanced › Network, the proxy’s address grayed.",
        },
    },
    Def {
        key: PROXY_EXCLUSIONS,
        sort: Sort::Sensitive,
        form: Form::Union,
        rows: &["noProxy"],
        within: None,
        category: Category::Network,
        later: false,
        example: r#"["localhost", "127.0.0.1", ".corp.example.com"]"#,
        fr: Text {
            title: "Exclusions du proxy",
            help: "Les hôtes joints sans le proxy (NO_PROXY), un par ligne.",
            locks: "Réglages › Avancé › Réseau, les exclusions grisées.",
        },
        en: Text {
            title: "Proxy exclusions",
            help: "The hosts reached without the proxy (NO_PROXY), one per line.",
            locks: "Settings › Advanced › Network, the exclusions grayed.",
        },
    },
    Def {
        key: DENY_INSECURE_TLS,
        sort: Sort::Restrict,
        form: on(),
        rows: &["insecureTls"],
        within: None,
        category: Category::Network,
        later: false,
        example: "true",
        fr: Text {
            title: "Interdire d’ignorer la vérification TLS",
            help: "« Ignorer la vérification des certificats TLS » est éteint et ne s’allume plus. Seule la valeur true (1) s’impose.",
            locks: "Réglages › Avancé › Réseau › « Ignorer la vérification des certificats TLS », éteint et grisé.",
        },
        en: Text {
            title: "Deny skipping the TLS check",
            help: "“Skip TLS certificate verification” is off and can no longer be turned on. Only the value true (1) is imposed.",
            locks: "Settings › Advanced › Network › “Skip TLS certificate verification”, off and grayed.",
        },
    },
    Def {
        key: AUTHORITIES,
        sort: Sort::Sensitive,
        form: Form::Text,
        rows: &["extraCaFile"],
        within: None,
        category: Category::Network,
        later: false,
        example: r#""C:\\ProgramData\\Corp\\root-ca.pem""#,
        fr: Text {
            title: "Fichier d’autorités de certification",
            help: "Le chemin d’un fichier .pem d’autorités crues en plus du magasin du système (la racine d’un proxy d’entreprise).",
            locks: "Réglages › Avancé › Réseau › « Autorités de certification supplémentaires », grisé.",
        },
        en: Text {
            title: "Certificate authorities file",
            help: "The path of a .pem file of authorities trusted besides the system’s store (a corporate proxy’s root).",
            locks: "Settings › Advanced › Network › “Extra certificate authorities”, grayed.",
        },
    },
    // ---------- privacy ----------
    Def {
        key: TELEMETRY,
        sort: Sort::Restrict,
        form: off(),
        rows: &["telemetry"],
        within: None,
        category: Category::Privacy,
        later: false,
        example: "false",
        fr: Text {
            title: "Éteindre les statistiques d’usage",
            help: "Les statistiques anonymes ne partent plus. Seule la valeur false (0) s’impose.",
            locks: "Réglages › Général › « Statistiques d’usage », éteint et grisé.",
        },
        en: Text {
            title: "Turn the usage statistics off",
            help: "The anonymous statistics are no longer sent. Only the value false (0) is imposed.",
            locks: "Settings › General › “Usage statistics”, off and grayed.",
        },
    },
    Def {
        key: VOICE_ONLINE,
        sort: Sort::Restrict,
        form: off(),
        rows: &[],
        within: None,
        category: Category::Privacy,
        later: false,
        example: "false",
        fr: Text {
            title: "Interdire les voix en ligne",
            help: "La dictée et la voix de la conversation restent sur le poste (Whisper et Piper) : aucun service en ligne. Seule la valeur false (0) s’impose.",
            locks: "Réglages › Voix, « En ligne » ramené au local (Whisper, Piper) et refusé.",
        },
        en: Text {
            title: "Deny online voices",
            help: "The dictation and the conversation’s voice stay on the device (Whisper and Piper): no online service. Only the value false (0) is imposed.",
            locks: "Settings › Voice, “Online” brought back to local (Whisper, Piper) and refused.",
        },
    },
    Def {
        key: DICTATION,
        sort: Sort::Restrict,
        form: off(),
        rows: &["dictation"],
        within: None,
        category: Category::Privacy,
        later: false,
        example: "false",
        fr: Text {
            title: "Éteindre la dictée",
            help: "La dictée ne démarre plus. Seule la valeur false (0) s’impose.",
            locks: "Réglages › Voix › Dictée, grisé ; la dictée refusée.",
        },
        en: Text {
            title: "Turn the dictation off",
            help: "The dictation no longer starts. Only the value false (0) is imposed.",
            locks: "Settings › Voice › Dictation, grayed; the dictation refused.",
        },
    },
    // ---------- remote ----------
    Def {
        key: REMOTE,
        sort: Sort::Restrict,
        form: off(),
        rows: &["remoteAll", "projectRemote"],
        within: None,
        category: Category::Remote,
        later: false,
        example: "false",
        fr: Text {
            title: "Interdire le Remote Control",
            help: "Aucun agent n’est joignable depuis claude.ai. Seule la valeur false (0) s’impose.",
            locks: "« Remote control pour tous les agents », celui d’un projet et d’un agent, éteints et grisés.",
        },
        en: Text {
            title: "Deny the Remote Control",
            help: "No agent is reachable from claude.ai. Only the value false (0) is imposed.",
            locks: "“Remote control for all agents”, a project’s and an agent’s, off and grayed.",
        },
    },
    // ---------- updates ----------
    Def {
        key: CHANNEL,
        sort: Sort::Restrict,
        form: Form::Choice {
            choices: &["stable", "beta"],
            restricted: Some("stable"),
        },
        rows: &["betaUpdates"],
        within: None,
        category: Category::Updates,
        later: false,
        example: r#""stable""#,
        fr: Text {
            title: "Canal stable seulement",
            help: "Aucune version bêta n’est proposée. Seule la valeur stable s’impose.",
            locks: "Réglages › Général › « Recevoir les versions bêta », éteint et grisé.",
        },
        en: Text {
            title: "Stable channel only",
            help: "No beta version is offered. Only the value stable is imposed.",
            locks: "Settings › General › “Receive beta versions”, off and grayed.",
        },
    },
    Def {
        key: AUTO_UPDATE,
        sort: Sort::Restrict,
        form: off(),
        rows: &["autoUpdate"],
        within: None,
        category: Category::Updates,
        later: false,
        example: "false",
        fr: Text {
            title: "Couper la mise à jour automatique",
            help: "Une nouvelle version est proposée, jamais installée seule : le service informatique déploie les siennes. Seule la valeur false (0) s’impose.",
            locks: "Réglages › Général › « Installer les mises à jour automatiquement », éteint et grisé.",
        },
        en: Text {
            title: "Turn automatic updates off",
            help: "A new version is offered, never installed by itself: the IT department deploys its own. Only the value false (0) is imposed.",
            locks: "Settings › General › “Install updates automatically”, off and grayed.",
        },
    },
    // ---------- servers ----------
    Def {
        key: SERVERS,
        sort: Sort::Restrict,
        form: Form::Bound { universe: None },
        rows: &[],
        within: None,
        category: Category::Servers,
        later: false,
        example: r#"["https://escouade.corp.example.com"]"#,
        fr: Text {
            title: "Serveurs permis",
            help: "Les adresses des serveurs d’équipe et des serveurs en mode serveur que l’app peut joindre. Absente, tous.",
            locks: "Un serveur hors de la liste est refusé, avec la raison.",
        },
        en: Text {
            title: "Allowed servers",
            help: "The addresses of the team servers and the servers in server mode the app may reach. Absent, all.",
            locks: "A server outside the list is refused, with the reason.",
        },
    },
    Def {
        key: PERSONAL_SPACE,
        sort: Sort::Restrict,
        form: off(),
        rows: &[],
        within: None,
        category: Category::Servers,
        later: true,
        example: "false",
        fr: Text {
            title: "Interdire l’espace perso",
            help: "Seuls les espaces des serveurs permis s’ouvrent, pas celui du poste seul. Seule la valeur false (0) s’impose.",
            locks: "Lu et gardé ; appliqué avec les espaces du mode serveur (à venir).",
        },
        en: Text {
            title: "Deny the personal space",
            help: "Only the spaces of the allowed servers open, not the device’s own. Only the value false (0) is imposed.",
            locks: "Read and kept; applied with the spaces of the server mode (to come).",
        },
    },
    Def {
        key: AGGREGATE_VIEW,
        sort: Sort::Restrict,
        form: off(),
        rows: &[],
        within: None,
        category: Category::Servers,
        later: true,
        example: "false",
        fr: Text {
            title: "Interdire la vue agrégée",
            help: "Les espaces ne se montrent pas ensemble dans une même vue. Seule la valeur false (0) s’impose.",
            locks: "Lu et gardé ; appliqué avec les espaces du mode serveur (à venir).",
        },
        en: Text {
            title: "Deny the aggregate view",
            help: "The spaces are not shown together in one view. Only the value false (0) is imposed.",
            locks: "Read and kept; applied with the spaces of the server mode (to come).",
        },
    },
    Def {
        key: CONTENTLESS,
        sort: Sort::Restrict,
        form: on(),
        rows: &[],
        within: None,
        category: Category::Servers,
        later: true,
        example: "true",
        fr: Text {
            title: "Notifications sans contenu",
            help: "Les notifications venues d’un serveur disent qu’il se passe quelque chose, jamais quoi. Seule la valeur true (1) s’impose.",
            locks: "Lu et gardé ; appliqué avec les notifications du serveur d’équipe (à venir).",
        },
        en: Text {
            title: "Notifications without content",
            help: "The notifications from a server say something happens, never what. Only the value true (1) is imposed.",
            locks: "Read and kept; applied with the team server’s notifications (to come).",
        },
    },
    // ---------- MCP ----------
    Def {
        key: MCP_EXTERNAL,
        sort: Sort::Restrict,
        form: Form::Ceiling {
            choices: MCP_CLIENTS,
        },
        rows: &[],
        within: None,
        category: Category::Mcp,
        later: false,
        example: r#""readOnly""#,
        fr: Text {
            title: "Clients MCP extérieurs",
            help: "Ce que Claude hors d’Escouade (un client MCP extérieur) peut faire par le serveur MCP d’Escouade : denied (rien), readOnly (lire seulement), allowed (tout). Les agents d’Escouade ne sont pas concernés.",
            locks: "Un client extérieur refusé à la connexion (denied), ou ses outils qui écrivent refusés (readOnly).",
        },
        en: Text {
            title: "Outside MCP clients",
            help: "What Claude outside Escouade (an outside MCP client) may do through Escouade’s MCP server: denied (nothing), readOnly (read only), allowed (everything). Escouade’s own agents are not concerned.",
            locks: "An outside client refused at its connection (denied), or its tools that write refused (readOnly).",
        },
    },
    // ---------- enrollment ----------
    Def {
        key: TEAM_SERVER,
        sort: Sort::Value,
        form: Form::Text,
        rows: &[],
        within: None,
        category: Category::Enrollment,
        later: false,
        example: r#""https://escouade.corp.example.com""#,
        fr: Text {
            title: "Serveur d’équipe",
            help: "L’adresse du serveur d’équipe de l’entreprise : le membre n’a plus qu’à accepter son invitation.",
            locks: "Gardé par l’app ; l’écran d’entrée dans l’équipe le préremplira (à venir).",
        },
        en: Text {
            title: "Team server",
            help: "The address of the company’s team server: the member only has to accept their invitation.",
            locks: "Kept by the app; the screen to join the team will fill it in (to come).",
        },
    },
    Def {
        key: TEAM_FINGERPRINT,
        sort: Sort::Value,
        form: Form::Text,
        rows: &[],
        within: None,
        category: Category::Enrollment,
        later: false,
        example: r#""SHA256:3f9a6c1e8b2d47f0a5c3e9d1b7f2a4c6e8d0b2f4a6c8e0d2b4f6a8c0e2d4b6f8""#,
        fr: Text {
            title: "Empreinte de l’équipe",
            help: "L’empreinte de l’équipe attendue sur le serveur d’équipe : une équipe d’une autre empreinte est refusée.",
            locks: "Gardée par l’app ; vérifiée à l’entrée dans l’équipe (à venir).",
        },
        en: Text {
            title: "Team fingerprint",
            help: "The fingerprint of the team expected on the team server: a team of another fingerprint is refused.",
            locks: "Kept by the app; checked when joining the team (to come).",
        },
    },
    Def {
        key: ONLY_THIS_SERVER,
        sort: Sort::Restrict,
        form: on(),
        rows: &[],
        within: None,
        category: Category::Enrollment,
        later: false,
        example: "true",
        fr: Text {
            title: "Refuser tout autre serveur d’équipe",
            help: "Seul le serveur d’équipe posé par la stratégie peut être joint. Seule la valeur true (1) s’impose.",
            locks: "Un autre serveur d’équipe est refusé, avec la raison.",
        },
        en: Text {
            title: "Refuse any other team server",
            help: "Only the team server the policy sets may be joined. Only the value true (1) is imposed.",
            locks: "Another team server is refused, with the reason.",
        },
    },
];

/// The policy of a key.
pub fn def(key: &str) -> Option<&'static Def> {
    POLICIES.iter().find(|d| d.key == key)
}

/// The keys of the list.
pub fn keys() -> Vec<&'static str> {
    POLICIES.iter().map(|d| d.key).collect()
}

/// A value as read (a registry `REG_DWORD`, a string, a list), brought to the type the list says: a `REG_DWORD` 0 or
/// 1 becomes a Boolean, one string a list of one, a JSON string an object. Err says why it does not fit.
pub fn convert(def: &Def, value: &Value) -> Result<Value, String> {
    match def.kind() {
        Type::Boolean => match value {
            Value::Bool(b) => Ok(Value::Bool(*b)),
            Value::Number(n) if n.as_u64() == Some(0) => Ok(Value::Bool(false)),
            Value::Number(n) if n.as_u64() == Some(1) => Ok(Value::Bool(true)),
            Value::String(s) if s == "true" || s == "1" => Ok(Value::Bool(true)),
            Value::String(s) if s == "false" || s == "0" => Ok(Value::Bool(false)),
            _ => Err("a Boolean is expected (true or false, 1 or 0)".to_owned()),
        },
        Type::String => match value {
            Value::String(s) => {
                let s = s.trim();
                if let Some(choices) = def.form.choices() {
                    if !choices.contains(&s) {
                        return Err(format!(
                            "one of these values is expected: {}",
                            choices.join(", ")
                        ));
                    }
                }
                Ok(Value::String(s.to_owned()))
            }
            _ => Err("a string is expected".to_owned()),
        },
        Type::Number => {
            let n = match value {
                Value::Number(n) => n.as_u64(),
                Value::String(s) => s.trim().parse::<u64>().ok(),
                _ => None,
            }
            .ok_or_else(|| "a whole number is expected".to_owned())?;
            let Form::Number { min, max } = def.form else {
                unreachable!("a number's form")
            };
            if n < u64::from(min) || n > u64::from(max) {
                return Err(format!("a number from {min} to {max} is expected"));
            }
            Ok(Value::from(n))
        }
        Type::List => {
            let items: Vec<&Value> = match value {
                Value::Array(a) => a.iter().collect(),
                Value::String(_) => vec![value],
                _ => return Err("a list is expected".to_owned()),
            };
            let mut out: Vec<Value> = Vec::new();
            for item in items {
                let Some(s) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
                    return Err("a list of names is expected".to_owned());
                };
                if let Some(universe) = def.form.choices() {
                    if !universe.contains(&s) {
                        return Err(format!("“{s}” is not one of: {}", universe.join(", ")));
                    }
                }
                if !out.iter().any(|o| o == s) {
                    out.push(Value::String(s.to_owned()));
                }
            }
            Ok(Value::Array(out))
        }
        Type::Json => {
            let parsed = match value {
                Value::String(s) => {
                    serde_json::from_str(s).map_err(|e| format!("not valid JSON: {e}"))?
                }
                other => other.clone(),
            };
            if !parsed.is_object() {
                return Err("a JSON object is expected".to_owned());
            }
            Ok(parsed)
        }
    }
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
