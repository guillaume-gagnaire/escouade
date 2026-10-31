//! Types persisted on disk and exchanged with the frontend.

use crate::agent::NotifyKind;
use crate::plan::PlanState;
use crate::resources::Resources;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Empty = auto-detect `claude` on PATH.
    pub claude_path: String,
    /// The model, effort and mode of new agents before they were each account's (ESC-128): read
    /// once, to give them to the Claude Code accounts kept by the update (`accounts::migrate`),
    /// never written again.
    #[serde(skip_serializing)]
    pub default_model: String,
    #[serde(skip_serializing)]
    pub default_effort: String,
    #[serde(skip_serializing)]
    pub default_mode: String,
    pub sound: bool,
    pub os_notifications: bool,
    /// "Me prévenir pour": what the chime and the system notifications are for.
    pub notify_for: NotifyFor,
    /// Stop idle Claude processes after N minutes (0 = never). Sessions stay resumable.
    pub idle_stop_minutes: u32,
    /// « Ouvrir les nouveaux agents dans l'interface de Claude Code »: a new agent (not a
    /// ticket's) starts in the Claude Code view (`AgentMeta::terminal`).
    pub terminal_view: bool,
    pub pwsh_path: String,
    pub bash_path: String,
    pub wsl_distro: String,
    /// HTTP(S) proxy URL, e.g. `http://user:pass@proxy:3128`. Empty = direct connection.
    pub proxy_url: String,
    /// Comma-separated hosts that bypass the proxy (NO_PROXY).
    pub no_proxy: String,
    /// Also export the proxy variables in integrated terminals.
    pub proxy_terminals: bool,
    /// TLS certificates are not checked (a proxy that decrypts the traffic with a certificate of
    /// its own): the app's own requests, its updates and Claude Code's.
    pub insecure_tls: bool,
    /// Send "continue" by itself to an agent stopped by the usage limit, once the quota resets.
    pub auto_resume: bool,
    /// "Pause au-delà du quota", in percent (80, 90, 95 or 100): no ticket of any board starts
    /// while the 5-hour or the weekly window is used this much or more (`board::autopilot_pause`).
    pub quota_pause: u32,
    /// « Changer de compte à », in percent (80, 90, 95, or 100: « À la limite »): an agent whose
    /// account has a quota meter used this much or more goes on, at the end of its turn, on another
    /// usable account of its harness (`switching`), and new agents go to another one (ESC-148).
    /// At 100, nothing moves before the limit, where « Reprendre sur un autre compte… »
    /// (`switch_on_limit`) decides, as before: settings saved before it have 100.
    pub switch_at: u32,
    /// "Installer les mises à jour automatiquement": a downloaded update restarts the app by
    /// itself once it is at rest (`updates::busy`). Off, the user restarts it, or the update
    /// installs when the app is closed.
    pub auto_update: bool,
    /// What the external ticket systems' links do (their accounts are kept apart, with their
    /// secrets: `integrations::Accounts`).
    pub integrations: IntegrationSettings,
    /// "Langue de l'interface": "system" (French when the system is, English otherwise), "fr" or
    /// "en" (`i18n::resolve`).
    pub language: String,
    /// "Langue des textes rédigés par Claude": "ui" (the interface's), "fr" or "en". Many teams
    /// commit in English with an interface in French.
    pub claude_language: String,
    /// « Modèle des textes rédigés »: the harness the one-shot questions go to (`engine::oneshot`).
    /// None, or one not found: Claude Code when it is found (its Haiku), else the first found.
    pub writer_engine: Option<EngineId>,
    /// The model of the texts, for a harness that takes the one Escouade chooses (OpenCode:
    /// `<provider>/<model>`); empty, or none of its models: its default one.
    pub writer_model: String,
    /// « Les agents peuvent piloter Escouade »: the MCP server runs (`mcp`), given to Escouade's
    /// agents and declared in the user config of every harness installed, account by account
    /// (`mcp::install`). A `mcpHarnesses` of before (« <harness> peut piloter Escouade », ESC-50)
    /// is dropped at the load: the one switch says for all (ESC-119).
    pub mcp_enabled: bool,
    /// The MCP server's port on 127.0.0.1: 0 until its first start chooses a free one (47000 to
    /// 47999), kept from then on. The backend's own: a save from the window leaves it as it is.
    pub mcp_port: u16,
    /// The accounts of every harness, in the order new agents try them: only those the user added
    /// (ESC-128), none on a first installation.
    pub accounts: Vec<Account>,
    /// How far the accounts were brought by the updates (`accounts::ACCOUNTS_VERSION`): 0 for
    /// settings saved before 1.8.1, whose Principals `accounts::migrate` keeps or drops once.
    /// A new installation starts at the current one.
    #[serde(default)]
    pub accounts_version: u32,
    /// « Ordre des harness »: the harnesses one after the other, every one Escouade knows listed
    /// once (`accounts::normalize`). It gives a project its fallback harnesses after its default
    /// one (`accounts::fallback_engines`, ESC-88) and the order of the groups of « Comptes ».
    pub engine_order: Vec<EngineId>,
    /// « Les agents tiennent une liste de tâches »: Claude Code's task list tools are switched on
    /// for the agents that start (`CLAUDE_CODE_ENABLE_TODO_TOOLS`, see `Settings::agent_env`), so
    /// that their progress can be shown (`plan`).
    pub todo_tools: bool,
    /// « Mettre à jour Claude Code automatiquement » (ESC-133): Escouade runs `claude update` at its
    /// start, then once a day, in a process of its own (`selfupdate`), every `claude` it launches
    /// having its own update turned off. Unless the user turned it off in Claude Code itself.
    pub claude_self_update: bool,
    /// « Reprendre sur un autre compte un agent de ticket arrêté par la limite »: such an agent
    /// goes on by itself on the account `accounts::pick` gives, when another is usable, rather than
    /// wait for its own to reset. Off unless the user switches it on (not verified with a second
    /// real account): settings saved before it have it off too.
    pub switch_on_limit: bool,
    /// « Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés »: an
    /// agent stopped by the usage limit, none of whose harness's accounts serving its project can
    /// take it, goes on by itself on the next of the project's fallback harnesses (a handover,
    /// `handover`); the autopilot follows it for the tickets. Off unless the user switches it on.
    pub handover_on_limit: bool,
    /// « Modèle de passation »: the model an agent handed over to a harness runs on there, by
    /// harness; none (or one it does not list): its best (`Engine::handover_model`).
    pub handover_models: std::collections::BTreeMap<EngineId, String>,
    /// The models marked as favorites, by harness (OpenCode lists every model of its connected
    /// providers): first in the composer's menu, in the order they were marked.
    pub favorite_models: std::collections::BTreeMap<EngineId, Vec<String>>,
    /// « Statistiques d’usage » (`telemetry`): None until the user answered the question
    /// of the first start (settings saved before it have none: they are asked too).
    pub telemetry: Option<bool>,
    /// « Taille maximale d’un fichier joint », in MB (0: none): a bigger file is refused
    /// (`attachfiles`).
    pub attach_max_mb: u32,
    /// « Avertir au-delà de », in MB (0: never): a file this big is joined, with a warning (the
    /// window's).
    pub attach_warn_mb: u32,
    /// « Joindre le fichier d’origine »: a file dropped is joined by the path it has, nothing copied;
    /// off, it is always copied (`attachfiles`).
    pub attach_origin: bool,
    /// « Texte de plus de 256 Ko »: "path" (sent by its path, as any file) or "refuse" (refused, as
    /// before 1.8). Any other value is "path" (`normalize_attachments`).
    pub attach_big_text: String,
    /// « Effacer les copies après », in days (0: they stay as long as their owner does): the copies
    /// of joined files older than this go (`attachfiles::purge`).
    pub attach_keep_days: u32,
    /// « Délai avant l'envoi pendant un tour », in seconds (0: off), one of `MID_TURN_DELAYS`.
    pub mid_turn_delay_secs: u32,
    /// « Remote control pour tous les agents »: every agent, those of tickets too, is reachable
    /// from claude.ai (`AgentMeta::remote_control`, `RemoteSource::Global`).
    pub remote_control_all: bool,
    /// « Nouveaux projets privés » (Réglages › Projets): a project added is private
    /// (`Project::private`) from the start. Off by default.
    pub private_projects: bool,
    /// « Index du code sur tous les projets » (Réglages › Projets, ESC-293): the index of every
    /// project that has not made its own choice (`Project::code_index`) is on. Off by default.
    /// Saved at once by `set_code_index_all`, never by the settings modal's draft.
    pub code_index_all: bool,
    /// « Thème » (Réglages › Application › Affichage): the window's look, the system's by default.
    pub theme: ThemeChoice,
    /// « Couleurs » (Réglages › Application › Affichage, PLG-7): the theme of a plugin the window takes,
    /// `<plugin id>/<theme id>`; empty, Escouade's. Its values are filtered by the window
    /// (src/lib/plugin-themes.ts); a plugin switched off or gone leaves Escouade's colours, the choice kept.
    pub plugin_theme: String,
    /// « N'installer que des versions vérifiées » (la vue Plugins, STORE-9): a version from the store
    /// without the seal « Vérifié par Escouade » is refused, at an install as at an update. Off by
    /// default (docs/STORE.md §4.4).
    pub plugins_verified_only: bool,
    /// « Zoom de l’interface », in percent (`ZOOM_LEVELS`): the whole window's, as the WebView's
    /// zoom (text, spacing and icons together); Ctrl+= / Ctrl+- / Ctrl+0 change it too.
    pub ui_zoom: u32,
    /// « Taille du texte des terminaux », in CSS pixels, before the zoom (`FONT_SIZE_RANGE`).
    pub terminal_font_size: f64,
    /// « Taille du texte de l’éditeur », in CSS pixels, before the zoom (`FONT_SIZE_RANGE`).
    pub editor_font_size: f64,
    /// The width of the projects' rail, in CSS pixels (ESC-168), its right edge dragged:
    /// `RAIL_WIDTH_RANGE`, the same for all projects; 0 is its usual width (52 px, 72 with a
    /// label under a tile), until the edge is dragged.
    pub rail_width: u32,
    /// The width of the sidebar of the agents and the Kanban, in CSS pixels (ESC-204), its right
    /// edge dragged: `SIDE_WIDTH_RANGE`, the same for all projects; 0 is its usual width (276 px).
    pub side_width: u32,
    /// « Projection des quotas »: where each quota would end at the pace used so far (« → 64 % »),
    /// and the day's cost where it would end the day, in the status bar past 80 % or always, or
    /// nowhere (`QuotaProjection`).
    pub quota_projection: QuotaProjection,
    /// « Voix »: the dictation (`voice`).
    pub voice: VoiceSettings,
    /// « Ton nom » (Réglages › Application › Équipe, ESC-190): written in the tickets shared with
    /// the team and in those this machine moves on; a project may have its own
    /// (`Project::user_name`). Empty until given: the name git knows stands in (ESC-243).
    pub user_name: String,
    /// « Ton nom » emptied on purpose in the settings (ESC-243): its field is no longer filled from git.
    pub user_name_cleared: bool,
    /// The id beside the name (`sharing::person_id`): from the git e-mail, else drawn once.
    pub user_id: String,
    /// The first launch's window asking the name was shown (answered or put off).
    pub name_asked: bool,
}

/// The widths the projects' rail is dragged to (ESC-168): its tiles alone, up to a list of names.
pub const RAIL_WIDTH_RANGE: (u32, u32) = (52, 300);

/// The widths the sidebar of the agents and the Kanban is dragged to (ESC-204).
pub const SIDE_WIDTH_RANGE: (u32, u32) = (220, 480);

/// « Projection des quotas »: where the status bar writes the projection of each quota (the
/// panel of the accounts and the tooltips always do, unless `Off`). Read from the switch of
/// before 2.0.2 too: `true` was `Alert`, `false` is `Off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuotaProjection {
    /// Past 80 % only (orange, red past 100 %): a green one asks for nothing.
    #[default]
    Alert,
    /// Every one, green ones too.
    Always,
    /// None anywhere.
    Off,
}

impl<'de> Deserialize<'de> for QuotaProjection {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Switch(bool),
            Name(String),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Switch(false) => Self::Off,
            Raw::Name(n) if n == "always" => Self::Always,
            Raw::Name(n) if n == "off" => Self::Off,
            // `true`, "alert", or a value of a later version: the default.
            _ => Self::Alert,
        })
    }
}

/// The steps of « Zoom de l’interface », in percent: the window's own (`Settings::normalize_display`).
pub const ZOOM_LEVELS: [u32; 8] = [80, 90, 100, 110, 125, 150, 175, 200];
/// The font sizes the terminals and the editor may have, in CSS pixels.
pub const FONT_SIZE_RANGE: (f64, f64) = (9.0, 24.0);
/// Their font size by default (what they had before it could be set).
pub const DEFAULT_FONT_SIZE: f64 = 12.5;
/// The least room the interface is laid out for, in CSS pixels: the window's minimum size.
pub const MIN_ROOM: (f64, f64) = (1000.0, 600.0);

/// The zoom `zoom` as a window of `width` × `height` logical pixels can show it: the biggest
/// step up to it that leaves the interface its `MIN_ROOM`, never under 100 % (as the window's
/// `fitZoom`).
/// « Thème »: the system's (and its changes, at once), light or dark. What a later version may
/// write, or a hand, is the system's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    Light,
    Dark,
    #[default]
    #[serde(other)]
    System,
}

pub fn fit_zoom(zoom: u32, width: f64, height: f64) -> u32 {
    let fits = |z: u32| {
        let f = f64::from(z) / 100.0;
        width / f >= MIN_ROOM.0 - 0.5 && height / f >= MIN_ROOM.1 - 0.5
    };
    ZOOM_LEVELS
        .iter()
        .copied()
        .filter(|&z| z <= zoom && (z <= 100 || fits(z)))
        .max()
        .unwrap_or(100)
}

/// An account of a harness: the harness with a configuration folder of its own (`accounts`), as
/// Claude Code with its `CLAUDE_CONFIG_DIR`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    /// `accounts::PRINCIPAL` for the user's own Claude Code (`accounts::principal_id` for another
    /// harness's).
    pub id: String,
    pub name: String,
    /// Its harness; Claude Code for an account saved before 1.8.
    pub engine: EngineId,
    /// Its configuration folder, given to its harness's processes by the variable the harness
    /// declares (`Engine::config_var`: `CLAUDE_CONFIG_DIR`); empty for Principal, launched
    /// without one.
    pub config_dir: String,
    /// Its own `claude`; empty: the settings' one.
    pub claude_path: String,
    /// New agents may go to it.
    pub active: bool,
    /// « Réservé aux projets »: the ids of the only projects whose new agents and tickets may go
    /// to it (a client's account kept to that client's projects); empty: every project. Principal's
    /// too: a project no active account serves then starts nothing (`accounts::serves_any`).
    pub projects: Vec<String>,
    /// « Budget par jour ($) », for a harness without a quota (OpenCode): once its turns of the day
    /// cost this much, the autopilot pauses until midnight, as at a quota used up
    /// (`board::account_pause`). None (or 0): no budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_budget: Option<f64>,
    /// The model, the effort and the permission mode of its new agents unless they say theirs
    /// (ESC-128); empty: its harness's default (`accounts::defaults`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_model: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_effort: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub default_mode: String,
    /// Gemini CLI (ESC-275): it goes by its API key (in the keychain, given once with « Clé
    /// d’API »), not by the Google sign-in made by `gemini` in its folder.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub api_key: bool,
}

/// The dictation's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VoiceSettings {
    /// The microphone (its cpal id); empty: the system's default one.
    pub device: String,
    /// The Whisper model (`voice::models::CATALOG`), downloaded at the first dictation.
    pub model: String,
    /// What Whisper is told the speech is in: "ui" (the interface's language), "auto" (found
    /// by Whisper), or a language's code ("fr", "en", "de"…).
    pub language: String,
    /// The shortcut that starts and stops a dictation, held down to talk: "Mod" is Ctrl, Cmd on
    /// macOS (the window reads it, `shortcuts.ts`).
    pub shortcut: String,
    /// The conversation's voice (ESC-97).
    pub speech: SpeechSettings,
    /// Who transcribes a dictation: "local" (Whisper, on the machine) or "online" (the service
    /// `online_service`, with the user's key kept in the keychain, ESC-110). Off by default.
    pub transcription: String,
    /// The service of « En ligne » (`voice::online::Provider::id`).
    pub online_service: String,
    /// The model chosen for each service, by its id (ESC-132); none: its default (`Provider::model`).
    pub online_models: std::collections::BTreeMap<String, String>,
    /// « Transcription en temps réel » of each service that offers it, by its id (ESC-136); none: on.
    pub online_live: std::collections::BTreeMap<String, bool>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            device: String::new(),
            model: crate::voice::models::DEFAULT_MODEL.into(),
            language: "ui".into(),
            shortcut: "Mod+Shift+Space".into(),
            speech: SpeechSettings::default(),
            transcription: "local".into(),
            online_service: "openai".into(),
            online_models: Default::default(),
            online_live: Default::default(),
        }
    }
}

/// How the agent's passages are read aloud in a conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeechSettings {
    /// The voice of each language (`fr`, `en`) by its id (`voice::speech::catalog`); none: the default one.
    pub voices: std::collections::BTreeMap<String, String>,
    /// The speaker of each voice that has several, by the voice's id.
    pub speakers: std::collections::BTreeMap<String, u32>,
    /// How fast it reads: 1 is the voice's own pace (0.5 to 2).
    pub rate: f32,
    /// How loud (0 to 1).
    pub volume: f32,
    /// The silence that ends what the user says, and sends it, in milliseconds.
    pub silence_ms: u32,
    /// Who makes the voice (ESC-132): "local" (Piper, on the machine) or "online" (the service `online_service`, with
    /// the key of its dictation).
    pub engine: String,
    pub online_service: String,
    /// The model chosen for each service, by its id; none: its default (`online_catalog::default_speak_model`).
    pub online_models: std::collections::BTreeMap<String, String>,
    /// The voice chosen, by `<service>|<lang>` (`<service>|<model>|<lang>` for a service whose voices are models of
    /// their own); none: the service's default voice.
    pub online_voices: std::collections::BTreeMap<String, String>,
}

impl Default for SpeechSettings {
    fn default() -> Self {
        Self {
            voices: Default::default(),
            speakers: Default::default(),
            rate: 1.0,
            volume: 1.0,
            silence_ms: 1200,
            engine: "local".into(),
            online_service: "openai".into(),
            online_models: Default::default(),
            online_voices: Default::default(),
        }
    }
}

impl SpeechSettings {
    /// The voice of `lang`: the one chosen, else the default one.
    pub fn voice(&self, lang: &str) -> String {
        self.voices
            .get(lang)
            .filter(|id| crate::voice::speech::catalog::find(id).is_some())
            .cloned()
            .unwrap_or_else(|| crate::voice::speech::catalog::default_voice(lang).to_string())
    }
}

impl Account {
    /// Whether a new agent of `project` (none: one of no project) may go to it.
    pub fn serves(&self, project: Option<&str>) -> bool {
        self.projects.is_empty() || project.is_some_and(|p| self.projects.iter().any(|x| x == p))
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            engine: EngineId::CLAUDE,
            config_dir: String::new(),
            claude_path: String::new(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            default_model: String::new(),
            default_effort: String::new(),
            default_mode: String::new(),
            api_key: false,
        }
    }
}

/// "Me prévenir pour": which kinds of event chime and show a system notification. A kind switched
/// off does neither; the other signals stay: the agent's tab and card blink, and the taskbar
/// flashes (the Dock bounces).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct NotifyFor {
    /// "Questions et autorisations".
    pub questions: bool,
    /// "Tâches terminées".
    pub done: bool,
    /// "Erreurs".
    pub errors: bool,
    /// "Tickets (prêt à tester, bloqué)".
    pub tickets: bool,
}

impl Default for NotifyFor {
    fn default() -> Self {
        Self {
            questions: true,
            done: true,
            errors: true,
            tickets: true,
        }
    }
}

impl NotifyFor {
    /// The user wants to be told of this kind of event.
    pub fn allows(&self, kind: NotifyKind) -> bool {
        match kind {
            NotifyKind::Question => self.questions,
            NotifyKind::Done => self.done,
            NotifyKind::Error => self.errors,
            NotifyKind::Ticket => self.tickets,
        }
    }
}

/// The sync with the external ticket systems and their automatic import, for every project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct IntegrationSettings {
    /// "Mettre à jour le statut externe": a ticket that changes column gives its external one the
    /// state its project's link says.
    pub sync_states: bool,
    /// "Publier un résumé à chaque boucle".
    pub loop_comments: bool,
    /// "Extraire les critères d'acceptation" of an imported ticket's description or checklists.
    pub extract_criteria: bool,
    /// "Importer les tickets étiquetés": the open tickets of the linked sources that carry
    /// `import_label` come in by themselves.
    pub auto_import: bool,
    pub import_label: String,
    /// Minutes between two automatic imports.
    pub import_every: u32,
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self {
            sync_states: true,
            loop_comments: false,
            extract_criteria: true,
            auto_import: false,
            import_label: "claude-ready".into(),
            import_every: 15,
        }
    }
}

/// The steps of « Délai avant l'envoi pendant un tour », in seconds (0: off).
pub const MID_TURN_DELAYS: [u32; 5] = [0, 5, 10, 20, 30];
const DEFAULT_MID_TURN_DELAY: u32 = 5;

/// The variable that turns on Claude Code's task list tools (read in its binary, not documented).
pub const TODO_TOOLS_VAR: &str = "CLAUDE_CODE_ENABLE_TODO_TOOLS";

impl Settings {
    /// What the settings of the attached files say that they may not (a value saved by a later
    /// version, or written by hand): « Texte de plus de 256 Ko » is "path" unless it is "refuse".
    pub fn normalize_attachments(&mut self) {
        if self.attach_big_text != "refuse" {
            self.attach_big_text = "path".into();
        }
    }

    /// A delay off its steps becomes the default.
    pub fn normalize_mid_turn_delay(&mut self) {
        if !MID_TURN_DELAYS.contains(&self.mid_turn_delay_secs) {
            self.mid_turn_delay_secs = DEFAULT_MID_TURN_DELAY;
        }
    }

    /// What the display settings say that they may not (written by hand, or by a later version):
    /// a zoom off the steps is the nearest one, a font size out of range its nearest bound (not a
    /// number: the default), to the half pixel; the rail's and the sidebar's widths in their
    /// ranges, 0 (their usual ones) left as it is.
    pub fn normalize_display(&mut self) {
        if self.rail_width != 0 {
            self.rail_width = self
                .rail_width
                .clamp(RAIL_WIDTH_RANGE.0, RAIL_WIDTH_RANGE.1);
        }
        if self.side_width != 0 {
            self.side_width = self
                .side_width
                .clamp(SIDE_WIDTH_RANGE.0, SIDE_WIDTH_RANGE.1);
        }
        let z = self.ui_zoom;
        self.ui_zoom = *ZOOM_LEVELS
            .iter()
            .min_by_key(|l| l.abs_diff(z))
            .unwrap_or(&100);
        for size in [&mut self.terminal_font_size, &mut self.editor_font_size] {
            *size = if size.is_finite() {
                ((*size * 2.0).round() / 2.0).clamp(FONT_SIZE_RANGE.0, FONT_SIZE_RANGE.1)
            } else {
                DEFAULT_FONT_SIZE
            };
        }
    }

    /// The network settings of a Claude Code process: its proxy, and its TLS verification off
    /// when asked.
    pub fn claude_env(&self) -> Vec<(String, String)> {
        let mut env = self.proxy_env();
        if self.insecure_tls {
            env.push(("NODE_TLS_REJECT_UNAUTHORIZED".into(), "0".into()));
        }
        env
    }

    /// What a Claude Code process that runs an agent is started with: its network settings, and
    /// the variable that gives it the task list tools (`TaskCreate`, `TaskUpdate`…) when the
    /// setting is on. Claude Code 2.1.296 has none of them for the current models without it. Off,
    /// nothing is added (the user's own environment decides); the other Claude Code processes
    /// (naming, commit messages, sign-in) have no use for the tools and get `claude_env` alone.
    pub fn agent_env(&self) -> Vec<(String, String)> {
        let mut env = self.claude_env();
        if self.todo_tools {
            env.push((TODO_TOOLS_VAR.into(), "1".into()));
        }
        env
    }

    /// Environment variables to inject into child processes for the configured proxy.
    pub fn proxy_env(&self) -> Vec<(String, String)> {
        let url = self.proxy_url.trim();
        if url.is_empty() {
            return Vec::new();
        }
        let mut env = Vec::new();
        for k in ["HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy"] {
            env.push((k.to_string(), url.to_string()));
        }
        let no = self.no_proxy.trim();
        if !no.is_empty() {
            env.push(("NO_PROXY".into(), no.to_string()));
            env.push(("no_proxy".into(), no.to_string()));
        }
        env
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            claude_path: String::new(),
            default_model: "sonnet".into(),
            default_effort: "medium".into(),
            default_mode: "auto".into(),
            accounts_version: crate::accounts::ACCOUNTS_VERSION,
            sound: true,
            os_notifications: true,
            notify_for: NotifyFor::default(),
            idle_stop_minutes: 30,
            terminal_view: false,
            pwsh_path: String::new(),
            bash_path: String::new(),
            wsl_distro: String::new(),
            proxy_url: String::new(),
            no_proxy: "localhost,127.0.0.1".into(),
            proxy_terminals: false,
            insecure_tls: false,
            auto_resume: true,
            quota_pause: 100,
            switch_at: 100,
            auto_update: true,
            integrations: IntegrationSettings::default(),
            language: "system".into(),
            claude_language: "ui".into(),
            writer_engine: None,
            writer_model: String::new(),
            mcp_enabled: false,
            mcp_port: 0,
            accounts: Vec::new(),
            engine_order: Vec::new(),
            todo_tools: true,
            claude_self_update: true,
            switch_on_limit: false,
            handover_on_limit: false,
            handover_models: Default::default(),
            favorite_models: Default::default(),
            telemetry: None,
            attach_max_mb: 0,
            attach_warn_mb: 200,
            attach_origin: true,
            attach_big_text: "path".into(),
            attach_keep_days: 0,
            mid_turn_delay_secs: DEFAULT_MID_TURN_DELAY,
            remote_control_all: false,
            private_projects: false,
            code_index_all: false,
            theme: ThemeChoice::System,
            plugin_theme: String::new(),
            plugins_verified_only: false,
            ui_zoom: 100,
            terminal_font_size: DEFAULT_FONT_SIZE,
            editor_font_size: DEFAULT_FONT_SIZE,
            rail_width: 0,
            side_width: 0,
            quota_projection: QuotaProjection::Alert,
            voice: VoiceSettings::default(),
            user_name: String::new(),
            user_id: String::new(),
            name_asked: false,
            user_name_cleared: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: String,
    /// « Lettres de la tuile » (ESC-167): what its tile on the rail says; empty is the initials of
    /// its name. Of this machine, as its colour.
    #[serde(default)]
    pub tile_letters: String,
    /// « Logo » (ESC-167): the file of its copy under the project in the app's data folder
    /// (`projectlogo`), shown in place of the letters; empty for none. Set by `set_project_logo`
    /// alone, never by `update_project`.
    #[serde(default)]
    pub logo: String,
    /// « Libellé sous la tuile » (ESC-168): its label under its tile on a narrow rail; off by
    /// default. Of this machine, as its colour.
    #[serde(default)]
    pub tile_label: bool,
    /// The label's text, shorter than its name; empty is its name. The rail widened to a list shows
    /// it beside the tile, on or not.
    #[serde(default)]
    pub tile_label_text: String,
    /// « Ton nom dans ce projet » (ESC-190): the name its shared tickets carry; empty is the
    /// app's (`Settings::user_name`). Of this machine, never in `.escouade.yml`.
    #[serde(default)]
    pub user_name: String,
    /// « Partager chaque nouveau ticket »: a ticket made here is shared from the start.
    #[serde(default)]
    pub share_new: bool,
    #[serde(default)]
    pub worktree_per_agent: bool,
    #[serde(default)]
    pub created_at: i64,
    /// Commands that launch the project (dev servers, watchers…), each in its own read-only terminal.
    #[serde(default)]
    pub run_commands: Vec<RunCommand>,
    /// The board: what validating a ticket does, its agents, its tickets' key.
    #[serde(default)]
    pub board: BoardSettings,
    /// Untracked files of the project copied into every new worktree (glob patterns).
    #[serde(default = "default_worktree_copy")]
    pub worktree_copy: Vec<String>,
    /// Run in every new worktree, one after the other, before its agent's first message
    /// (dependencies, generated code…).
    #[serde(default)]
    pub worktree_setup: Vec<WorktreeStep>,
    /// Run in a worktree before the app removes it (what its setup made outside of it).
    #[serde(default)]
    pub worktree_teardown: Vec<WorktreeStep>,
    /// The external ticket systems its tickets come from, and what moving them does there.
    #[serde(default)]
    pub integrations: ProjectIntegrations,
    /// "Commit": who writes the commits of the files panel's « Commit… » and « Commit tout… ».
    #[serde(default)]
    pub commit_mode: CommitMode,
    /// « Compte préféré »: the Claude account its new agents and its tickets go to (an
    /// `Account`'s id); empty is « Automatique » (`accounts::pick`).
    #[serde(default)]
    pub account: String,
    /// « Créer le worktree au premier message, au nom de l’agent » (ESC-156): an ordinary new
    /// agent gets its worktree once its first message named it, its branch and its folder under
    /// that name. Off: at once, as `agent-N`, as before. On by default, for the projects saved
    /// before it too.
    #[serde(default = "crate::model::yes")]
    pub worktree_at_first_message: bool,
    /// « Branche de départ des nouveaux worktrees » (ESC-160): where a new agent's worktree starts.
    /// It took the place of `BoardSettings::agents_from_target` (`settle_legacy`).
    #[serde(default)]
    pub worktree_base: WorktreeBase,
    /// « Dossier des worktrees »: where its new worktrees go, from its folder when relative
    /// (`../app-worktrees`, `.worktrees`) or absolute; empty is `.claude/worktrees`
    /// (`paths::worktrees_root`). Those made already stay where they are.
    #[serde(default)]
    pub worktree_dir: String,
    /// « Préfixe des branches des agents »: a new agent's branch is `<prefix><name>`.
    #[serde(default = "default_agent_branch_prefix")]
    pub agent_branch_prefix: String,
    /// « Préfixe des branches des tickets »: a ticket's own branch is `<prefix><key>`.
    #[serde(default = "default_ticket_branch_prefix")]
    pub ticket_branch_prefix: String,
    /// « Remote control pour les agents de ce projet »: its agents, those of tickets too, are
    /// reachable from claude.ai (`RemoteSource::Project`).
    #[serde(default)]
    pub remote_control: bool,
    /// « Harness par défaut »: the one its new agents and its tickets' run on, unless they say
    /// theirs. None: Claude Code (and for a project saved before 1.8).
    #[serde(default)]
    pub engine: Option<EngineId>,
    /// « Projet privé »: through the MCP server, the agents of the other projects do not know it,
    /// and its own agents know it alone (`mcp::resolve::Sight`). Claude outside Escouade and the
    /// window see every project.
    #[serde(default)]
    pub private: bool,
    /// « Branche des docs » (ESC-256): where `.escouade/spec`, `decisions` and `documents` live. Empty: the
    /// branch checked out (a ticket’s agent writes them on its own branch); a name: that branch, in a
    /// worktree of its own that Escouade holds and never merges anywhere (`docsbranch`).
    #[serde(default)]
    pub docs_branch: String,
    /// « Comptes partagés à la limite » (a private project, ESC-141): its agents may go on on an
    /// account it shares with other projects when its own reach the usage limit. Off: they wait
    /// for the reset (`accounts::confined`).
    #[serde(default)]
    pub shared_on_limit: bool,
    /// The memory Claude Code kept for this project was offered to its shared memory (ESC-255): once,
    /// taken or not. Set by `claude_memory_take` and `claude_memory_dismiss` alone.
    #[serde(default)]
    pub claude_memory_asked: bool,
    /// Its `.escouade.yml` (ESC-146): the keys kept on this machine, the commands trusted, the file
    /// as last read.
    #[serde(default)]
    pub shared: crate::projectfile::ProjectShared,
    /// Its repositories (ESC-169), from its folder with `/` (`repos::detect`): empty for a project
    /// of one repository (or none), as before; the root's own as `""`, first, when it holds others.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repos: Vec<String>,
    /// Those found once and gone since (« retiré »), until the user forgets them: the worktrees
    /// agents have of them stay until the cleaning.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repos_gone: Vec<String>,
    /// The user's corrections of its workflows (ESC-195): ahead of what the reading finds. Kept in
    /// the files of its workflows (`workflowfiles`), read from them; here, the window's copy.
    #[serde(default)]
    pub workflow_fixes: crate::workflowfix::Fixes,
    /// « Où garder la configuration des workflows » (ESC-195): in the repository,
    /// `.escouade/workflows/`, for the team (by default), or on this machine alone.
    #[serde(default = "crate::model::yes")]
    pub workflows_in_repo: bool,
    /// « Relire quand les fichiers changent » (ESC-195): its workflows read again when the window
    /// comes back to the front and a file read changed. Off: by « Relire » alone.
    #[serde(default = "crate::model::yes")]
    pub workflows_auto_read: bool,
    /// « Pas de workflow pour ce projet » (ESC-309) when off: its workflows are neither read nor
    /// written, their files left as they are, nothing followed. In its `.escouade.yml` when shared
    /// (`workflows.enabled`).
    #[serde(default = "crate::model::yes")]
    pub workflows_enabled: bool,
    /// « Index du code » (ESC-251): the project's code cut into symbols and links, in a database
    /// of this machine (`codeindex`), for the agents to find code with for less; off, nothing is
    /// read and the database is erased. The project's own choice (ESC-293), on or off; None
    /// follows « Index du code sur tous les projets » of the app's settings. Read through
    /// `Core::code_index_on` alone.
    #[serde(
        default,
        rename = "codeIndexChoice",
        skip_serializing_if = "Option::is_none"
    )]
    pub code_index: Option<bool>,
    /// The switch saved before ESC-293 (`codeIndex`), read once by `settle_legacy`, never
    /// written again.
    #[serde(default, rename = "codeIndex", skip_serializing)]
    pub code_index_before: bool,
}

impl Project {
    /// The harness of its new agents and tickets that do not say theirs.
    pub fn engine(&self) -> EngineId {
        self.engine.unwrap_or_default()
    }

    /// A project of several repositories (ESC-169).
    pub fn is_multi(&self) -> bool {
        !self.repos.is_empty()
    }

    /// Its checkouts in its own folder: each repository, or the folder itself.
    pub fn checkouts(&self) -> Vec<Checkout> {
        if self.repos.is_empty() {
            return vec![Checkout {
                repo: String::new(),
                dir: self.path.clone(),
                base: String::new(),
            }];
        }
        self.repos
            .iter()
            .map(|r| Checkout {
                repo: r.clone(),
                dir: crate::repos::join(&self.path, r),
                base: String::new(),
            })
            .collect()
    }

    /// The folder its new worktrees go to.
    pub fn worktrees_root(&self) -> std::path::PathBuf {
        crate::paths::worktrees_root(&self.path, &self.worktree_dir)
    }

    /// A project saved before ESC-160 with « Les agents partent de la branche cible » on starts its
    /// new worktrees from the board's target: the choice is kept as « Branche de départ des
    /// nouveaux worktrees », the old switch never saved again.
    ///
    /// « Index du code » saved before ESC-293: on stays a choice of the project's; off follows the
    /// app's setting, nothing having told the off by default from the off wanted.
    pub fn settle_legacy(&mut self) {
        if std::mem::take(&mut self.board.agents_from_target)
            && self.worktree_base == WorktreeBase::Head
        {
            self.worktree_base = WorktreeBase::Target;
        }
        if std::mem::take(&mut self.code_index_before) && self.code_index.is_none() {
            self.code_index = Some(true);
        }
    }

    /// Whether its index is on: its own choice, or the app's « Index du code sur tous les
    /// projets » (`all`) when it made none. Read through `Core::code_index_on`.
    pub fn code_index_on(&self, all: bool) -> bool {
        self.code_index.unwrap_or(all)
    }
}

/// « Branche de départ des nouveaux worktrees » (ESC-160): where a new agent's worktree starts,
/// and so its base (what « Intégrer », its merge and its pull request go to).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "kind", content = "branch", rename_all = "lowercase")]
pub enum WorktreeBase {
    /// « Branche actuelle du dossier »: the branch the project's folder is on (HEAD).
    #[default]
    Head,
    /// « Branche cible du Kanban ».
    Target,
    /// A branch named: a local one, or a remote one (`origin/x`).
    Branch(String),
}

impl WorktreeBase {
    /// As `.escouade.yml` writes it: `head`, `target`, or the branch's name (`refs/heads/<name>`
    /// for a branch named `head` or `target`).
    pub fn to_text(&self) -> String {
        match self {
            Self::Head => "head".into(),
            Self::Target => "target".into(),
            Self::Branch(b) if b == "head" || b == "target" => format!("refs/heads/{b}"),
            Self::Branch(b) => b.clone(),
        }
    }

    /// From what `to_text` writes; empty is `head`.
    pub fn from_text(text: &str) -> Self {
        match text.trim() {
            "" | "head" => Self::Head,
            "target" => Self::Target,
            b => Self::Branch(b.strip_prefix("refs/heads/").unwrap_or(b).to_string()),
        }
    }
}

pub fn default_agent_branch_prefix() -> String {
    crate::paths::AGENT_BRANCH_PREFIX.into()
}

pub fn default_ticket_branch_prefix() -> String {
    crate::paths::TICKET_BRANCH_PREFIX.into()
}

/// Who writes a project's commits asked from the files panel.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommitMode {
    /// "Rédigé par l'agent": the agent is asked to commit its changes.
    #[default]
    Agent,
    /// "Direct, avec un message proposé": the app commits, with the message the user read (Haiku
    /// proposes one).
    Direct,
}

/// What a direct commit takes: the files it commits, and the files copied into the worktrees
/// (`.env`…) among the changes, which it never commits.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitScope {
    pub files: Vec<FileChange>,
    pub left_out: Vec<String>,
}

/// An external ticket system.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    #[default]
    Jira,
    Trello,
    Github,
    Linear,
    Clickup,
    /// GitLab Issues: its account is the GitLab code host's (`forge::accounts`).
    Gitlab,
    /// Azure Boards: its account is the Azure DevOps code host's (`forge::accounts`).
    Azure,
}

impl Service {
    pub const ALL: [Service; 7] = [
        Service::Jira,
        Service::Trello,
        Service::Github,
        Service::Linear,
        Service::Clickup,
        Service::Gitlab,
        Service::Azure,
    ];

    /// Its name, as the window and the comments say it.
    pub fn label(self) -> &'static str {
        match self {
            Service::Jira => "Jira",
            Service::Trello => "Trello",
            Service::Github => "GitHub",
            Service::Linear => "Linear",
            Service::Clickup => "ClickUp",
            Service::Gitlab => "GitLab",
            Service::Azure => "Azure Boards",
        }
    }
}

/// A state of an external ticket: a Jira status, a Trello list, or for GitHub "open", "closed"
/// or "label:<name>".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalState {
    pub id: String,
    pub name: String,
}

/// A project's source of tickets in an external system: a Jira project, a Trello board, a GitHub
/// repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceLink {
    pub service: Service,
    /// The Jira project's key, the Trello board's id, `owner/repo`.
    pub container: String,
    /// As the window shows it.
    pub name: String,
    /// The state an imported ticket's external one is given when it comes into each column
    /// (none: left as it is).
    pub states: BTreeMap<Column, ExternalState>,
}

/// What a project's tickets have to do with external ticket systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectIntegrations {
    /// One per service at most.
    pub links: Vec<SourceLink>,
    /// The columns whose arrival writes a comment on the external ticket.
    pub comments: Vec<Column>,
    /// Every external ticket imported into the project (`<service>|<id>`), even those deleted
    /// since: the automatic import never brings one back.
    pub imported: Vec<String>,
}

impl Default for ProjectIntegrations {
    fn default() -> Self {
        Self {
            links: Vec::new(),
            comments: vec![Column::Review, Column::Done],
            imported: Vec::new(),
        }
    }
}

impl ProjectIntegrations {
    pub fn link(&self, service: Service) -> Option<&SourceLink> {
        self.links.iter().find(|l| l.service == service)
    }
}

/// Where an imported ticket comes from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalRef {
    pub service: Service,
    /// What its API knows it by: the Jira issue's key, the Trello card's id, the GitHub issue's
    /// number.
    pub id: String,
    /// As it is shown: ATL-1287, #142, #42.
    pub key: String,
    /// Its Jira project, Trello board or GitHub repository (`SourceLink::container`).
    pub container: String,
    pub url: String,
    /// Why its last sync failed, until one succeeds.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RunCommand {
    pub id: String,
    pub name: String,
    pub command: String,
    /// Shell id ("pwsh", "powershell", "bash", "wsl").
    pub shell: String,
    /// Folder relative to the project's, empty for the project itself.
    pub cwd: String,
}

/// A command run in a worktree once it is made (its setup) or before it is removed (its
/// teardown).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WorktreeStep {
    pub id: String,
    pub command: String,
    /// Shell id ("pwsh", "powershell", "bash", "wsl"…), the system's default one when not found.
    pub shell: String,
    /// Folder relative to the worktree's, empty for the worktree itself.
    pub cwd: String,
}

/// What a new worktree gets from the project when nothing was set: its `.env` files.
/// `true`, for a setting on by default.
pub fn yes() -> bool {
    true
}

pub fn default_worktree_copy() -> Vec<String> {
    vec![".env*".into()]
}

/// Where a ticket stands on the board.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    /// « À faire »
    #[default]
    Todo,
    /// « En cours »
    Doing,
    /// « À tester »
    Review,
    /// « Terminé »
    Done,
}

/// An acceptance criterion, with what the agent last said of it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Criterion {
    pub text: String,
    pub ok: bool,
    pub note: String,
}

/// A ticket of a project's board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Ticket {
    pub id: String,
    pub project_id: String,
    /// `<PRÉFIXE>-<n>`, e.g. ATL-42.
    pub key: String,
    pub title: String,
    pub description: String,
    pub criteria: Vec<Criterion>,
    /// The features in place so far, as its agent last listed them (1 to 8 short lines); the
    /// agent writes it `avancement`.
    #[serde(alias = "avancement")]
    pub progress: Vec<String>,
    /// 3, 5 or 8.
    pub max_loops: u32,
    pub column: Column,
    /// Order in "À faire": the lowest first.
    pub rank: i64,
    /// The tickets of its project it comes after ("Après"), by id: the autopilot starts it once
    /// they are all "Terminé". Empty for a ticket saved before.
    pub after: Vec<String>,
    pub agent_id: Option<String>,
    /// n of "Boucle n/max" (0 before the start).
    pub iteration: u32,
    /// Every loop its agent began, sent back or not ("Renvoyer", failed tests, a conflict handed
    /// over start again at 1): what "n boucles" says once it is done. 0 for a ticket saved before
    /// they were counted.
    pub loops: u32,
    /// Sent to "À tester" without all its criteria (loop limit).
    pub partial: bool,
    /// Why it no longer moves on by itself.
    pub blocked: Option<String>,
    /// The block is a merge conflict: the card offers "L'agent résout" and "Annuler".
    pub conflict: bool,
    /// The validation step running ("Tests…", "Commit…", "Merge…", "Push…").
    pub step: Option<String>,
    /// What its validation did ("⤵ Mergé dans main · squash"…), and its link.
    pub outcome: Option<String>,
    /// The same, in the language of the texts for Claude: what the comment published for the team
    /// quotes, for it not to mix two languages. None for a ticket done before 1.7 (its `outcome`
    /// is quoted then).
    pub outcome_claude: Option<String>,
    pub outcome_url: Option<String>,
    /// The commit its merge made on the target (the merge commit, the squash commit, or the last
    /// commit rebased); None when it was not merged (pull request, push, kept, nothing to merge)
    /// or before 1.8.
    pub outcome_commit: Option<String>,
    /// "Lancer" was clicked while the autopilot is off.
    pub forced: bool,
    /// Its agent was reminded once to end with its report.
    pub reminded: bool,
    /// What its agents cost together (archived ones included), once done.
    pub cost: f64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub review_at: Option<i64>,
    pub done_at: Option<i64>,
    /// Imported from an external ticket system: the ticket there, kept in step.
    pub external: Option<ExternalRef>,
    /// An existing branch the ticket's agent takes up (« Reprendre une branche existante… »), a
    /// local one or a remote one (`origin/feat`); empty: its own branch, `ticket/<key>`, made at
    /// its start.
    pub branch: String,
    /// The model and the effort of this ticket's agent alone (« Modèle » and « Effort » in its
    /// form, or `start_ticket`); empty: « Auto », the board's, else the account's.
    pub model: String,
    pub effort: String,
    /// The account its agents start on (« Compte » in its form, ESC-153), by id: they stay on it,
    /// at its limit as past « Changer de compte à ». Empty: « Auto », the current account of its
    /// harness; so for a ticket saved before.
    pub account: String,
    /// Why it went back to « Auto » (its account removed, switched off or signed out), as its card
    /// says it until the ticket is saved again.
    pub account_lost: Option<String>,
    /// The harness of its agents (« Harness » in its form), which the autopilot starts them on;
    /// None: the project's (`Project::engine`), and for a ticket saved before 1.8.
    pub engine: Option<EngineId>,
    /// Its agent was set apart from Remote Control by hand (`AgentMeta::remote_off`): so is the
    /// next one it gets (sent back to do and started again), until the user turns Remote Control
    /// on for one by hand.
    pub remote_off: bool,
    /// The images pasted or dropped in its form, in their order: files of its folder
    /// (`DataDir::ticket_dir`), sent to its agent with its first message.
    pub images: Vec<TicketImage>,
    /// The files other than images joined in its form, in their order: copies in its `files`
    /// folder (`DataDir::ticket_files`), whose paths its agent is given with its first message.
    /// Always in the JSON, empty for a ticket saved before.
    pub files: Vec<TicketFile>,
    /// Shared with the team (ESC-190): its file `.escouade/tickets/<id>.yml` in the project's
    /// folder, written by Escouade and committed by the user (`sharing`). None: this machine's
    /// alone, and for a ticket saved before.
    pub shared: Option<Shared>,
    /// The pages of the spec it implements (`subject/page`), as « Planifier » made it (ESC-262);
    /// each of them names it in its `tickets`. Empty for a ticket made otherwise.
    pub pages: Vec<String>,
    /// The decisions it implements, the same way: by number, or by id while they are proposals.
    pub decisions: Vec<crate::docs::Ref>,
}

/// Someone of the team, as the files of the shared tickets name them (ESC-190): the name they
/// gave, and the id that tells two of the same name apart (`sharing::person_id`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Person {
    pub name: String,
    pub id: String,
}

/// An image or a file of a shared ticket, as its file lists it: its name, and where it is in the
/// ticket's folder beside the file (`.escouade/tickets/<id>/<file>`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SharedMedia {
    pub name: String,
    pub file: String,
}

/// The ticket of an outside service a shared ticket was imported from, as its file says it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SharedExternal {
    /// `jira`, `trello`, `github`, `linear` or `clickup` (`Service`).
    pub source: String,
    pub key: String,
    pub url: String,
}

/// What a shared ticket's file says beside the ticket itself, and how it stands with its file
/// (`Ticket::shared`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Shared {
    /// Its file's name, without `.yml`: stable, never its key.
    pub id: String,
    /// Its key as its author gave it (the ticket's own may have `@author` after it, when this
    /// machine has a ticket of that key already).
    pub key: String,
    pub author: Person,
    /// Who took it (started it): None while nobody did.
    pub taken_by: Option<Person>,
    /// Who last changed its file.
    pub updated_by: Option<Person>,
    /// When it was made, as its file writes it.
    pub created: String,
    /// The tickets it comes after, by the ids of their files (some may not be on this machine).
    pub after: Vec<String>,
    /// The ticket of an outside service it was imported from, as its file says it.
    pub external: Option<SharedExternal>,
    /// Its author is this machine's user (`Person::id`): only then may the autopilot start it.
    pub mine: bool,
    /// The fingerprint of its file as last read or written: one that differs was changed outside
    /// Escouade (a pull, a checkout, a hand).
    pub seen: String,
    /// Why its file is not read (a git conflict, a file that cannot be read): the ticket keeps its
    /// last state, and its file is not written until the user settles it.
    pub problem: Option<String>,
    /// The problem is a git conflict: « Garder le mien », « Prendre le sien ».
    pub conflict: bool,
    /// Written by a newer Escouade: read for what it knows, never written.
    pub newer: bool,
    /// Its file is gone from the folder (another branch, deleted by a teammate): not written again
    /// until it comes back, or the ticket is shared again.
    pub gone: bool,
    /// Its author stopped sharing it while this machine's agent had it: it stays, as one of ours.
    pub left: bool,
    /// « Masquer chez moi »: not on this board, its file kept.
    pub hidden: bool,
}

/// A file of a ticket that is no image the chat reads: a copy in the ticket's `files` folder
/// (`ticketfiles`), never in the JSON of the tickets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TicketFile {
    /// Its name, as shown and as its agent is told it.
    pub name: String,
    /// Where it is under the ticket's `files` folder: `<uniq>/<name>`.
    pub file: String,
    /// Its size, in bytes, when it was joined.
    pub size: u64,
}

/// An image of a ticket: its content is a file of the ticket's folder, never in the JSON of the
/// tickets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TicketImage {
    /// As it was pasted or dropped (« image.png »).
    pub name: String,
    /// One of the image types the API reads.
    pub media_type: String,
    /// Its file's name in the ticket's folder.
    pub file: String,
}

/// A project's board: what validating a ticket does, and its agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct BoardSettings {
    /// "merge", "pr", "push" or "keep".
    pub action: String,
    /// Where tickets merge, the base of their PR and of their branch. Empty until the first
    /// ticket, which sets the project's current branch.
    pub target: String,
    /// « Les agents partent de la branche cible », before ESC-160: read from the projects saved
    /// then, never written (`Project::settle_legacy` makes it `Project::worktree_base`).
    #[serde(skip_serializing)]
    pub agents_from_target: bool,
    /// "merge" (merge commit), "squash" or "rebase".
    pub strategy: String,
    pub draft: bool,
    pub tests_first: bool,
    pub test_command: String,
    pub cleanup: bool,
    /// Commit messages written by Haiku in the Conventional Commits form.
    pub conventional: bool,
    /// "ask", "agent" or "abort".
    pub conflict: String,
    /// Tickets "En cours" at once, 1 to 6.
    pub max_parallel: u32,
    /// The ticket agents' model, effort and mode; empty: the app's defaults.
    pub model: String,
    pub effort: String,
    pub mode: String,
    pub autopilot: bool,
    /// The tickets' key prefix, fixed by the first ticket.
    pub prefix: String,
    /// The next ticket's number (never reused).
    pub next_number: u32,
}

impl Default for BoardSettings {
    fn default() -> Self {
        Self {
            action: "merge".into(),
            target: String::new(),
            agents_from_target: false,
            strategy: "squash".into(),
            draft: false,
            tests_first: false,
            test_command: String::new(),
            cleanup: true,
            conventional: true,
            conflict: "ask".into(),
            max_parallel: 2,
            model: String::new(),
            effort: String::new(),
            mode: String::new(),
            autopilot: true,
            prefix: String::new(),
            next_number: 1,
        }
    }
}

impl BoardSettings {
    /// Whether the tests run again before a ticket is approved: tests first is on, with a test
    /// command to run (one that is empty is no command: the window shows tests first off, and the
    /// validation skips it).
    pub fn tests_first_on(&self) -> bool {
        self.tests_first && !self.test_command.trim().is_empty()
    }
}

/// A step of a test launch's preparation (`npm install`…), in a folder of the worktree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecipeStep {
    #[serde(alias = "commande", deserialize_with = "null_default")]
    pub command: String,
    #[serde(alias = "dossier", deserialize_with = "null_default")]
    pub dir: String,
}

/// A process of a test launch (a dev server…) and the address that answers once it is ready.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecipeProcess {
    #[serde(alias = "nom", deserialize_with = "null_default")]
    pub name: String,
    #[serde(alias = "commande", deserialize_with = "null_default")]
    pub command: String,
    #[serde(alias = "dossier", deserialize_with = "null_default")]
    pub dir: String,
    #[serde(deserialize_with = "env_strings")]
    pub env: BTreeMap<String, String>,
    #[serde(deserialize_with = "null_default")]
    pub url: String,
}

/// How to launch a worktree for a test, as its agent wrote it (key `lancement` of its report).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TestRecipe {
    #[serde(alias = "preparation", deserialize_with = "null_default")]
    pub prepare: Vec<RecipeStep>,
    #[serde(alias = "processus", deserialize_with = "null_default")]
    pub processes: Vec<RecipeProcess>,
    /// The address that shows the feature itself; else the first process's url.
    #[serde(alias = "ouvrir", deserialize_with = "null_default")]
    pub open: String,
}

/// A `null` where the agent had nothing to say is the field's default (empty), not an unreadable
/// recipe.
fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// Environment values as strings, whatever the agent wrote (`"PORT": 4110`); a null `env` is an
/// empty one and a null value is skipped.
fn env_strings<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    let raw: Option<BTreeMap<String, Value>> = Option::deserialize(d)?;
    Ok(raw
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| match v {
            Value::Null => None,
            Value::String(s) => Some((k, s)),
            other => Some((k, other.to_string())),
        })
        .collect())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    #[default]
    Idle,
    Running,
    Waiting,
    Done,
    Error,
}

impl AgentStatus {
    pub fn is_active(self) -> bool {
        matches!(self, AgentStatus::Running | AgentStatus::Waiting)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    pub branch: String,
    pub base_branch: String,
    /// The branch was there before the agent (« Nouvel agent sur une branche… », a ticket that takes
    /// one up): it is the user's, so deleting the agent's worktree leaves it, and Haiku never
    /// renames it after the agent.
    #[serde(default)]
    pub existing: bool,
    /// The branch was made for the agent under the name the user typed (« Nouvel agent sur une
    /// branche… », ESC-155): the agent's, it goes with its worktree, but Haiku never renames it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub named_branch: bool,
    /// Of a project of several repositories (ESC-169): `path` is the agent's folder, and these the
    /// repositories it has a worktree of, each at `path/<repo>` on `branch`. Empty: `path` is the
    /// worktree, as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repos: Vec<RepoWorktree>,
}

/// The worktree of one repository in an agent's folder (`Worktree::repos`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoWorktree {
    /// From the project's folder, with `/`; `""` for the root's own.
    pub repo: String,
    /// What it started from, and what its merge and its pull request go to.
    pub base_branch: String,
}

/// One checkout of some code: a repository's folder or worktree, or the only one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkout {
    /// From the project's folder; `""` for a project of one repository (and the root's own).
    pub repo: String,
    pub dir: String,
    /// The branch it goes to (an agent's worktree); empty for the project's own folders.
    pub base: String,
}

impl Worktree {
    /// Its checkouts: each repository's worktree, or itself.
    pub fn checkouts(&self) -> Vec<Checkout> {
        if self.repos.is_empty() {
            return vec![Checkout {
                repo: String::new(),
                dir: self.path.clone(),
                base: self.base_branch.clone(),
            }];
        }
        self.repos
            .iter()
            .map(|r| Checkout {
                repo: r.repo.clone(),
                dir: crate::repos::join(&self.path, &r.repo),
                base: r.base_branch.clone(),
            })
            .collect()
    }

    pub fn is_multi(&self) -> bool {
        !self.repos.is_empty()
    }
}

/// A worktree of the project's repository that an agent without one of Escouade's went into by
/// itself (ESC-161: `EnterWorktree`, a `cd`), as its session's latest entry says: its branch is the
/// one its pull requests are read on, and its header shows. Never the project's folder, nor
/// another agent's worktree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FoundWorktree {
    pub path: String,
    pub branch: String,
}

/// A harness that runs agents (`engine::Engine`), as the data keeps it: an agent's
/// (`AgentMeta::engine`), a ticket's (`Ticket::engine`), a project's default (`Project::engine`),
/// a turn's (`TurnRow::engine`, the `engine` column of the statistics). Claude Code for what was
/// saved before 1.8, when it was the only one.
///
/// An open id (INT-3, docs/PLUGINS.md §4.7): the harnesses written are in the registry
/// (`engine::registry::all()`), and the data may name one this version does not have (a later
/// version's, a plugin's gone), which it keeps as it is: its agents are « harness absent »
/// (`engine::by_id`). Kept as the string it is (`claude`, `codex`…), as when it was an enumeration:
/// nothing to migrate. Its text is interned, so that it stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct EngineId(&'static str);

impl EngineId {
    pub const CLAUDE: EngineId = EngineId("claude");
    pub const CODEX: EngineId = EngineId("codex");
    pub const COPILOT: EngineId = EngineId("copilot");
    pub const CURSOR: EngineId = EngineId("cursor");
    pub const OPENCODE: EngineId = EngineId("opencode");
    /// Mistral Vibe (ESC-266).
    pub const VIBE: EngineId = EngineId("vibe");
    /// Antigravity CLI (ESC-270).
    pub const AGY: EngineId = EngineId("agy");
    /// Gemini CLI (ESC-274).
    pub const GEMINI: EngineId = EngineId("gemini");
    /// The minimal fake engine of the conformance bench, run by the core in the tests of the
    /// handover (`engine/minimal/`): a second harness beside Claude Code's fake.
    #[cfg(test)]
    pub const BENCH: EngineId = EngineId("bench");

    /// The id `text` names, whether a harness of this version bears it or not.
    pub fn new(text: &str) -> EngineId {
        use parking_lot::Mutex;
        use std::collections::HashSet;
        static SEEN: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
        let mut seen = SEEN.lock();
        let seen = seen.get_or_insert_with(HashSet::new);
        if let Some(s) = seen.get(text) {
            return EngineId(s);
        }
        let s: &'static str = Box::leak(text.to_owned().into_boxed_str());
        seen.insert(s);
        EngineId(s)
    }

    /// As the data says it (`claude`, `opencode`…).
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl Default for EngineId {
    fn default() -> Self {
        EngineId::CLAUDE
    }
}

impl<'de> Deserialize<'de> for EngineId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = std::borrow::Cow::<'de, str>::deserialize(d)?;
        if text.is_empty() {
            return Err(serde::de::Error::custom("an empty harness id"));
        }
        Ok(EngineId::new(&text))
    }
}

impl std::fmt::Display for EngineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where an agent's Remote Control comes from (`AgentMeta::remote_source`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteSource {
    /// Turned on for this agent, from its menu.
    Hand,
    /// The project's setting.
    Project,
    /// The global setting.
    Global,
}

/// The longest status line an agent reports (`AgentMeta::progress_line`), in characters.
pub const PROGRESS_LINE_MAX: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentMeta {
    pub id: String,
    pub project_id: String,
    pub name: String,
    /// True once the name was generated or chosen by the user.
    pub named: bool,
    /// The harness it runs on, one at a time: Claude Code for an agent saved before 1.8. It
    /// changes only by a handover to another one; each turn keeps the one that ran it
    /// (`TurnRow::engine`), and a copy takes the one it has when copied.
    pub engine: EngineId,
    pub model: String,
    pub effort: String,
    /// Changed by `set_mode`, which keeps `plan_from`.
    pub mode: String,
    /// The mode its work resumes in once a plan is approved (ESC-277): the one it was in when it
    /// went into Plan, or its mode of departure when it started in it. None: the app's, `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_from: Option<String>,
    pub session_id: Option<String>,
    /// The latest entry of its session's main chain (the uuid of its latest assistant message),
    /// saved with the end of each turn: where a copy made now forks its session.
    pub last_entry: Option<String>,
    /// A copy of another agent (« Dupliquer la conversation »): the original's session, which
    /// its starts fork (`--resume <it> --fork-session`) until a turn gives it one of its own.
    pub fork_of: Option<String>,
    /// The original's latest entry when it was copied: the copy's starts fork its session there
    /// (`--resume-session-at`), the turns it ran since left out. None: the whole session.
    pub fork_at: Option<String>,
    pub cwd: String,
    pub worktree: Option<Worktree>,
    /// Its worktree waits for its first message (ESC-156, « Créer le worktree au premier message,
    /// au nom de l’agent »): named first, it gets its branch and its folder under that name, then
    /// its setup, and its first turn goes. It works nowhere until then (`cwd` is the project’s).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub worktree_pending: bool,
    /// Without a worktree of its own: the one it went into by itself, if any (ESC-161).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub found_worktree: Option<FoundWorktree>,
    /// With `worktree_pending`: the branch its worktree is to start from, chosen at its making
    /// (« Nouvel agent depuis une autre branche… », ESC-160). None: the project's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_from: Option<String>,
    pub created_at: i64,
    pub archived: bool,
    /// Why its worktree, its ticket over, was kept (`leftovers`): work nothing else keeps. None once
    /// it went, or when the agent is restored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_kept: Option<String>,
    pub status: AgentStatus,
    pub tokens: u64,
    pub cost: f64,
    pub active_ms: u64,
    /// Files edited by the agent, relative to its cwd with forward slashes.
    pub touched_files: Vec<String>,
    pub last_activity: i64,
    pub prompts: u32,
    /// Remote Control in force now, whatever it comes from (`remote_source`): the session is also
    /// reachable from claude.ai / the Claude app, so the agent's process stays up (started with
    /// the app, never idle-stopped).
    pub remote_control: bool,
    /// Where the Remote Control in force comes from: the user's choice for this agent, its
    /// project's setting or the global one. None while it is not in force.
    pub remote_source: Option<RemoteSource>,
    /// Set apart by hand (« Désactiver le remote control », whether a setting covered it then or
    /// not): the settings give it none, restarts included, until the user turns it on for this
    /// agent. Its copies and, for a ticket's agent, the ticket's next agents are set apart too.
    pub remote_off: bool,
    /// Remote session to reattach to when the process restarts, and its claude.ai link.
    pub remote_session: Option<String>,
    pub remote_url: Option<String>,
    /// Stopped by the usage limit: when the agent is sent "continue" by itself (quota reset).
    pub resume_at: Option<i64>,
    /// The board's ticket this agent works on.
    pub ticket_id: Option<String>,
    /// What the agent last said it does (`report_progress`, a tool of Escouade's MCP server), on
    /// one line of `PROGRESS_LINE_MAX` characters at most: shown on its card and its ticket's,
    /// until its next call.
    pub progress_line: Option<String>,
    /// Its pull requests (ESC-64), as its code host last told them: those whose branch is its
    /// worktree's, whoever opened them, a fork's branch of the same name left out. Shown on its
    /// card and in its conversation's header.
    pub prs: Vec<crate::forge::PullRequest>,
    /// The code host's word for them: « MR » on GitLab, « PR » elsewhere. None until read.
    pub pr_word: Option<String>,
    /// Its worktree was switched to another branch by the user (the status bar's picker): the
    /// agent is told with the next message it gets, then this goes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_moved: Option<BranchMove>,
    /// Appended to Claude Code's system prompt at every start (the ticket's protocol), on one line.
    pub append_prompt: Option<String>,
    /// First of the 10 ports reserved for its test launches.
    pub port_base: Option<u16>,
    /// How to launch its worktree for a test, as it last wrote it.
    pub recipe: Option<TestRecipe>,
    /// The recipe the user read and let run (« Lancer » in the test modal), kept whole rather than
    /// hashed: a recipe that differs in anything, the agent's answer to a later request included,
    /// is asked again, with no collision to forge.
    pub approved_recipe: Option<TestRecipe>,
    /// What the user read and let run for an agent whose services isola runs (`isola up` runs the
    /// commands of its `.isola.toml`, which the agent can write): see `IsolaApproval`.
    pub approved_isola: Option<IsolaApproval>,
    /// The Claude account it runs on (an `Account`'s id), where its session is kept: Principal for
    /// an agent saved before there were accounts.
    #[serde(default = "principal_id")]
    pub account: String,
    /// How far it is through a plan: its task list, its subagents, its workflows, worked out from
    /// its stream (`plan`). None until it has any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanState>,
    /// The account it ran on before « Reprendre sur <compte> » moved it, until a turn ends on the
    /// new one: a turn that fails there is the resume's failure, and this is where it goes back to.
    pub moved_from: Option<String>,
    /// The resume it waited for on that account (`resume_at`), which the move dropped: where it waits
    /// again when it goes back, if the account's windows no longer say when it resets.
    pub moved_resume_at: Option<i64>,
    /// The harness it left by a handover (« Continuer sur <harness> »), until it goes back: set
    /// aside until its quota resets, then offered back (« Revenir sur <harness> »), never gone
    /// back to by itself.
    pub handed_from: Option<HandedFrom>,
    /// Stopped because its account's sign-in could not be refreshed (ESC-162): the account is to
    /// sign in again, and the agent goes on by itself once its sign-in changes.
    pub sign_in: Option<SignInWait>,
    /// « Vue Claude Code »: it runs in Claude Code's own interface, in a terminal (`tui`), not
    /// over the stream that Escouade's conversation view drives. Never a ticket's agent.
    pub terminal: bool,
    /// « Vue <harness> » of a harness whose interface joins the agent's process
    /// (`Engine::attaches`, OpenCode): the interface shows in a terminal (`AgentRt::attach`), while
    /// the process goes on as in Escouade's view. Never with `terminal`, nor for a ticket's agent.
    pub attached: bool,
    /// How far the transcript its interface writes was read: the file and the offset (bytes),
    /// so that what was written while the app was not reading it is read when it starts again.
    pub transcript_read: Option<(String, u64)>,
    /// « Vue Cursor » (`TerminalView::relay_path`): the conversation's last item when the agent
    /// went to its harness's interface ("" when there was none). Cursor keeps the conversations of
    /// its interface apart from its session's: what came after is told to the session on the way
    /// back (`relay`).
    pub relay_from: Option<String>,
    /// What was said in its harness's interface, still to be told to its session with its next
    /// message (`handover::relay`).
    pub relay: Option<String>,
    /// The folders Escouade made in the agent's folder for its relay (`.cursor`, `.cursor/rules`),
    /// outermost first: taken away with it once empty, never one that was there before.
    pub relay_made: Vec<String>,
    /// The files joined to its messages by their own path (no copy), as its Claude Code reads
    /// them: the only files outside its folders that its `Read` is allowed without asking
    /// (`attachfiles::granted`). The `MAX_GRANTED_FILES` last.
    pub granted_files: Vec<String>,
    /// A discussion of the Docs tab (ESC-261): its role, its subject and what it proposes for the
    /// spec. None for any other agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discussion: Option<crate::discussion::Discussion>,
}

/// The sign-in an agent waits for (`AgentMeta::sign_in`): its account's, another than `stamp`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SignInWait {
    pub account: String,
    /// The sign-in it stopped with (`usage::Stored::stamp`): None when there was none.
    pub stamp: Option<String>,
}

/// The harness an agent left by a handover (`AgentMeta::handed_from`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandedFrom {
    pub engine: EngineId,
    /// The account it ran on there.
    pub account: String,
    /// When the quota of that harness resets (ms): it is not gone back to before. None: it left
    /// with quota to spare (a handover asked for by hand).
    pub until: Option<i64>,
}

/// The most files of the user's own an agent keeps the right to read (`AgentMeta::granted_files`):
/// the oldest ones go past it, and are asked for again when read.
pub const MAX_GRANTED_FILES: usize = 200;

impl AgentMeta {
    /// Its mode is now `mode`, however it changed (the menu, its harness, Escouade): going into
    /// Plan, the mode it leaves is kept for the plan's approval (`plan_resume`). False when it was
    /// already in it.
    pub fn set_mode(&mut self, mode: &str) -> bool {
        if self.mode == mode {
            return false;
        }
        if mode == "plan" && !self.mode.is_empty() {
            self.plan_from = Some(self.mode.clone());
        }
        self.mode = mode.to_string();
        true
    }

    /// The mode its work resumes in when its plan is approved (« Approuver le plan »): the one it
    /// was in before Plan, else the app's mode of new agents (`auto`); its own when it is not in
    /// Plan; never `plan`, and `default` for `auto` on a model that has no auto mode (`takes_auto`).
    pub fn plan_resume(&self, takes_auto: bool) -> String {
        let mode = if !self.mode.is_empty() && self.mode != "plan" {
            self.mode.as_str()
        } else {
            self.plan_from
                .as_deref()
                .filter(|m| !m.is_empty() && *m != "plan")
                .unwrap_or("auto")
        };
        if mode == "auto" && !takes_auto {
            "default".into()
        } else {
            mode.into()
        }
    }

    /// Remembers that the user joined the file `claude_path` (as Claude Code reads it) by its own
    /// path: its `Read` of exactly that file is allowed from now on. Joined again, it only moves to
    /// the end; past `MAX_GRANTED_FILES` the oldest go.
    pub fn grant_file(&mut self, claude_path: String) {
        self.granted_files
            .retain(|f| !crate::attachfiles::same_file(f, &claude_path));
        self.granted_files.push(claude_path);
        let extra = self.granted_files.len().saturating_sub(MAX_GRANTED_FILES);
        self.granted_files.drain(..extra);
    }
}

fn principal_id() -> String {
    crate::accounts::PRINCIPAL.into()
}

/// What the user approved of an agent's isola launch: the content of the worktree's `.isola.toml`
/// (the services and setup commands `isola up` runs in their shell) and the address the agent gave to
/// open. Kept whole, like `approved_recipe`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IsolaApproval {
    pub config: String,
    pub open: String,
}

/// A request waiting for the user's answer, summed up as « Vue d'ensemble » shows it: each text
/// capped (`MAX_PENDING_FIELD` characters), the card in the conversation has it whole.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingView {
    /// The item's id in the conversation, which the answer names.
    pub id: String,
    /// "permission" or "question".
    pub kind: String,
    /// The tool asking for permission (AskUserQuestion for a question).
    pub tool: String,
    /// What the tool is asked to act on, as the conversation's card sums it up (the command,
    /// the file from the agent's folder…), its line breaks kept.
    pub arg: String,
    pub description: Option<String>,
    /// Why Claude Code asks (the permission rule it met).
    pub reason: Option<String>,
    /// A question's questions, with their options' labels.
    pub questions: Vec<PendingQuestion>,
    /// A text, a question or an option was cut: what is shown is not the whole request.
    pub cut: bool,
    /// Claude Code would refuse it by default (`default_to_no`): it is not allowed in one click.
    pub default_no: bool,
    /// A permission can be allowed for good (« Toujours autoriser ») or for the session.
    pub can_always: bool,
    /// What « always » stands for: "session" for the rest of the session (Codex, Copilot CLI),
    /// none for the rules the harness keeps (Claude Code, OpenCode's pattern, told in `description`).
    pub always_scope: Option<String>,
}

impl PendingView {
    /// The view with what « always » allows, as the conversation's card `item` offers it.
    pub fn always_as(mut self, item: &serde_json::Value) -> Self {
        self.can_always = item["canAlways"].as_bool().unwrap_or(false);
        self.always_scope = item["alwaysScope"].as_str().map(str::to_string);
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingQuestion {
    pub question: String,
    pub options: Vec<String>,
    /// Several options can be ticked.
    pub multi: bool,
}

/// The most characters a field of a `PendingView` carries.
pub const MAX_PENDING_FIELD: usize = 2000;
/// The most questions, and options of a question, a `PendingView` carries.
pub const MAX_PENDING_ITEMS: usize = 10;

/// Agent as shown by the UI: persisted metadata plus live runtime fields.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    #[serde(flatten)]
    pub meta: AgentMeta,
    pub active_since: Option<i64>,
    pub alive: bool,
    /// « Outils d'Escouade au prochain démarrage »: its live process has none of Escouade's tools,
    /// which its next one gets (« Claude et les agents peuvent piloter Escouade » was turned on
    /// since it started).
    pub escouade_next: bool,
    /// Ids of the question/permission items awaiting an answer, in the order they were asked.
    pub pending: Vec<String>,
    /// The same requests summed up, in the same order: what « Vue d'ensemble » shows of them
    /// without loading the conversation.
    pub requests: Vec<PendingView>,
    pub context_tokens: u64,
    /// Size of the context window of the conversation's model (0 until a turn told it).
    pub context_window: u64,
    /// Tokens of the running turn so far (not yet in `tokens`, which the turn's end updates).
    pub live_tokens: u64,
    /// Estimated cost of the running turn so far, from list prices (not yet in `cost`).
    pub live_cost: f64,
    /// Remote Control link state reported by Claude Code ("ready", "connected"…).
    pub remote_state: Option<String>,
    /// What it does right now ("Lit src/db.ts", "Lance npm test"…), during a turn.
    pub activity: Option<String>,
    /// The terminal of Claude Code's interface running for it (« Vue Claude Code »).
    pub terminal_id: Option<String>,
    /// Its interface runs (Claude Code view) and was started with Remote Control or not; None: no
    /// interface runs.
    pub interface_remote: Option<bool>,
    /// How the last interface it ran ended, until the next one starts (not saved).
    pub tui_exit: Option<TuiExit>,
    /// The setup of its new worktree under way: the command running ("npm ci (1/2)").
    pub setup: Option<String>,
    /// isola runs its worktree's services (installed, and an `.isola.toml` in the worktree): its
    /// test launches go through it, with no recipe nor reserved ports.
    pub isola: bool,
}

/// How Claude Code's interface of an agent ended (« Vue Claude Code »), as the window reads it to
/// decide whether to start it again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TuiExit {
    /// Its exit code; None when it was killed.
    pub code: Option<u32>,
    /// It stopped soon after it started, and not by the user's wish (`/exit`, code 0).
    pub quick: bool,
    /// Claude Code refused the session to resume (« No conversation found »), at once: the agent's
    /// session was given up, its next start is a new one.
    pub session_lost: bool,
}

/// How many of the last lines of the step running a worktree's setup keeps for the window.
pub const SETUP_LINES: usize = 500;

/// What the step of a worktree's setup under way wrote, as the window shows it (its whole output
/// goes to the agent's setup log).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct SetupOutput {
    /// The step's rank, from 0.
    pub step: usize,
    /// How many lines it wrote so far: the window takes the lines it is sent once.
    pub total: usize,
    /// Its last lines, `SETUP_LINES` at most.
    pub lines: std::collections::VecDeque<String>,
}

impl SetupOutput {
    /// Step `step`, before it wrote anything.
    pub fn new(step: usize) -> Self {
        Self {
            step,
            ..Self::default()
        }
    }

    /// `lines`, just written: the oldest beyond `SETUP_LINES` go.
    pub fn push(&mut self, lines: &[String]) {
        self.total += lines.len();
        let kept = &lines[lines.len().saturating_sub(SETUP_LINES)..];
        self.lines.extend(kept.iter().cloned());
        let excess = self.lines.len().saturating_sub(SETUP_LINES);
        self.lines.drain(..excess);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub active_project: Option<String>,
    pub view: String,
    pub selected_agent: HashMap<String, String>,
    /// "split" (conversation | files) or "" (classic).
    pub layout: String,
}

/// A choice of Claude Code's model picker (`initialize`): an alias or a full id, and the model
/// it runs (an alias follows the version of Claude Code).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelInfo {
    pub value: String,
    pub resolved_model: String,
    /// Whether it takes the permission mode `auto`, as Claude Code tells it (`supportsAutoMode`,
    /// absent for a model without it); None when it says nothing of it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_auto_mode: Option<bool>,
    /// Its name in Claude Code's `/model` (`Opus 5.5`, `Default (recommended)`), empty when it
    /// tells none (ESC-124).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub display_name: String,
    /// Whether it takes an effort (`supportsEffort`, absent for a model without one), and which
    /// (`supportedEffortLevels`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_effort: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub supported_effort_levels: Vec<String>,
}

/// What the `claude` of an account told of its models at its last reading (ESC-121, ESC-124): the
/// version that told it, its models with their mode `auto` (`engine::claude::told_auto`), and
/// the menu of its agents they make (`engine::claude::catalog`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountModels {
    /// Its version (`2.1.302`), empty when it was not read.
    pub version: String,
    pub models: Vec<ModelInfo>,
    /// The composer's menu of the account's agents, as `claude` told them: empty while it told
    /// none (a list read before ESC-124, which kept no names), when the fixed one serves.
    pub catalog: Vec<crate::engine::registry::ToldModel>,
    /// When they were last read (ms since the epoch), 0 when unknown: the menu of a Codex agent
    /// reads its account's again once they are a day old (ESC-186).
    pub read_at: i64,
}

impl AccountModels {
    /// What `claude` at `version` told of `models`, with the menu they make.
    pub fn new(version: String, models: Vec<ModelInfo>) -> Self {
        let catalog = crate::engine::claude_catalog(&models);
        AccountModels {
            version,
            models,
            catalog,
            read_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

impl ModelInfo {
    /// The models of an `initialize` response, skipping the entries that say no model.
    pub fn list(reported: &Value) -> Vec<Self> {
        reported
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| serde_json::from_value::<Self>(m.clone()).ok())
            .filter(|m| !m.value.is_empty() && !m.resolved_model.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistedState {
    pub projects: Vec<Project>,
    pub agents: Vec<AgentMeta>,
    pub ui: UiState,
    /// Claude Code's models as it last reported them, to label the aliases from the start.
    pub models: Vec<ModelInfo>,
    /// The models of the harnesses that list those of their account (Codex: `model/list`), as
    /// they last listed them: the composer's menu of their agents from the start.
    pub engine_models: BTreeMap<EngineId, Vec<crate::engine::registry::ToldModel>>,
    /// What the `claude` of each account (by its id) last told of its models: which take the mode
    /// `auto`, from the start.
    pub account_models: BTreeMap<String, AccountModels>,
    /// Every project's tickets.
    pub tickets: Vec<Ticket>,
    /// The autopilot's pause as the app stopped: the next start holds the tickets back the same.
    pub pause: SavedPause,
    /// The closed projects' histories (by project id) the state holds the truth of, whose file
    /// could not be removed: given back to their project, or erased. The next start tries again
    /// to remove them, and never reads them (`closed::scan`).
    pub leftover_histories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub is_repo: bool,
    /// "(detached)" for a detached HEAD.
    pub branch: String,
    /// The remote branch it tracks ("origin/main"), if any.
    pub upstream: Option<String>,
    /// The upstream no longer exists on the remote (deleted, e.g. once merged).
    pub upstream_gone: bool,
    /// Commits to push / to pull, against the upstream as last fetched.
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
    /// When the repository was last fetched, ms since epoch.
    pub last_fetch: Option<i64>,
    pub modified: u32,
    pub added: u32,
    pub deleted: u32,
    pub total: u32,
    /// Dirty file count attributed to each agent of the project.
    pub agents: HashMap<String, u32>,
    /// The checkout of each agent that works in a folder of its own (its worktree, or the one it
    /// went into by itself, ESC-161), as git tells it: what the status bar shows of the agent
    /// open. None for an agent in the project's folder: the fields above are its.
    pub agent_git: HashMap<String, AgentGit>,
    /// Of a project of several repositories (ESC-169), each one's: the fields above are then
    /// their sums, and `branch` theirs when they share it (empty otherwise).
    pub repos: Vec<RepoGit>,
}

/// An agent's worktree switched from one branch to another (`AgentMeta::branch_moved`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BranchMove {
    pub from: String,
    pub to: String,
}

/// The checkout of an agent's own folder (`GitInfo::agent_git`): the branch HEAD is really on,
/// whatever the agent's worktree was made on.
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentGit {
    /// "(detached)" for a detached HEAD. Of several repositories, theirs when they share it
    /// (empty otherwise).
    pub branch: String,
    pub upstream: Option<String>,
    pub upstream_gone: bool,
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
    /// Of an agent of several repositories (ESC-169), each one's: the fields above are then
    /// their sums.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub repos: Vec<RepoGit>,
}

/// The state of one repository of a project of several (`GitInfo::repos`).
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoGit {
    pub repo: String,
    /// "(detached)" for a detached HEAD; empty when the folder is gone.
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
    pub has_commit: bool,
    /// Its own changes (`modified + added + deleted`), the agents' worktrees apart.
    pub modified: u32,
    pub added: u32,
    pub deleted: u32,
    /// Its folder is no longer there (« retiré »).
    pub gone: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub status: String,
    pub add: u32,
    pub del: u32,
    pub agent_id: Option<String>,
    /// Listed from `agent_id`'s worktree rather than the project's repository.
    pub in_worktree: bool,
    /// Its repository, in a project of several (ESC-169): `path` starts with it.
    pub repo: String,
}

/// The unified diff of the files of the files panel that one agent owns in one checkout (the
/// project's own, or the agent's worktree); `agent_id` is `None` for files no agent is credited with.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedDiff {
    pub agent_id: Option<String>,
    /// Read from `agent_id`'s worktree rather than the project's repository.
    pub in_worktree: bool,
    pub diff: String,
}

/// One commit of the repository graph.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    /// Author date, Unix seconds.
    pub time: i64,
    /// Branches and tags pointing at it ("HEAD", "main", "origin/main", "tag: v1.0").
    pub refs: Vec<String>,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLog {
    pub commits: Vec<Commit>,
    /// The branch the agent works on (its worktree's, else the project's current branch).
    pub head: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RateWindow {
    /// 0-100.
    pub pct: f64,
    /// Epoch milliseconds.
    pub resets_at: Option<i64>,
}

/// The length of Claude Code's 5-hour window, in minutes (`QuotaMeter::minutes`).
pub const FIVE_HOURS: u32 = 300;
/// The length of a weekly window, in minutes.
pub const WEEK: u32 = 10_080;

/// One meter of an account's quota (docs/ENGINES.md, « Le quota »): a window of a length
/// (`minutes`: 300 for 5 hours, 10 080 for a week), used in percent until it starts again, or the
/// month's credits (`minutes`: none), used in percent until they are given again. What a harness
/// has of them is in its capabilities (`EngineCaps::quota`: windows, credits or none).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct QuotaMeter {
    /// The window's length; none for the month's credits.
    pub minutes: Option<u32>,
    /// 0-100.
    pub pct: f64,
    /// Epoch milliseconds.
    pub resets_at: Option<i64>,
}

impl QuotaMeter {
    /// The window of `minutes` as `w` reads it.
    pub fn window(minutes: u32, w: RateWindow) -> Self {
        Self {
            minutes: Some(minutes),
            pct: w.pct,
            resets_at: w.resets_at,
        }
    }

    /// The month's credits as `w` reads them (Copilot CLI).
    pub fn credits(w: RateWindow) -> Self {
        Self {
            minutes: None,
            pct: w.pct,
            resets_at: w.resets_at,
        }
    }

    /// Its use and its end.
    pub fn rate(&self) -> RateWindow {
        RateWindow {
            pct: self.pct,
            resets_at: self.resets_at,
        }
    }

    /// What tells it from the account's other meters: its length, `credits` for the credits.
    pub fn key(&self) -> String {
        self.minutes
            .map_or_else(|| "credits".to_string(), |m| m.to_string())
    }
}

/// `old` with what a reading found (`new`): each meter read takes the place of the one of the
/// same kind, the others stay as last read (they still say what was used until their end). The
/// shortest window first, the credits last.
pub fn merge_meters(old: &[QuotaMeter], new: &[QuotaMeter]) -> Vec<QuotaMeter> {
    let mut all: Vec<QuotaMeter> = old
        .iter()
        .filter(|m| !new.iter().any(|n| n.minutes == m.minutes))
        .chain(new)
        .copied()
        .collect();
    all.sort_by_key(|m| m.minutes.unwrap_or(u32::MAX));
    all
}

/// Claude Code's two windows (5 hours, week) as meters, those it read.
pub fn claude_meters(
    five_hour: Option<RateWindow>,
    seven_day: Option<RateWindow>,
) -> Vec<QuotaMeter> {
    [(FIVE_HOURS, five_hour), (WEEK, seven_day)]
        .into_iter()
        .filter_map(|(m, w)| w.map(|w| QuotaMeter::window(m, w)))
        .collect()
}

/// What holds the autopilot back: one of the quota's windows or its credits over « Pause
/// au-delà du quota », or the pause after a usage limit with no resume planned.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PauseReason {
    /// A window, of `AutopilotPause::minutes`.
    Window,
    /// The month's credits.
    Credits,
    Limit,
    /// The day's budget of an account of a harness without a quota (`Account::daily_budget`).
    Budget,
}

/// What holds the autopilot back on one account besides the quota windows read, or no longer does
/// (`board::autopilot_pause`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountHold {
    /// After a usage limit with no resume planned: until then.
    pub limit_until: Option<i64>,
    /// "Reprendre maintenant": the 5-hour and the weekly window hold nothing back until the end
    /// they had then (the next window holds again).
    pub lifted: [Option<i64>; 2],
    /// The same for the quota's other meters (`QuotaMeter::key`): windows of another length, the
    /// month's credits. Apart from `lifted`, which the versions before 1.8 read.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lifted_other: BTreeMap<String, i64>,
}

impl AccountHold {
    /// Until when "Reprendre maintenant" lifted the meter `m`.
    pub fn lifted_until(&self, m: &QuotaMeter) -> Option<i64> {
        match m.minutes {
            Some(FIVE_HOURS) => self.lifted[0],
            Some(WEEK) => self.lifted[1],
            _ => self.lifted_other.get(&m.key()).copied(),
        }
    }

    /// "Reprendre maintenant" lifted the meter `m` until `end`.
    pub fn lift(&mut self, m: &QuotaMeter, end: i64) {
        match m.minutes {
            Some(FIVE_HOURS) => self.lifted[0] = Some(end),
            Some(WEEK) => self.lifted[1] = Some(end),
            _ => {
                self.lifted_other.insert(m.key(), end);
            }
        }
    }
}

/// What holds the autopilot back besides the quota windows read, or no longer does, account by
/// account: a ticket may start on one account while another waits (`board::autopilot_pause`).
/// Principal's is where the versions before the accounts kept the only one.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Hold {
    /// Principal's: after a usage limit with no resume planned, until then.
    pub limit_until: Option<i64>,
    /// Principal's: "Reprendre maintenant" lifted its 5-hour and its weekly window until the end
    /// they had then.
    pub lifted: [Option<i64>; 2],
    /// Principal's other meters lifted (`AccountHold::lifted_other`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lifted_other: BTreeMap<String, i64>,
    /// The other accounts', by account (only those that hold something).
    pub accounts: BTreeMap<String, AccountHold>,
}

/// The autopilot's pause, kept across a restart: the quota windows last read on each account (a
/// reading holds until its window's end, and with an API key none comes again) and what else
/// holds it.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedPause {
    /// Principal's windows, where the versions before the accounts kept the only ones: a state
    /// they saved gives them to Principal, and a version rolled back still reads them.
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    /// Principal's other meters (`SavedWindows::other`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<QuotaMeter>,
    pub hold: Hold,
    /// The other accounts' windows, by account.
    pub accounts: BTreeMap<String, SavedWindows>,
}

/// An account's quota meters, as last read: Claude Code's two windows where the versions before
/// 1.8 kept them, the others beside them.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedWindows {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    /// Its other meters: windows of another length, the month's credits.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<QuotaMeter>,
}

impl SavedWindows {
    /// The meters to keep, as saved.
    pub fn of(meters: &[QuotaMeter]) -> Self {
        let at = |m: u32| {
            meters
                .iter()
                .find(|x| x.minutes == Some(m))
                .map(QuotaMeter::rate)
        };
        Self {
            five_hour: at(FIVE_HOURS),
            seven_day: at(WEEK),
            other: meters
                .iter()
                .filter(|x| x.minutes != Some(FIVE_HOURS) && x.minutes != Some(WEEK))
                .copied()
                .collect(),
        }
    }

    /// The meters saved.
    pub fn meters(&self) -> Vec<QuotaMeter> {
        merge_meters(&claude_meters(self.five_hour, self.seven_day), &self.other)
    }

    pub fn is_empty(&self) -> bool {
        self.five_hour.is_none() && self.seven_day.is_none() && self.other.is_empty()
    }
}

/// Why no ticket starts, and until when (`board::autopilot_pause`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotPause {
    pub reason: PauseReason,
    /// The window's length, for `PauseReason::Window`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    /// The window's use, 0-100 (none after a limit).
    pub pct: Option<f64>,
    /// When the tickets start again: the window's end, or about then after a limit.
    pub until: i64,
    /// The accounts a ticket could start on, all held back (the reason and the end are those of
    /// the first to be free again); empty with a single account, which there is nothing to tell
    /// from.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<String>,
}

/// The quotas of every account, of every harness, as the window is told them (`usage::settle`).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    /// What the turns cost today, on every account (also those the settings no longer list).
    pub today_cost: f64,
    /// Every account, in the settings' order.
    pub accounts: Vec<AccountUsage>,
    /// The account new agents of each harness would go to (`accounts::pick`): the one whose
    /// meters the status bar shows for that harness. Every harness that has an account.
    pub current: BTreeMap<EngineId, String>,
}

/// One account's quota, as last read.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsage {
    pub id: String,
    /// Its harness.
    pub engine: EngineId,
    /// Its meters as last read (`QuotaMeter`), the shortest window first; none for a harness
    /// without a quota, or not read yet.
    pub meters: Vec<QuotaMeter>,
    /// Signed in, as the last reading tells (an account not read yet is taken as signed in).
    pub connected: bool,
    /// Why its quota is not read (not signed in, sign-in expired), in the interface's language.
    pub reason: Option<String>,
    /// What its turns cost today (`UsageSnapshot::set_today`).
    pub today_cost: f64,
    /// Its « Budget par jour ($) » (`Account::daily_budget`), when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_budget: Option<f64>,
    /// The start of the day `today_cost` is of (local midnight, ms): a budget holds nothing back
    /// once that day is over, before the cost is read again.
    #[serde(skip)]
    pub today_start: i64,
    /// When its windows were last read, ms since epoch (0: not in this run).
    pub updated_at: i64,
    /// Its agents go on answering over their window, as extra usage billed apart (the last
    /// `rate_limit_event` of one of them says so, `isUsingOverage`).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub overage: bool,
    /// What `connected` and `reason` tell, kept to write them again in another language.
    #[serde(skip)]
    pub problem: Option<SignInProblem>,
}

#[cfg(test)]
impl UsageSnapshot {
    /// The account new agents of Claude Code go to.
    pub fn claude_current(&self) -> &str {
        self.current
            .get(&EngineId::CLAUDE)
            .map_or("", String::as_str)
    }

    /// Its 5-hour window, as the status bar shows it for Claude Code.
    pub fn five_hour(&self) -> Option<RateWindow> {
        let id = self.claude_current();
        self.accounts.iter().find(|a| a.id == id)?.five_hour()
    }

    pub fn seven_day(&self) -> Option<RateWindow> {
        let id = self.claude_current();
        self.accounts.iter().find(|a| a.id == id)?.seven_day()
    }

    /// When the current account of Claude Code was last read.
    pub fn updated_at(&self) -> i64 {
        let id = self.claude_current();
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .map_or(0, |a| a.updated_at)
    }
}

#[cfg(test)]
impl AccountUsage {
    pub fn five_hour(&self) -> Option<RateWindow> {
        self.window(FIVE_HOURS)
    }

    pub fn seven_day(&self) -> Option<RateWindow> {
        self.window(WEEK)
    }
}

impl AccountUsage {
    /// Its window of `minutes`, as last read.
    pub fn window(&self, minutes: u32) -> Option<RateWindow> {
        self.meters
            .iter()
            .find(|m| m.minutes == Some(minutes))
            .map(QuotaMeter::rate)
    }
}

/// Why an account's quota cannot be read from the usage endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInProblem {
    /// No sign-in to claude.ai in its folder (an API key, or never signed in).
    NotSignedIn,
    /// Its sign-in is out of date: Claude Code renews it when it runs.
    Expired,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ConvOp {
    Append {
        item: Value,
    },
    Patch {
        id: String,
        patch: Value,
    },
    /// Text appended to the `text` field of an item (streaming).
    Delta {
        id: String,
        text: String,
    },
}

// Serialized and sent right away: the size gap between variants does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiEvent {
    /// Where an agent is in its project's workflow, when what shows of it changed (ESC-193).
    WorkflowTrack {
        track: crate::workflow_track::TrackView,
    },
    Agent {
        agent: AgentView,
    },
    #[serde(rename_all = "camelCase")]
    AgentRemoved {
        id: String,
        project_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Conv {
        agent_id: String,
        ops: Vec<ConvOp>,
    },
    /// A project's shared memory changed (ESC-255): written or forgotten through the server or the window.
    #[serde(rename_all = "camelCase")]
    Memory {
        project_id: String,
    },
    /// A project's « Index du code » changed (ESC-293): read, erased, or waiting for its reading.
    #[serde(rename_all = "camelCase")]
    CodeIndex {
        project_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Git {
        project_id: String,
        git: GitInfo,
    },
    Usage {
        usage: UsageSnapshot,
    },
    #[serde(rename_all = "camelCase")]
    Focus {
        project_id: String,
        agent_id: Option<String>,
    },
    /// "Quitter" while the editor holds unsaved files: the window confirms first.
    QuitRequested {
        unsaved: usize,
    },
    /// The automatic restart for the update downloaded: when it comes (the user is warned before
    /// it), or none once it is called off.
    UpdateRestart {
        at: Option<i64>,
    },
    /// The update downloaded did not install, the app still running: it does not restart by
    /// itself for it any more (« Réessayer »).
    UpdateFailed {
        version: String,
    },
    #[serde(rename_all = "camelCase")]
    TerminalExit {
        id: String,
        code: Option<u32>,
    },
    Resources {
        resources: Resources,
    },
    Models {
        models: Vec<ModelInfo>,
    },
    /// The accounts as the backend changed them by itself (a harness's Principal made as it first
    /// serves, `Core::accounts_of`).
    Accounts {
        accounts: Vec<Account>,
        engine_order: Vec<EngineId>,
    },
    /// What the `claude` of an account told of its models (`PersistedState::account_models`).
    AccountModels {
        account: String,
        models: AccountModels,
    },
    /// The models a harness listed for its account (`PersistedState::engine_models`).
    EngineModels {
        engine: EngineId,
        models: Vec<crate::engine::registry::ToldModel>,
    },
    Ticket {
        ticket: Ticket,
    },
    #[serde(rename_all = "camelCase")]
    TicketRemoved {
        id: String,
        project_id: String,
    },
    /// A project changed on the backend's side (its board's prefix, target or settings).
    Project {
        project: Project,
    },
    /// A ticket's notification was clicked: the project's board is shown.
    #[serde(rename_all = "camelCase")]
    FocusBoard {
        project_id: String,
    },
    /// Why no ticket of the project's board starts (its target branch has no commit yet, or is
    /// gone), or None once they may start again.
    #[serde(rename_all = "camelCase")]
    BoardIssue {
        project_id: String,
        issue: Option<String>,
    },
    /// Why no ticket of a board starts for now (a quota, a usage limit), or None once they may
    /// start again: `pause` for the projects that go to any account, `projects` for the paused
    /// ones that prefer an account (each pauses on its own).
    AutopilotPause {
        pause: Option<AutopilotPause>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        projects: BTreeMap<String, AutopilotPause>,
    },
    /// An address to open in the default browser (a pull request to finish on GitHub).
    OpenUrl {
        url: String,
    },
    /// Something done in the background the window says in passing (tickets imported by
    /// themselves).
    Toast {
        text: String,
    },
    /// The settings changed the language of the interface or of the texts Claude writes: the
    /// window switches to it at once.
    Language {
        lang: crate::i18n::LangInfo,
    },
    /// Lines the step `step` (from 0) of the setup of an agent's worktree wrote since the window
    /// was last sent some (50 ms apart at most), its `total` lines so far counting them. A step
    /// starts with none: the window forgets the lines of the step before.
    #[serde(rename_all = "camelCase")]
    SetupOutput {
        agent_id: String,
        step: usize,
        total: usize,
        lines: Vec<String>,
    },
    /// The MCP server started, stopped, could not start, or listens on another port now.
    McpStatus {
        status: crate::mcp::McpStatus,
    },
    /// What the MCP server was asked, or refused, as its activity log keeps it.
    McpActivity {
        entry: crate::mcp::activity::ActivityEntry,
    },
    /// Where the server stands in each active account's Claude Code (declared, or why not).
    McpDeclared {
        declared: Vec<crate::mcp::install::Declaration>,
    },
    /// The dictation: the level while it listens, a listening that ended by itself, a model's
    /// download.
    Voice {
        voice: crate::voice::VoiceEvent,
    },
    /// A link `escouade://` came (STORE-11): the window takes it (`deep_links_take`).
    DeepLink {},
    /// The download of the vectors' model (ESC-254): bytes so far, of `total`.
    VectorsDownload {
        done: u64,
        total: u64,
    },
    /// What a project's docs have to share (ESC-257): the changes not shared yet, and the conflict
    /// a « Partager » stopped on, if any.
    #[serde(rename_all = "camelCase")]
    DocsShare {
        project_id: String,
        pending: u32,
        conflict: Option<crate::docsshare::Conflict>,
    },
    /// The pages of a project being compared to the code (ESC-263), told when one starts or ends.
    #[serde(rename_all = "camelCase")]
    DocsCompared {
        project_id: String,
        comparing: Vec<String>,
    },
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

/// Merges consecutive text deltas of the same item, keeping every other op in order.
pub fn coalesce_ops(ops: Vec<ConvOp>) -> Vec<ConvOp> {
    let mut out: Vec<ConvOp> = Vec::with_capacity(ops.len());
    for op in ops {
        if let (
            ConvOp::Delta { id, text },
            Some(ConvOp::Delta {
                id: last_id,
                text: last,
            }),
        ) = (&op, out.last_mut())
        {
            if id == last_id {
                last.push_str(text);
                continue;
            }
        }
        out.push(op);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn delta(id: &str, t: &str) -> ConvOp {
        ConvOp::Delta {
            id: id.into(),
            text: t.into(),
        }
    }

    #[test]
    fn a_plan_approved_resumes_in_the_mode_before_plan_never_in_ask_by_default() {
        let mut a = AgentMeta {
            mode: "auto".into(),
            ..Default::default()
        };
        // Auto → Plan (by the menu or by its harness): Auto again once approved.
        assert!(a.set_mode("plan"));
        assert!(!a.set_mode("plan"));
        assert_eq!(a.plan_from.as_deref(), Some("auto"));
        assert_eq!(a.plan_resume(true), "auto");
        // A model with no auto mode (Claude Code's Haiku 4.5): « Demander ».
        assert_eq!(a.plan_resume(false), "default");
        // Bypass only where it was.
        a.set_mode("bypassPermissions");
        a.set_mode("plan");
        assert_eq!(a.plan_resume(true), "bypassPermissions");
        // Out of Plan without Escouade knowing it went in: its own mode.
        a.set_mode("acceptEdits");
        assert_eq!(a.plan_resume(true), "acceptEdits");
        // In Plan with nothing kept (saved before ESC-277): the app's mode of new agents.
        let old = AgentMeta {
            mode: "plan".into(),
            ..Default::default()
        };
        assert_eq!(old.plan_resume(true), "auto");
        // Its Plan's mode of departure never `plan`.
        let odd = AgentMeta {
            mode: "plan".into(),
            plan_from: Some("plan".into()),
            ..Default::default()
        };
        assert_eq!(odd.plan_resume(true), "auto");
        // Saved and read again, with its mode kept.
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["planFrom"], "bypassPermissions");
    }

    #[test]
    fn settings_saved_with_an_external_editor_still_load() {
        let s: Settings =
            serde_json::from_str(r#"{"editorCommand":"code","sound":false}"#).unwrap();
        assert!(!s.sound);
        // The field is gone: saved again, it is not written back.
        assert!(!serde_json::to_string(&s).unwrap().contains("editorCommand"));
        // Saved before the TLS setting: certificates are checked.
        assert!(!s.insecure_tls);
    }

    #[test]
    fn settings_without_the_languages_are_the_defaults_and_the_load_decides_for_an_old_file() {
        // (A file of before 1.7 is read as French when the app loads it, `Core::load`: here it is
        // only the fields that are not there.)
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert_eq!(
            (s.language.as_str(), s.claude_language.as_str()),
            ("system", "ui")
        );
        let d = Settings::default();
        assert_eq!(
            (d.language.as_str(), d.claude_language.as_str()),
            ("system", "ui")
        );
        let v = serde_json::to_value(Settings {
            language: "en".into(),
            claude_language: "fr".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            (&v["language"], &v["claudeLanguage"]),
            (&json!("en"), &json!("fr"))
        );
    }

    #[test]
    fn what_was_saved_before_the_notification_choices_notifies_for_everything() {
        let s: Settings =
            serde_json::from_str(r#"{"sound":false,"osNotifications":true}"#).unwrap();
        assert_eq!(s.notify_for, NotifyFor::default());
        for kind in [
            NotifyKind::Question,
            NotifyKind::Done,
            NotifyKind::Error,
            NotifyKind::Ticket,
        ] {
            assert!(s.notify_for.allows(kind), "{kind:?}");
        }
        // A choice saved alone leaves the others on.
        let s: Settings = serde_json::from_str(r#"{"notifyFor":{"done":false}}"#).unwrap();
        assert_eq!(
            s.notify_for,
            NotifyFor {
                done: false,
                ..NotifyFor::default()
            }
        );
        assert_eq!(
            serde_json::to_value(&s).unwrap()["notifyFor"],
            json!({ "questions": true, "done": false, "errors": true, "tickets": true })
        );
    }

    #[test]
    fn a_notification_kind_is_let_through_by_its_own_choice_only() {
        let all_but = |off: NotifyKind| {
            let mut n = NotifyFor::default();
            match off {
                NotifyKind::Question => n.questions = false,
                NotifyKind::Done => n.done = false,
                NotifyKind::Error => n.errors = false,
                NotifyKind::Ticket => n.tickets = false,
            }
            n
        };
        let kinds = [
            NotifyKind::Question,
            NotifyKind::Done,
            NotifyKind::Error,
            NotifyKind::Ticket,
        ];
        for off in kinds {
            let n = all_but(off);
            for kind in kinds {
                assert_eq!(n.allows(kind), kind != off, "{kind:?} with {off:?} off");
            }
        }
    }

    #[test]
    fn an_agent_saved_before_the_accounts_runs_on_principal() {
        let m: AgentMeta =
            serde_json::from_value(json!({ "id": "a1", "sessionId": "s1" })).unwrap();
        assert_eq!(m.account, "principal");
        let v = serde_json::to_value(AgentMeta {
            account: "pro".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["account"], json!("pro"));
        // Settings saved before them have none: the load puts Principal in (`accounts::normalize`).
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert!(s.accounts.is_empty());
    }

    #[test]
    fn usage_statistics_wait_for_the_users_answer() {
        // A new install, and settings saved before the question existed: not asked yet.
        assert_eq!(Settings::default().telemetry, None);
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert_eq!(s.telemetry, None);
        for answer in [true, false] {
            let s: Settings = serde_json::from_value(json!({ "telemetry": answer })).unwrap();
            assert_eq!(s.telemetry, Some(answer));
        }
        let v = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(v["telemetry"], Value::Null);
    }

    #[test]
    fn a_tickets_agent_stopped_by_the_limit_goes_to_another_account_only_when_the_user_says_yes() {
        // Off unless the user asks (not verified with a second real account): settings saved
        // before the setting, and a new install, have it off.
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert!(!s.switch_on_limit);
        assert!(!Settings::default().switch_on_limit);
        let on: Settings =
            serde_json::from_value(json!({ "switchOnLimit": true, "autoResume": false })).unwrap();
        assert!(on.switch_on_limit);
        let v = serde_json::to_value(Settings {
            switch_on_limit: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["switchOnLimit"], json!(true));
        // An agent saved before it was ever moved has no account it came from.
        let m: AgentMeta =
            serde_json::from_value(json!({ "id": "a1", "sessionId": "s1" })).unwrap();
        assert_eq!((m.moved_from, m.moved_resume_at), (None, None));
        let v = serde_json::to_value(AgentMeta {
            moved_from: Some("principal".into()),
            moved_resume_at: Some(1_790_000_000_000),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["movedFrom"], json!("principal"));
        assert_eq!(v["movedResumeAt"], json!(1_790_000_000_000_i64));
    }

    #[test]
    fn settings_saved_before_the_quota_pause_hold_tickets_back_at_100_percent_only() {
        let s: Settings =
            serde_json::from_value(json!({ "sound": false, "autoResume": false })).unwrap();
        assert_eq!(s.quota_pause, 100);
        assert_eq!(Settings::default().quota_pause, 100);
        let v = serde_json::to_value(Settings {
            quota_pause: 90,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["quotaPause"], json!(90));
    }

    #[test]
    fn settings_saved_before_the_switch_threshold_change_account_at_the_limit_only_and_keep_the_rest(
    ) {
        // ESC-148: « À la limite », the behaviour of before; the move at the limit kept as it was.
        for on in [true, false] {
            let s: Settings = serde_json::from_value(
                json!({ "quotaPause": 90, "switchOnLimit": on, "handoverOnLimit": on }),
            )
            .unwrap();
            assert_eq!(
                (s.switch_at, s.switch_on_limit, s.handover_on_limit),
                (100, on, on)
            );
            assert_eq!(s.quota_pause, 90);
        }
        assert_eq!(Settings::default().switch_at, 100);
        let v = serde_json::to_value(Settings {
            switch_at: 95,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["switchAt"], json!(95));
    }

    #[test]
    fn the_quota_projection_shows_past_80_percent_unless_chosen_otherwise() {
        let read = |v: Value| {
            serde_json::from_value::<Settings>(json!({ "quotaProjection": v }))
                .unwrap()
                .quota_projection
        };
        let missing: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert_eq!(missing.quota_projection, QuotaProjection::Alert);
        assert_eq!(Settings::default().quota_projection, QuotaProjection::Alert);
        // The switch of before: on, its default; off, none.
        assert_eq!(read(json!(true)), QuotaProjection::Alert);
        assert_eq!(read(json!(false)), QuotaProjection::Off);
        assert_eq!(read(json!("always")), QuotaProjection::Always);
        assert_eq!(read(json!("off")), QuotaProjection::Off);
        // A value of a later version is read as the default, not as an error that loses the settings.
        assert_eq!(read(json!("later")), QuotaProjection::Alert);
        let saved = serde_json::to_value(Settings {
            quota_projection: QuotaProjection::Always,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(saved["quotaProjection"], json!("always"));
        assert_eq!(
            read(saved["quotaProjection"].clone()),
            QuotaProjection::Always
        );
    }

    #[test]
    fn settings_saved_before_the_attached_files_get_the_defaults_of_the_spec() {
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        let d = Settings::default();
        for s in [&s, &d] {
            assert_eq!(
                (s.attach_max_mb, s.attach_warn_mb, s.attach_origin),
                (0, 200, true)
            );
            assert_eq!(
                (s.attach_big_text.as_str(), s.attach_keep_days),
                ("path", 0)
            );
        }
        let v = serde_json::to_value(Settings {
            attach_max_mb: 500,
            attach_warn_mb: 50,
            attach_origin: false,
            attach_big_text: "refuse".into(),
            attach_keep_days: 30,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["attachMaxMb"], json!(500));
        assert_eq!(v["attachWarnMb"], json!(50));
        assert_eq!(v["attachOrigin"], json!(false));
        assert_eq!(v["attachBigText"], json!("refuse"));
        assert_eq!(v["attachKeepDays"], json!(30));
    }

    #[test]
    fn the_display_settings_default_to_the_sizes_of_before_and_stay_in_their_range() {
        let s: Settings = serde_json::from_value(json!({})).unwrap();
        assert_eq!(
            (s.ui_zoom, s.terminal_font_size, s.editor_font_size),
            (100, 12.5, 12.5)
        );
        let mut s: Settings = serde_json::from_value(json!({
            "uiZoom": 133, "terminalFontSize": 3, "editorFontSize": 13.3
        }))
        .unwrap();
        s.normalize_display();
        assert_eq!(
            (s.ui_zoom, s.terminal_font_size, s.editor_font_size),
            (125, 9.0, 13.5)
        );
        s.ui_zoom = 1000;
        s.terminal_font_size = f64::NAN;
        s.editor_font_size = 40.0;
        s.normalize_display();
        assert_eq!(
            (s.ui_zoom, s.terminal_font_size, s.editor_font_size),
            (200, 12.5, 24.0)
        );
        s.ui_zoom = 0;
        s.normalize_display();
        assert_eq!(s.ui_zoom, 80);
        // The rail's width (ESC-168): its usual one until dragged, then in its range.
        assert_eq!(s.rail_width, 0);
        for (kept, read) in [(0, 0), (10, 52), (180, 180), (900, 300)] {
            s.rail_width = kept;
            s.normalize_display();
            assert_eq!(s.rail_width, read);
        }
        // The sidebar’s (ESC-204): the same.
        assert_eq!(s.side_width, 0);
        for (kept, read) in [(0, 0), (10, 220), (320, 320), (900, 480)] {
            s.side_width = kept;
            s.normalize_display();
            assert_eq!(s.side_width, read);
        }
        s.rail_width = 180;
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(
            (
                v["uiZoom"].clone(),
                v["editorFontSize"].clone(),
                v["railWidth"].clone(),
                v["sideWidth"].clone()
            ),
            (json!(80), json!(24.0), json!(180), json!(480))
        );
    }

    #[test]
    fn the_theme_is_the_system_s_by_default_and_for_what_it_does_not_know() {
        let s: Settings = serde_json::from_value(json!({})).unwrap();
        assert_eq!(s.theme, ThemeChoice::System);
        for (written, theme) in [
            ("light", ThemeChoice::Light),
            ("dark", ThemeChoice::Dark),
            ("system", ThemeChoice::System),
            ("sepia", ThemeChoice::System),
        ] {
            let s: Settings = serde_json::from_value(json!({ "theme": written })).unwrap();
            assert_eq!(s.theme, theme, "{written}");
        }
        let v = serde_json::to_value(Settings {
            theme: ThemeChoice::Light,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["theme"], json!("light"));
    }

    #[test]
    fn the_colours_are_escouade_s_until_a_plugin_s_theme_is_chosen() {
        let s: Settings = serde_json::from_value(json!({})).unwrap();
        assert_eq!(s.plugin_theme, "");
        let s: Settings =
            serde_json::from_value(json!({ "pluginTheme": "dev.exemple.hc/high-contrast" }))
                .unwrap();
        assert_eq!(s.plugin_theme, "dev.exemple.hc/high-contrast");
        assert_eq!(
            serde_json::to_value(&s).unwrap()["pluginTheme"],
            json!("dev.exemple.hc/high-contrast")
        );
    }

    #[test]
    fn the_zoom_is_held_to_what_leaves_the_interface_its_room() {
        assert_eq!(fit_zoom(200, 1440.0, 1080.0), 125);
        assert_eq!(fit_zoom(200, 1920.0, 1080.0), 175);
        assert_eq!(fit_zoom(150, 2560.0, 1400.0), 150);
        assert_eq!(fit_zoom(200, 2560.0, 700.0), 110);
        assert_eq!(fit_zoom(80, 1000.0, 600.0), 80);
        assert_eq!(fit_zoom(125, 1000.0, 600.0), 100);
    }

    #[test]
    fn the_delay_before_a_message_sent_during_a_turn_is_five_seconds_and_one_of_its_steps() {
        assert_eq!(Settings::default().mid_turn_delay_secs, 5);
        let read: Settings = serde_json::from_value(json!({})).unwrap();
        assert_eq!(read.mid_turn_delay_secs, 5);
        for (saved, kept) in [(7, 5), (0, 0), (10, 10), (20, 20), (30, 30), (45, 5)] {
            let mut s = Settings {
                mid_turn_delay_secs: saved,
                ..Default::default()
            };
            s.normalize_mid_turn_delay();
            assert_eq!(s.mid_turn_delay_secs, kept, "{saved}");
        }
        let v = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(v["midTurnDelaySecs"], json!(5));
    }

    #[test]
    fn a_big_text_setting_the_app_does_not_know_is_sent_by_path() {
        let mut s = Settings {
            attach_big_text: "whatever".into(),
            ..Default::default()
        };
        s.normalize_attachments();
        assert_eq!(s.attach_big_text, "path");
        s.attach_big_text = "refuse".into();
        s.normalize_attachments();
        assert_eq!(s.attach_big_text, "refuse");
    }

    #[test]
    fn the_files_granted_to_an_agent_are_saved_and_the_oldest_go_past_the_cap() {
        let old: AgentMeta = serde_json::from_value(json!({ "id": "a1" })).unwrap();
        assert!(old.granted_files.is_empty());
        let mut m = AgentMeta::default();
        for n in 0..MAX_GRANTED_FILES + 5 {
            m.grant_file(format!("C:/docs/f{n}.xlsx"));
        }
        assert_eq!(m.granted_files.len(), MAX_GRANTED_FILES);
        assert_eq!(m.granted_files[0], "C:/docs/f5.xlsx");
        // One granted again moves to the end rather than twice in the list.
        m.grant_file("C:/docs/f5.xlsx".into());
        assert_eq!(m.granted_files.len(), MAX_GRANTED_FILES);
        assert_eq!(m.granted_files.last().unwrap(), "C:/docs/f5.xlsx");
        assert_eq!(m.granted_files[0], "C:/docs/f6.xlsx");
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(
            v["grantedFiles"].as_array().unwrap().len(),
            MAX_GRANTED_FILES
        );
    }

    #[test]
    fn a_state_saved_before_the_autopilots_pause_still_loads_without_one() {
        let s: PersistedState = serde_json::from_value(json!({ "projects": [] })).unwrap();
        assert_eq!(s.pause, SavedPause::default());
        let saved = SavedPause {
            five_hour: Some(RateWindow {
                pct: 100.0,
                resets_at: Some(9),
            }),
            seven_day: None,
            hold: Hold {
                limit_until: Some(5),
                lifted: [None, Some(7)],
                ..Default::default()
            },
            ..Default::default()
        };
        let v = serde_json::to_value(PersistedState {
            pause: saved.clone(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["pause"]["fiveHour"]["resetsAt"], json!(9));
        assert_eq!(v["pause"]["hold"]["limitUntil"], json!(5));
        let back: PersistedState = serde_json::from_value(v).unwrap();
        assert_eq!(back.pause, saved);
        // Saved with only part of it.
        let part: SavedPause = serde_json::from_value(json!({ "hold": {} })).unwrap();
        assert_eq!(part, SavedPause::default());
    }

    #[test]
    fn the_quota_windows_are_saved_by_account_and_an_older_state_keeps_principals() {
        let full = RateWindow {
            pct: 100.0,
            resets_at: Some(9),
        };
        // Saved before the accounts: the windows of the only account there was.
        let old: SavedPause =
            serde_json::from_value(json!({ "fiveHour": { "pct": 100.0, "resetsAt": 9 },
                                           "hold": {} }))
            .unwrap();
        assert_eq!(old.five_hour, Some(full));
        assert!(old.accounts.is_empty());
        // The other accounts' beside them, by id.
        let saved = SavedPause {
            seven_day: Some(full),
            accounts: BTreeMap::from([(
                "pro".to_string(),
                SavedWindows {
                    five_hour: Some(full),
                    seven_day: None,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let v = serde_json::to_value(&saved).unwrap();
        assert_eq!(v["sevenDay"]["resetsAt"], json!(9));
        assert_eq!(
            v["accounts"],
            json!({ "pro": { "fiveHour": { "pct": 100.0, "resetsAt": 9 }, "sevenDay": null } })
        );
        assert_eq!(serde_json::from_value::<SavedPause>(v).unwrap(), saved);
    }

    #[test]
    fn the_quotas_of_every_account_travel_in_camel_case() {
        let w = RateWindow {
            pct: 12.0,
            resets_at: Some(5),
        };
        let usage = UsageSnapshot {
            today_cost: 1.5,
            accounts: vec![AccountUsage {
                id: "pro".into(),
                meters: vec![
                    QuotaMeter::window(FIVE_HOURS, w),
                    QuotaMeter::credits(RateWindow {
                        pct: 40.0,
                        resets_at: None,
                    }),
                ],
                connected: false,
                reason: Some("Pas connecté".into()),
                updated_at: 3,
                problem: Some(SignInProblem::NotSignedIn),
                ..Default::default()
            }],
            current: BTreeMap::from([(EngineId::CLAUDE, "pro".to_string())]),
        };
        assert_eq!(
            serde_json::to_value(&usage).unwrap(),
            json!({ "todayCost": 1.5, "current": { "claude": "pro" },
                    "accounts": [{ "id": "pro", "engine": "claude",
                                   "meters": [{ "minutes": 300, "pct": 12.0, "resetsAt": 5 },
                                              { "minutes": null, "pct": 40.0, "resetsAt": null }],
                                   "connected": false,
                                   "reason": "Pas connecté", "todayCost": 0.0,
                                   "updatedAt": 3 }] })
        );
    }

    #[test]
    fn the_autopilots_pause_travels_in_camel_case() {
        let e = UiEvent::AutopilotPause {
            pause: Some(AutopilotPause {
                reason: PauseReason::Window,
                minutes: Some(FIVE_HOURS),
                pct: Some(100.0),
                until: 5,
                accounts: vec![],
            }),
            projects: BTreeMap::new(),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "autopilotPause",
                    "pause": { "reason": "window", "minutes": 300, "pct": 100.0, "until": 5 } })
        );
        let over = UiEvent::AutopilotPause {
            pause: None,
            projects: BTreeMap::new(),
        };
        assert_eq!(
            serde_json::to_value(&over).unwrap(),
            json!({ "type": "autopilotPause", "pause": null })
        );
    }

    #[test]
    fn a_setup_step_keeps_its_last_500_lines_and_counts_them_all() {
        let mut out = SetupOutput::new(1);
        assert_eq!((out.step, out.total, out.lines.len()), (1, 0, 0));
        let lines: Vec<String> = (1..=600).map(|i| format!("ligne {i}")).collect();
        out.push(&lines[..550]);
        out.push(&lines[550..]);
        assert_eq!(SETUP_LINES, 500);
        assert_eq!(
            (out.step, out.total, out.lines.len()),
            (1, 600, SETUP_LINES)
        );
        assert_eq!(out.lines.front().map(String::as_str), Some("ligne 101"));
        assert_eq!(out.lines.back().map(String::as_str), Some("ligne 600"));
        // As the window reads it, and as the lines just written are sent to it.
        let sent = serde_json::to_value(&out).unwrap();
        assert_eq!(
            (sent["step"].clone(), sent["total"].clone()),
            (json!(1), json!(600))
        );
        assert_eq!(sent["lines"][0], "ligne 101");
        let e = UiEvent::SetupOutput {
            agent_id: "a1".into(),
            step: 1,
            total: 600,
            lines: vec!["ligne 600".into()],
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "setupOutput", "agentId": "a1", "step": 1, "total": 600, "lines": ["ligne 600"] })
        );
    }

    #[test]
    fn settings_saved_before_automatic_updates_install_them() {
        let s: Settings = serde_json::from_str(r#"{"sound":false}"#).unwrap();
        assert!(s.auto_update);
        assert_eq!(serde_json::to_value(&s).unwrap()["autoUpdate"], json!(true));
        let off: Settings = serde_json::from_str(r#"{"autoUpdate":false}"#).unwrap();
        assert!(!off.auto_update);
    }

    #[test]
    fn claude_code_gets_the_proxy_and_skips_tls_verification_only_when_asked() {
        let mut s = Settings::default();
        assert!(s.claude_env().is_empty());
        s.insecure_tls = true;
        assert_eq!(
            s.claude_env(),
            [("NODE_TLS_REJECT_UNAUTHORIZED".to_string(), "0".to_string())]
        );
        s.proxy_url = "http://proxy:3128".into();
        let env = s.claude_env();
        assert!(env.contains(&("HTTPS_PROXY".to_string(), "http://proxy:3128".to_string())));
        assert!(env.contains(&("NODE_TLS_REJECT_UNAUTHORIZED".to_string(), "0".to_string())));
        // The terminals' proxy stays the proxy alone.
        assert!(s
            .proxy_env()
            .iter()
            .all(|(k, _)| k != "NODE_TLS_REJECT_UNAUTHORIZED"));
        assert_eq!(
            serde_json::to_value(&s).unwrap()["insecureTls"],
            json!(true)
        );
    }

    #[test]
    fn agents_get_the_task_list_tools_while_the_setting_is_on_and_nothing_else_does() {
        let mut s = Settings::default();
        assert!(s.todo_tools);
        let var = (TODO_TOOLS_VAR.to_string(), "1".to_string());
        assert_eq!(s.agent_env(), vec![var.clone()]);
        // The other Claude Code processes (naming, commit messages, sign-in) have no use for them.
        assert!(s.claude_env().is_empty());
        // Added to the network settings, not instead of them.
        s.proxy_url = "http://proxy:3128".into();
        s.insecure_tls = true;
        assert!(s.agent_env().contains(&var));
        assert!(s.agent_env().len() > 2);
        assert!(!s.claude_env().contains(&var));
        // Off: nothing is added, and no value is forced the other way (the user's environment decides).
        s.todo_tools = false;
        assert_eq!(s.agent_env(), s.claude_env());
        assert!(s.agent_env().iter().all(|(k, _)| k != TODO_TOOLS_VAR));
    }

    #[test]
    fn settings_saved_before_the_task_list_setting_keep_it_on() {
        let s: Settings = serde_json::from_str(r#"{"sound":false}"#).unwrap();
        assert!(s.todo_tools);
        assert_eq!(serde_json::to_value(&s).unwrap()["todoTools"], json!(true));
        let off: Settings = serde_json::from_str(r#"{"todoTools":false}"#).unwrap();
        assert!(!off.todo_tools);
    }

    #[test]
    fn consecutive_deltas_of_one_item_are_merged_in_order() {
        let ops = vec![
            delta("a", "Bon"),
            delta("a", "jour"),
            delta("b", "x"),
            delta("a", " !"),
            ConvOp::Patch {
                id: "a".into(),
                patch: json!({ "streaming": false }),
            },
            delta("a", "?"),
        ];
        let out = serde_json::to_value(coalesce_ops(ops)).unwrap();
        assert_eq!(
            out,
            json!([
                { "op": "delta", "id": "a", "text": "Bonjour" },
                { "op": "delta", "id": "b", "text": "x" },
                { "op": "delta", "id": "a", "text": " !" },
                { "op": "patch", "id": "a", "patch": { "streaming": false } },
                { "op": "delta", "id": "a", "text": "?" },
            ])
        );
    }

    #[test]
    fn a_click_on_a_tickets_notification_asks_for_the_projects_board() {
        let e = UiEvent::FocusBoard {
            project_id: "p1".into(),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "focusBoard", "projectId": "p1" })
        );
    }

    #[test]
    fn the_models_claude_code_reports_are_read_one_by_one() {
        let reported = json!([
            { "value": "sonnet", "resolvedModel": "claude-sonnet-5-5", "displayName": "Sonnet 5.5" },
            { "value": "odd", "resolvedModel": null },
            "garbage",
            { "value": "bare" },
            { "value": "haiku", "resolvedModel": "claude-haiku-4-5-20251001" },
        ]);
        let values: Vec<String> = ModelInfo::list(&reported)
            .into_iter()
            .map(|m| m.value)
            .collect();
        assert_eq!(values, ["sonnet", "haiku"]);
        assert!(ModelInfo::list(&Value::Null).is_empty());
    }

    #[test]
    fn ui_layout_is_kept_and_defaults_to_classic_for_older_state_files() {
        let ui: UiState =
            serde_json::from_value(json!({ "view": "project", "layout": "split" })).unwrap();
        assert_eq!(serde_json::to_value(&ui).unwrap()["layout"], "split");
        let old: UiState = serde_json::from_value(json!({ "view": "project" })).unwrap();
        assert_eq!(serde_json::to_value(&old).unwrap()["layout"], "");
    }

    #[test]
    fn what_was_saved_before_the_harnesses_runs_on_claude_code() {
        let a: AgentMeta =
            serde_json::from_value(json!({ "id": "a1", "sessionId": "s1" })).unwrap();
        assert_eq!(a.engine, EngineId::CLAUDE);
        let t: Ticket = serde_json::from_value(json!({ "id": "t1", "key": "DEM-1" })).unwrap();
        assert_eq!(t.engine, None);
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert_eq!((p.engine, p.engine()), (None, EngineId::CLAUDE));
        // Each harness is kept by its id, and read back.
        for (engine, id) in [
            (EngineId::CLAUDE, "claude"),
            (EngineId::CODEX, "codex"),
            (EngineId::COPILOT, "copilot"),
            (EngineId::CURSOR, "cursor"),
            (EngineId::OPENCODE, "opencode"),
            (EngineId::VIBE, "vibe"),
            (EngineId::AGY, "agy"),
            (EngineId::GEMINI, "gemini"),
        ] {
            assert_eq!(engine.as_str(), id);
            let v = serde_json::to_value(AgentMeta {
                engine,
                ..Default::default()
            })
            .unwrap();
            assert_eq!(v["engine"], json!(id));
            assert_eq!(
                serde_json::from_value::<AgentMeta>(v).unwrap().engine,
                engine
            );
            let v = serde_json::to_value(Ticket {
                engine: Some(engine),
                ..Default::default()
            })
            .unwrap();
            assert_eq!(v["engine"], json!(id));
            assert_eq!(
                serde_json::from_value::<Ticket>(v).unwrap().engine,
                Some(engine)
            );
            let v = serde_json::to_value(Project {
                engine: Some(engine),
                ..p.clone()
            })
            .unwrap();
            assert_eq!(
                serde_json::from_value::<Project>(v).unwrap().engine(),
                engine
            );
        }
    }

    #[test]
    fn tickets_and_worktrees_saved_before_existing_branches_still_load() {
        let t: Ticket = serde_json::from_value(json!({ "id": "t1", "key": "DEM-1" })).unwrap();
        assert_eq!(t.branch, "");
        let t: Ticket =
            serde_json::from_value(json!({ "id": "t1", "branch": "feat/login" })).unwrap();
        assert_eq!(t.branch, "feat/login");
        // An agent's worktree made before them has its own branch.
        let wt: Worktree = serde_json::from_value(
            json!({ "path": "C:/demo/.claude/worktrees/x", "branch": "escouade/x", "baseBranch": "main" }),
        )
        .unwrap();
        assert!(!wt.existing);
        let kept: Worktree = serde_json::from_value(json!({
            "path": "C:/demo/.claude/worktrees/x", "branch": "feat/x", "baseBranch": "main", "existing": true
        }))
        .unwrap();
        assert!(kept.existing);
        assert_eq!(serde_json::to_value(&kept).unwrap()["existing"], true);
    }

    #[test]
    fn projects_agents_and_states_saved_before_the_board_still_load() {
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert_eq!(p.worktree_copy, vec![".env*".to_string()]);
        assert!(p.worktree_setup.is_empty() && p.worktree_teardown.is_empty());
        assert_eq!(p.board, BoardSettings::default());
        assert_eq!(
            (
                p.board.action.as_str(),
                p.board.strategy.as_str(),
                p.board.conflict.as_str()
            ),
            ("merge", "squash", "ask")
        );
        assert_eq!((p.board.max_parallel, p.board.next_number), (2, 1));
        assert!(p.board.autopilot && p.board.cleanup && p.board.conventional);
        assert!(!p.board.tests_first && !p.board.draft);
        let a: AgentMeta = serde_json::from_value(json!({ "id": "a1", "name": "x" })).unwrap();
        assert_eq!((a.ticket_id, a.port_base, a.recipe), (None, None, None));
        let s: PersistedState = serde_json::from_value(json!({ "projects": [] })).unwrap();
        assert!(s.tickets.is_empty());
    }

    #[test]
    fn a_project_saved_before_the_commit_choice_has_its_agent_write_the_commits() {
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert_eq!(p.commit_mode, CommitMode::Agent);
        let direct = Project {
            commit_mode: CommitMode::Direct,
            ..p
        };
        let v = serde_json::to_value(&direct).unwrap();
        assert_eq!(v["commitMode"], json!("direct"));
        let back: Project = serde_json::from_value(v).unwrap();
        assert_eq!(back.commit_mode, CommitMode::Direct);
    }

    #[test]
    fn a_project_saved_before_the_preferred_account_goes_to_any_account() {
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        // Empty: « Automatique ».
        assert_eq!(p.account, "");
        let pro = Project {
            account: "pro".into(),
            ..p
        };
        let v = serde_json::to_value(&pro).unwrap();
        assert_eq!(v["account"], json!("pro"));
        let back: Project = serde_json::from_value(v).unwrap();
        assert_eq!(back.account, "pro");
    }

    #[test]
    fn a_project_saved_before_the_worktrees_settings_keeps_its_folder_and_its_prefixes() {
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert_eq!(
            (
                p.worktree_dir.as_str(),
                p.agent_branch_prefix.as_str(),
                p.ticket_branch_prefix.as_str()
            ),
            ("", "escouade/", "ticket/")
        );
        assert_eq!(
            p.worktrees_root(),
            std::path::Path::new("C:/demo")
                .join(".claude")
                .join("worktrees")
        );
        let own = Project {
            worktree_dir: "../demo-worktrees".into(),
            agent_branch_prefix: String::new(),
            ticket_branch_prefix: "gg/".into(),
            ..p
        };
        let v = serde_json::to_value(&own).unwrap();
        assert_eq!(
            (
                &v["worktreeDir"],
                &v["agentBranchPrefix"],
                &v["ticketBranchPrefix"]
            ),
            (&json!("../demo-worktrees"), &json!(""), &json!("gg/"))
        );
        // An emptied prefix stays empty: the name alone.
        let back: Project = serde_json::from_value(v).unwrap();
        assert_eq!(back.agent_branch_prefix, "");
        assert_eq!(
            back.worktrees_root(),
            std::path::Path::new("C:/demo-worktrees")
        );
    }

    #[test]
    fn what_holds_the_autopilot_back_is_kept_by_account_and_an_older_state_gives_it_to_principal() {
        // Saved before the accounts: the pauses of the only account there was.
        let old: Hold =
            serde_json::from_value(json!({ "limitUntil": 5, "lifted": [null, 7] })).unwrap();
        assert!(old.accounts.is_empty());
        assert_eq!(
            old.of("principal"),
            AccountHold {
                limit_until: Some(5),
                lifted: [None, Some(7)],
                ..Default::default()
            }
        );
        assert_eq!(old.of("pro"), AccountHold::default());
        // The other accounts' beside them, by id.
        let mut hold = old.clone();
        hold.accounts.insert(
            "pro".into(),
            AccountHold {
                limit_until: Some(9),
                lifted: [Some(3), None],
                ..Default::default()
            },
        );
        let v = serde_json::to_value(&hold).unwrap();
        assert_eq!(v["limitUntil"], json!(5));
        assert_eq!(
            v["accounts"],
            json!({ "pro": { "limitUntil": 9, "lifted": [3, null] } })
        );
        assert_eq!(serde_json::from_value::<Hold>(v).unwrap(), hold);
        // Saved with only part of it.
        let part: Hold = serde_json::from_value(json!({ "accounts": { "pro": {} } })).unwrap();
        assert_eq!(part.of("pro"), AccountHold::default());
    }

    #[test]
    fn the_pause_of_several_accounts_names_them_and_the_projects_that_wait_for_their_own() {
        let pause = |accounts: &[&str]| AutopilotPause {
            reason: PauseReason::Window,
            minutes: Some(WEEK),
            pct: Some(97.0),
            until: 5,
            accounts: accounts.iter().map(|a| a.to_string()).collect(),
        };
        let e = UiEvent::AutopilotPause {
            pause: Some(pause(&["principal", "pro"])),
            projects: BTreeMap::from([("p1".to_string(), pause(&["pro"]))]),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "autopilotPause",
                    "pause": { "reason": "window", "minutes": 10080, "pct": 97.0, "until": 5,
                               "accounts": ["principal", "pro"] },
                    "projects": { "p1": { "reason": "window", "minutes": 10080, "pct": 97.0, "until": 5,
                                          "accounts": ["pro"] } } })
        );
    }

    #[test]
    fn an_agent_saved_before_recipes_were_approved_loads_with_nothing_approved() {
        // A state.json of 1.5: the agent has a recipe, and no approval was ever asked.
        let s: PersistedState = serde_json::from_value(json!({
            "agents": [{
                "id": "a1",
                "name": "x",
                "recipe": { "prepare": [], "processes": [{ "name": "web", "command": "node web.js" }], "open": "" }
            }]
        }))
        .unwrap();
        assert!(s.agents[0].recipe.is_some());
        assert_eq!(s.agents[0].approved_recipe, None);
        assert_eq!(s.agents[0].approved_isola, None);
    }

    #[test]
    fn an_agent_saved_before_copies_has_no_session_to_fork() {
        // A state.json of 1.5: an agent with a session of its own, and no copy ever made.
        let s: PersistedState = serde_json::from_value(json!({
            "agents": [{ "id": "a1", "name": "x", "sessionId": "s1" }]
        }))
        .unwrap();
        let a = &s.agents[0];
        assert_eq!(a.session_id.as_deref(), Some("s1"));
        // No entry recorded yet: a copy made before its next turn forks its whole session.
        assert_eq!(
            (&a.fork_of, &a.fork_at, &a.last_entry),
            (&None, &None, &None)
        );
        // A copy's are saved with it, in camel case.
        let copy = AgentMeta {
            fork_of: Some("s1".into()),
            fork_at: Some("e1".into()),
            last_entry: Some("e2".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&copy).unwrap();
        assert_eq!(
            (&v["forkOf"], &v["forkAt"], &v["lastEntry"]),
            (&json!("s1"), &json!("e1"), &json!("e2"))
        );
        let back: AgentMeta = serde_json::from_value(v).unwrap();
        assert_eq!(
            (
                back.fork_of.as_deref(),
                back.fork_at.as_deref(),
                back.last_entry.as_deref()
            ),
            (Some("s1"), Some("e1"), Some("e2"))
        );
    }

    #[test]
    fn the_approved_recipe_is_saved_with_the_agent_and_travels_in_camel_case() {
        let recipe = TestRecipe {
            prepare: vec![RecipeStep {
                command: "npm install".into(),
                dir: "web".into(),
            }],
            open: "http://localhost:4111".into(),
            ..Default::default()
        };
        let state = PersistedState {
            agents: vec![AgentMeta {
                id: "a1".into(),
                recipe: Some(recipe.clone()),
                approved_recipe: Some(recipe.clone()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let saved = serde_json::to_string_pretty(&state).unwrap();
        let back: PersistedState = serde_json::from_str(&saved).unwrap();
        assert_eq!(back.agents[0].approved_recipe, Some(recipe));
        let sent = serde_json::to_value(&state.agents[0]).unwrap();
        assert_eq!(
            sent["approvedRecipe"]["prepare"][0]["command"],
            "npm install"
        );
    }

    #[test]
    fn a_projects_worktree_steps_travel_in_camel_case() {
        let p: Project = serde_json::from_value(json!({
            "id": "p1", "name": "demo", "path": "C:/demo", "color": "red",
            "worktreeSetup": [{ "id": "s1", "command": "npm ci", "shell": "pwsh", "cwd": "web" }],
            "worktreeTeardown": [{ "command": "docker compose down" }],
        }))
        .unwrap();
        assert_eq!(
            p.worktree_setup,
            [WorktreeStep {
                id: "s1".into(),
                command: "npm ci".into(),
                shell: "pwsh".into(),
                cwd: "web".into(),
            }]
        );
        assert_eq!(p.worktree_teardown[0].command, "docker compose down");
        let sent = serde_json::to_value(&p).unwrap();
        assert_eq!(sent["worktreeSetup"][0]["cwd"], "web");
        assert_eq!(sent["worktreeTeardown"][0]["shell"], "");
    }

    #[test]
    fn a_ticket_and_a_recipe_travel_in_camel_case() {
        let t = Ticket {
            id: "t1".into(),
            key: "ATL-42".into(),
            max_loops: 5,
            column: Column::Review,
            agent_id: Some("a1".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(
            (
                v["maxLoops"].clone(),
                v["column"].clone(),
                v["agentId"].clone()
            ),
            (json!(5), json!("review"), json!("a1"))
        );
        // As the agent writes it, in French (a port may come as a number)…
        let r: TestRecipe = serde_json::from_value(json!({
            "preparation": [{ "commande": "npm install", "dossier": "web" }],
            "processus": [{ "nom": "api", "commande": "npm run dev", "env": { "PORT": 4110 },
                            "url": "http://localhost:4110" }],
            "ouvrir": "http://localhost:4111/connexion"
        }))
        .unwrap();
        assert_eq!(r.prepare[0].dir, "web");
        assert_eq!(r.processes[0].env["PORT"], "4110");
        assert_eq!(r.open, "http://localhost:4111/connexion");
        // …sent to the window in English.
        let sent = serde_json::to_value(&r).unwrap();
        assert_eq!(sent["processes"][0]["name"], "api");
        assert_eq!(sent["prepare"][0]["command"], "npm install");
    }

    #[test]
    fn a_ticket_saved_before_the_progress_still_loads_and_keeps_it_in_english() {
        let t: Ticket = serde_json::from_value(json!({ "id": "t1", "key": "ATL-42" })).unwrap();
        assert!(t.progress.is_empty());
        // As the agent names it, in French…
        let t: Ticket =
            serde_json::from_value(json!({ "id": "t1", "avancement": ["a", "b"] })).unwrap();
        assert_eq!(t.progress, ["a", "b"]);
        // …and as it is saved and sent to the window.
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["progress"], json!(["a", "b"]));
        assert!(v.get("avancement").is_none());
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(back, t);
        // A new ticket has none to show.
        assert_eq!(
            serde_json::to_value(Ticket::default()).unwrap()["progress"],
            json!([])
        );
    }

    #[test]
    fn what_was_saved_before_the_integrations_still_loads_with_their_defaults() {
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert_eq!(s.integrations, IntegrationSettings::default());
        assert!(s.integrations.sync_states && s.integrations.extract_criteria);
        assert!(!s.integrations.auto_import && !s.integrations.loop_comments);
        assert_eq!(
            (
                s.integrations.import_label.as_str(),
                s.integrations.import_every
            ),
            ("claude-ready", 15)
        );
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert!(p.integrations.links.is_empty());
        assert_eq!(p.integrations.comments, [Column::Review, Column::Done]);
        let t: Ticket = serde_json::from_value(json!({ "id": "t1", "key": "ATL-42" })).unwrap();
        assert_eq!(t.external, None);
    }

    #[test]
    fn a_links_states_travel_by_column_and_an_external_ref_in_camel_case() {
        let mut states = BTreeMap::new();
        states.insert(
            Column::Doing,
            ExternalState {
                id: "3".into(),
                name: "In Progress".into(),
            },
        );
        let link = SourceLink {
            service: Service::Jira,
            container: "ATL".into(),
            name: "Atlas".into(),
            states,
        };
        let v = serde_json::to_value(&link).unwrap();
        assert_eq!(v["service"], "jira");
        assert_eq!(v["states"]["doing"]["name"], "In Progress");
        let back: SourceLink = serde_json::from_value(v).unwrap();
        assert_eq!(back, link);
        let r = ExternalRef {
            service: Service::Github,
            id: "42".into(),
            key: "#42".into(),
            container: "acme/api".into(),
            url: "https://github.com/acme/api/issues/42".into(),
            error: None,
        };
        let t = serde_json::to_value(Ticket {
            external: Some(r),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(t["external"]["service"], "github");
        assert_eq!(t["external"]["container"], "acme/api");
    }

    #[test]
    fn a_ticket_saved_before_its_loops_were_counted_still_loads_with_none() {
        let t: Ticket =
            serde_json::from_value(json!({ "id": "t1", "key": "ATL-42", "iteration": 3 })).unwrap();
        assert_eq!((t.loops, t.iteration), (0, 3));
        let v = serde_json::to_value(Ticket {
            loops: 4,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["loops"], json!(4));
    }

    #[test]
    fn a_ticket_done_before_1_7_loads_with_the_outcome_of_its_card_only() {
        // A state.json of 1.6: its done tickets have an `outcome` and no other language of it.
        let s: PersistedState = serde_json::from_value(json!({
            "projects": [],
            "tickets": [{
                "id": "t1", "projectId": "p1", "key": "DEM-1", "column": "done",
                "outcome": "⤵ Mergé dans main · squash"
            }]
        }))
        .unwrap();
        let t = &s.tickets[0];
        assert_eq!(
            (t.outcome.as_deref(), t.outcome_claude.as_deref()),
            (Some("⤵ Mergé dans main · squash"), None)
        );
        // Saved from now on with both, and read back.
        let v = serde_json::to_value(Ticket {
            outcome: Some("⤵ Mergé dans main · squash".into()),
            outcome_claude: Some("⤵ Merged into main · squash".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["outcomeClaude"], json!("⤵ Merged into main · squash"));
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(
            back.outcome_claude.as_deref(),
            Some("⤵ Merged into main · squash")
        );
    }

    #[test]
    fn a_ticket_done_before_1_8_has_no_merge_commit_and_one_kept_is_saved_and_read_back() {
        // A state.json of 1.7: its done tickets have an `outcome` and no commit.
        let s: PersistedState = serde_json::from_value(json!({
            "projects": [],
            "tickets": [{
                "id": "t1", "projectId": "p1", "key": "DEM-1", "column": "done",
                "outcome": "⤵ Mergé dans main · squash"
            }]
        }))
        .unwrap();
        assert_eq!(s.tickets[0].outcome_commit, None);
        let v = serde_json::to_value(Ticket {
            outcome_commit: Some("0a1b2c".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["outcomeCommit"], json!("0a1b2c"));
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(back.outcome_commit.as_deref(), Some("0a1b2c"));
    }

    #[test]
    fn a_ticket_saved_before_its_files_still_loads_with_none_and_always_tells_its_files() {
        // A state.json of 1.7: its tickets have images, and no `files`.
        let s: PersistedState = serde_json::from_value(json!({
            "projects": [],
            "tickets": [{ "id": "t1", "projectId": "p1", "key": "DEM-1", "column": "todo" }]
        }))
        .unwrap();
        assert!(s.tickets[0].files.is_empty());
        // The window reads `ticket.files.map(…)`: the field is there even when empty.
        let v = serde_json::to_value(&s.tickets[0]).unwrap();
        assert_eq!(v["files"], json!([]));
        let v = serde_json::to_value(Ticket {
            files: vec![TicketFile {
                name: "rapport.xlsx".into(),
                file: "4f2a9c01/rapport.xlsx".into(),
                size: 4096,
            }],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            v["files"],
            json!([{ "name": "rapport.xlsx", "file": "4f2a9c01/rapport.xlsx", "size": 4096 }])
        );
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(back.files[0].file, "4f2a9c01/rapport.xlsx");
    }

    #[test]
    fn a_state_saved_before_dependencies_still_loads_with_tickets_waiting_for_none() {
        // A state.json of 1.5: its tickets have no `after`.
        let s: PersistedState = serde_json::from_value(json!({
            "projects": [],
            "tickets": [{ "id": "t1", "projectId": "p1", "key": "DEM-1", "column": "todo" }]
        }))
        .unwrap();
        assert!(s.tickets[0].after.is_empty());
        let v = serde_json::to_value(Ticket {
            after: vec!["t2".into(), "t3".into()],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["after"], json!(["t2", "t3"]));
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(back.after, ["t2", "t3"]);
    }

    #[test]
    fn a_null_env_or_a_null_value_in_it_does_not_spoil_the_recipe() {
        let r: TestRecipe = serde_json::from_value(json!({
            "processus": [
                { "nom": "web", "commande": "npm run dev", "env": null },
                { "nom": "api", "commande": "npm start",
                  "env": { "PORT": 4110, "DEBUG": null, "NAME": "x" } }
            ]
        }))
        .unwrap();
        assert!(r.processes[0].env.is_empty());
        let env = &r.processes[1].env;
        assert_eq!(
            (env.len(), env["PORT"].as_str(), env["NAME"].as_str()),
            (2, "4110", "x")
        );
    }

    #[test]
    fn a_null_text_or_list_in_the_recipe_is_an_absent_one() {
        let r: TestRecipe = serde_json::from_value(json!({
            "preparation": [{ "commande": "npm install", "dossier": null },
                            { "commande": null, "dossier": "web" }],
            "processus": [{ "nom": null, "commande": "npm run dev", "dossier": null,
                            "env": null, "url": null }],
            "ouvrir": null
        }))
        .unwrap();
        assert_eq!(
            r.prepare,
            [
                RecipeStep {
                    command: "npm install".into(),
                    dir: String::new()
                },
                RecipeStep {
                    command: String::new(),
                    dir: "web".into()
                }
            ]
        );
        let p = &r.processes[0];
        assert_eq!(p.command, "npm run dev");
        assert!(p.name.is_empty() && p.dir.is_empty() && p.url.is_empty() && p.env.is_empty());
        assert!(r.open.is_empty());
        // The lists themselves may be null.
        let r: TestRecipe = serde_json::from_value(json!({
            "preparation": null, "processus": null, "ouvrir": null
        }))
        .unwrap();
        assert_eq!(r, TestRecipe::default());
        // The English names the window sends back are as tolerant.
        let r: TestRecipe = serde_json::from_value(json!({
            "prepare": null, "processes": [{ "name": null, "url": null }], "open": null
        }))
        .unwrap();
        assert_eq!(r.processes, [RecipeProcess::default()]);
    }
}

#[cfg(test)]
mod run_command_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projects_saved_before_launch_commands_still_load() {
        let p: Project = serde_json::from_value(json!({
            "id": "p1", "name": "demo", "path": "C:/demo", "color": "red"
        }))
        .unwrap();
        assert!(p.run_commands.is_empty());
        let with = json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red",
            "runCommands": [{ "id": "c1", "name": "Front", "command": "npm run dev", "shell": "pwsh", "cwd": "web" }] });
        let p: Project = serde_json::from_value(with.clone()).unwrap();
        assert_eq!(p.run_commands[0].cwd, "web");
        assert_eq!(
            serde_json::to_value(&p).unwrap()["runCommands"],
            with["runCommands"]
        );
    }

    // ----- INT-3: an open harness id, kept as it was -----

    #[test]
    fn a_harness_id_is_kept_as_the_string_it_was_as_an_enumeration() {
        let ids = [
            (EngineId::CLAUDE, "claude"),
            (EngineId::CODEX, "codex"),
            (EngineId::COPILOT, "copilot"),
            (EngineId::CURSOR, "cursor"),
            (EngineId::OPENCODE, "opencode"),
            (EngineId::VIBE, "vibe"),
            (EngineId::AGY, "agy"),
            (EngineId::GEMINI, "gemini"),
        ];
        for (id, text) in ids {
            assert_eq!(serde_json::to_value(id).unwrap(), json!(text));
            assert_eq!(serde_json::from_value::<EngineId>(json!(text)).unwrap(), id);
            assert_eq!(id.as_str(), text);
            assert_eq!(EngineId::new(text), id);
        }
        assert_eq!(EngineId::default(), EngineId::CLAUDE);
    }

    #[test]
    fn a_harness_id_this_version_does_not_have_is_read_and_written_back_as_it_was() {
        let later: EngineId = serde_json::from_value(json!("hermes")).unwrap();
        assert_eq!(later.as_str(), "hermes");
        assert_eq!(later.to_string(), "hermes");
        assert_eq!(later, EngineId::new("hermes"));
        assert_ne!(later, EngineId::CLAUDE);
        assert_eq!(serde_json::to_value(later).unwrap(), json!("hermes"));
        // No id: refused, as the enumeration refused it.
        assert!(serde_json::from_value::<EngineId>(json!("")).is_err());
        assert!(serde_json::from_value::<EngineId>(json!(3)).is_err());
    }

    #[test]
    fn an_agent_a_ticket_or_a_project_on_a_harness_this_version_lacks_is_read_and_kept() {
        // An agent of 1.7 (no harness: Claude Code), of 2.1 (Codex), of a later version (one this
        // version does not have, kept as it is).
        let old: AgentMeta =
            serde_json::from_value(json!({"id": "a1", "projectId": "p1"})).unwrap();
        assert_eq!(old.engine, EngineId::CLAUDE);
        let codex: AgentMeta =
            serde_json::from_value(json!({"id": "a2", "projectId": "p1", "engine": "codex"}))
                .unwrap();
        assert_eq!(codex.engine, EngineId::CODEX);
        let later: AgentMeta =
            serde_json::from_value(json!({"id": "a3", "projectId": "p1", "engine": "hermes"}))
                .unwrap();
        assert_eq!(later.engine.as_str(), "hermes");
        assert_eq!(
            serde_json::to_value(&later).unwrap()["engine"],
            json!("hermes")
        );
        let ticket: Ticket = serde_json::from_value(json!({
            "id": "t1", "projectId": "p1", "key": "ATL-1", "engine": "hermes"
        }))
        .unwrap();
        assert_eq!(ticket.engine.map(EngineId::as_str), Some("hermes"));
        assert_eq!(
            serde_json::to_value(&ticket).unwrap()["engine"],
            json!("hermes")
        );
        let project: Project = serde_json::from_value(json!({
            "id": "p1", "name": "P", "path": "/p", "color": "red", "engine": "hermes"
        }))
        .unwrap();
        assert_eq!(project.engine.map(EngineId::as_str), Some("hermes"));
        assert_eq!(
            serde_json::to_value(&project).unwrap()["engine"],
            json!("hermes")
        );
    }
}
