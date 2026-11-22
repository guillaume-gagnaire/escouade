//! The team server's client in the app (TS-3, `docs/TEAM-SERVER.md` §3.3 to §3.5 and §3.7): the first time the app
//! speaks to a team server. `escouade-team` does the work; this module gives it what is the app's:
//!
//! - **the keychain**: the app's `SecretStore` (Windows' Credential Manager, macOS' Keychain), each name under a prefix
//!   of this data folder (`teams/device.id`), so two data folders of one machine (two users, the end-to-end tests)
//!   never share a team's keys; without a keychain, the files the crate falls back to, in `teams/secrets`;
//! - **the network**: the app's HTTP client (`tls::app_builder`: its proxy, its authorities), and the same proxy and
//!   TLS for the live connection (`tls::proxy_for`, `tls::config`);
//! - **the folder** `teams` of the data: each team's state, and `teams.json`, what the window lists (a name, the
//!   server, whether the recovery words were confirmed, the alarm);
//! - the **commands** of the screens (`commands.rs`, `team_*`), and a **watch** that reads each team's stream and does
//!   what an online device does for the others (`Team::serve`: let an invitee in, register a member's token, keep the
//!   server's roles in step with the roster, read the second uses of an invitation).
//!
//! **The alarm.** A fork, a rollback, a rewritten entry: the server shows what it should not. The team is then
//! stopped on this device: nothing more is read nor sent (no invitation, no admission, no new device, no new recovery
//! key), and the window says « Le serveur montre deux versions de l'équipe : contacte un administrateur », with the
//! six words of the head to compare, « Réessayer » (a reading that passes lifts it) and « Retirer l'équipe de cet
//! appareil ».
//!
//! Nothing of it is shown in the spaces nor in the team's settings: that is TS-4.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use escouade_team::crypto::invite::{InviteLink, DEFAULT_LIFETIME};
use escouade_team::crypto::roster::EscrowPublic;
use escouade_team::crypto::{MemberId, Purpose, RecoveryKey, Role};
use escouade_team::pairing::{PairCode, Pairing};
use escouade_team::team::{InvitationInfo, Invited, OWN_DEVICE_LIFETIME};
use escouade_team::{
    Alarm, ClientError, CryptoError, Error as TeamError, HttpTransport, KeyPlace, Keychain,
    Options, PairState, Setup, Team, Transport, Vault,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::integrations::secrets::SecretStore;
use crate::model::Settings;

/// What the window says of a fork, everywhere it shows it.
pub const FORK: &str = "Le serveur montre deux versions de l’équipe : contacte un administrateur.";

/// How long a new device waits for the device in the team to answer its short code.
const PAIR_WAIT: Duration = Duration::from_secs(120);

/// The longest name of a team.
const MAX_NAME: usize = 80;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn random_hex<const N: usize>() -> String {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("the system gives randomness");
    hex::encode(b)
}

// === The keychain ===

/// The app's `SecretStore` as the crate's `Keychain`: each name under this data folder's prefix.
struct AppKeychain {
    store: Arc<dyn SecretStore>,
    prefix: String,
}

impl AppKeychain {
    fn name(&self, name: &str) -> String {
        format!("{}{name}", self.prefix)
    }
}

impl Keychain for AppKeychain {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        self.store
            .get(&self.name(name))
            .map_err(|e| format!("{e:#}"))
    }

    fn set(&self, name: &str, secret: &str) -> Result<(), String> {
        self.store
            .set(&self.name(name), secret)
            .map_err(|e| format!("{e:#}"))
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.store
            .delete(&self.name(name))
            .map_err(|e| format!("{e:#}"))
    }
}

// === Addresses and links ===

/// Whether `host` (a name or an address, a port or not) is this machine: the one place a server is reached in http.
pub fn is_loopback(host: &str) -> bool {
    let bare = host
        .rsplit_once(':')
        .filter(|(h, p)| {
            !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && !h.ends_with(':')
        })
        .map_or(host, |(h, _)| h)
        .trim_start_matches('[')
        .trim_end_matches(']');
    bare.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// The base of the server (`https://team.acme.fr`, `http://127.0.0.1:8443`) and its host as the links name it
/// (`team.acme.fr`, `127.0.0.1:8443`), from what was typed: an address with its scheme or not, never a path. http is
/// for this machine only.
pub fn address(typed: &str) -> Result<(String, String), String> {
    let typed = typed.trim().trim_end_matches('/');
    if typed.is_empty() {
        return Err("Donne l’adresse du serveur d’équipe (team.exemple.fr).".into());
    }
    let with_scheme = if typed.contains("://") {
        typed.to_string()
    } else {
        format!("https://{typed}")
    };
    let url = reqwest::Url::parse(&with_scheme)
        .map_err(|_| format!("« {typed} » n’est pas une adresse de serveur."))?;
    let host = url
        .host_str()
        .ok_or_else(|| format!("« {typed} » n’est pas une adresse de serveur."))?;
    if url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("L’adresse du serveur s’écrit sans chemin : team.exemple.fr.".into());
    }
    let host = match url.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.to_string(),
    };
    let base = match url.scheme() {
        "https" if is_loopback(&host) && !typed.starts_with("https://") => format!("http://{host}"),
        "https" => format!("https://{host}"),
        "http" if is_loopback(&host) => format!("http://{host}"),
        "http" => {
            return Err(
                "Un serveur d’équipe se joint en https (http seulement sur cette machine).".into(),
            )
        }
        other => {
            return Err(format!(
                "« {other}:// » : un serveur d’équipe se joint en https."
            ))
        }
    };
    Ok((base, host))
}

/// The base of the server a link names: https, but http on this machine.
fn base_of(host: &str) -> String {
    if is_loopback(host) {
        format!("http://{host}")
    } else {
        format!("https://{host}")
    }
}

/// An invitation link as pasted or opened: `https://<server>/join/<id>#…`, or `escouade://team/join/<server>/<id>#…`
/// (what the server's page hands to the app). The secret is after the `#`.
pub fn parse_link(text: &str) -> Result<InviteLink, String> {
    let text: String = text.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let bad = || {
        "Ce n’est pas un lien d’invitation d’équipe complet (il finit par #s=…&t=…).".to_string()
    };
    let https = match text.get(..20) {
        Some(head) if head.eq_ignore_ascii_case("escouade://team/join") => {
            let rest = text[20..].trim_start_matches('/');
            let (before, fragment) = rest.split_once('#').ok_or_else(bad)?;
            let (host, id) = before.rsplit_once('/').ok_or_else(bad)?;
            format!("https://{host}/join/{id}#{fragment}")
        }
        _ => text.replacen("http://", "https://", 1),
    };
    InviteLink::parse(&https).map_err(|_| bad())
}

/// The deep link of an invitation: what the server's page opens (`escouade://team/join/<server>/<id>#…`).
pub fn deep_link(link: &InviteLink) -> String {
    let url = link.to_url();
    let fragment = url.split_once('#').map_or("", |(_, f)| f);
    format!(
        "escouade://team/join/{}/{}#{fragment}",
        link.server,
        hex::encode(link.id)
    )
}

/// The QR code of a link, as SVG, dark on light whatever the theme (the cameras read it so).
pub fn qr_svg(text: &str) -> String {
    use qrcode::render::svg;
    match qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(220, 220)
            .quiet_zone(true)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build(),
        Err(e) => {
            log::warn!("teams: no QR code: {e}");
            String::new()
        }
    }
}

/// What the window says of an error of the team client, in French.
pub fn say(e: &TeamError) -> String {
    if e.alarm().is_some() {
        return FORK.into();
    }
    match e {
        TeamError::InvitationUsed => {
            "Cette invitation a déjà servi : demande un nouveau lien à son administrateur, qui est prévenu.".into()
        }
        TeamError::InvitationUnknown => "Le serveur ne connaît pas cette invitation : elle a expiré, ou un administrateur ne l’a pas encore validée. Réessaie dans un moment, ou demande un nouveau lien.".into(),
        TeamError::WrongCode => "Le code ne correspond pas. Un code ne sert qu’à un essai : demandes-en un nouveau sur l’autre appareil.".into(),
        TeamError::CodeTried => "Ce code a déjà eu son essai : demandes-en un nouveau sur l’autre appareil.".into(),
        TeamError::CodeGone => "Ce code n’existe plus (10 minutes passées, ou il a servi) : demandes-en un nouveau.".into(),
        TeamError::NoRecoveryWrap => "Le serveur ne garde aucune clé de secours de cette équipe.".into(),
        TeamError::Crypto(CryptoError::Words) => "Ces mots ne sont pas une clé de secours : vérifie-les (24 mots, dans l’ordre).".into(),
        TeamError::Crypto(CryptoError::NotAdmin) => "Seul un administrateur de l’équipe peut le faire.".into(),
        TeamError::Crypto(CryptoError::Decrypt) => "Ces mots n’ouvrent pas la dernière clé de secours de l’équipe : est-ce la clé en vigueur ?".into(),
        TeamError::Client(ClientError::Net(m)) => format!("Le serveur d’équipe ne répond pas : {m}"),
        TeamError::Client(ClientError::SignInRefused) => "Le serveur refuse cet appareil : il a été retiré de l’équipe.".into(),
        TeamError::Client(c @ (ClientError::ClientTooOld | ClientError::ServerTooOld)) => c.to_string(),
        TeamError::Client(c) if c.code() == Some("bad_token") || c.code() == Some("token_used") => {
            "Ce jeton n’est pas (ou plus) valable : il ne sert qu’une fois.".into()
        }
        e => e.to_string(),
    }
}

// === What the window shows ===

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlarmView {
    /// `fork`, `rollback`, `rewritten` or `journal` (a project's operations played again, reordered, dropped, forged).
    pub kind: String,
    pub epoch: Option<u64>,
    /// Seconds since the epoch.
    pub at: u64,
}

impl AlarmView {
    fn of(alarm: Alarm) -> AlarmView {
        let (kind, epoch) = match alarm {
            Alarm::Fork { epoch } => ("fork", Some(epoch)),
            Alarm::Rollback => ("rollback", None),
            Alarm::Rewritten => ("rewritten", None),
            Alarm::Journal => ("journal", None),
        };
        AlarmView {
            kind: kind.into(),
            epoch,
            at: now(),
        }
    }
}

/// What this data folder keeps of a team, beside the crate's state (`teams.json`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Known {
    id: String,
    name: String,
    base: String,
    host: String,
    /// The six words of the team, as its link gave them (before the device is in) or as the group says.
    fingerprint: Vec<String>,
    recovery_confirmed: bool,
    alarm: Option<AlarmView>,
    last_error: Option<String>,
    /// The last version of the team's settings the member saw (« Voir » on the band, TS-13).
    settings_seen: Option<u64>,
    /// The name this device signs the team's settings with (« Claire Martin »).
    signer_name: String,
    /// When the team's server last answered this device (seconds, TS-15: « sans contact depuis N heures »).
    last_contact: Option<u64>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvitationView {
    pub id: String,
    /// `member` (a new member) or `device` (another device of a member).
    pub kind: String,
    pub role: Option<String>,
    pub expires_at: u64,
    pub used: bool,
    pub ready: bool,
    pub reused_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamView {
    pub id: String,
    pub name: String,
    pub server: String,
    /// The device is in the group (its Welcome came, or it made the team).
    pub joined: bool,
    /// `admin`, `member`, `recovery`, or `none` while it waits.
    pub role: String,
    pub admins: usize,
    pub members: usize,
    pub epoch: Option<u64>,
    /// The team's six words, to compare out of band.
    pub fingerprint: Vec<String>,
    /// The six words of the current head (the same on every device of an honest server).
    pub head: Vec<String>,
    /// Who holds the organisation's escrow key, when the team has one: shown to every member.
    pub escrow: Option<String>,
    /// The keys are in a file, the keychain refused them (Linux without Secret Service).
    pub keys_in_file: bool,
    pub recovery_confirmed: bool,
    /// The recovery words are still held, to show (until they are confirmed).
    pub words_pending: bool,
    pub alarm: Option<AlarmView>,
    pub invitations: Vec<InvitationView>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub team: TeamView,
    /// The 24 words, shown once.
    pub words: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InviteView {
    pub url: String,
    pub deep_link: String,
    pub expires_at: u64,
    pub ready: bool,
    /// The link's QR code, SVG.
    pub qr: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JoinPreview {
    pub server: String,
    pub fingerprint: Vec<String>,
    /// Another device of a member (a QR code, a short code), not a new member.
    pub own_device: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PairView {
    /// `K7Q4-M2XZ`.
    pub code: String,
    pub server: String,
    pub expires_at: u64,
    /// `waiting`, `waitingForToken`, `sent`, `wrongCode`.
    pub state: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EscrowPair {
    pub public: String,
    pub secret: String,
    pub holder: String,
}

fn pair_state(s: &PairState) -> String {
    match s {
        PairState::Waiting => "waiting",
        PairState::WaitingForToken => "waitingForToken",
        PairState::Sent => "sent",
        PairState::WrongCode => "wrongCode",
    }
    .into()
}

fn role_name(r: Role) -> String {
    match r {
        Role::Admin => "admin",
        Role::Member => "member",
    }
    .into()
}

fn invitation_view(i: &InvitationInfo) -> InvitationView {
    let (kind, role) = match &i.purpose {
        Purpose::NewMember(r) => ("member", Some(role_name(*r))),
        Purpose::OwnDevice(_) => ("device", None),
    };
    InvitationView {
        id: hex::encode(i.id),
        kind: kind.into(),
        role,
        expires_at: i.expires_at,
        used: i.used,
        ready: i.ready,
        reused_at: i.reused_at,
    }
}

fn invite_view(invited: &Invited) -> InviteView {
    InviteView {
        url: invited.url.clone(),
        deep_link: deep_link(&invited.link),
        expires_at: invited.expires_at,
        ready: invited.ready,
        qr: qr_svg(&invited.url),
    }
}

fn words(w: [&'static str; 6]) -> Vec<String> {
    w.iter().map(|s| s.to_string()).collect()
}

fn clean_name(name: &str, host: &str) -> String {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME)
        .collect();
    if name.is_empty() {
        host.to_string()
    } else {
        name
    }
}

// === The teams of this data folder ===

pub(crate) type Shared = Arc<tokio::sync::Mutex<Team>>;

pub struct Teams {
    root: PathBuf,
    keychain: Arc<dyn Keychain>,
    rescue_keychain: Arc<dyn Keychain>,
    settings: Arc<dyn Fn() -> Settings + Send + Sync>,
    changed: Arc<dyn Fn() + Send + Sync>,
    known: Mutex<Vec<Known>>,
    open: Mutex<BTreeMap<String, Shared>>,
    /// The recovery words of a team, held until the owner typed three of them back (or the app stops: then a new
    /// key is offered).
    words: Mutex<BTreeMap<String, RecoveryKey>>,
    pairings: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<Pairing>>>>,
    /// The live feed of each team (TS-5): a frame of the server reads the team at once, instead of waiting for the
    /// watch.
    live: Mutex<BTreeMap<String, tokio::task::JoinHandle<()>>>,
    /// What each team's signed `.escouade.yml` says on this device, as last read (TS-13).
    settings_views: Mutex<BTreeMap<String, escouade_team::settings::View>>,
    /// Told the version in force of a team's settings (None: the device left it): the core applies it.
    on_settings: Mutex<Option<SettingsSink>>,
    /// Told a team was read (its projects too): the core brings the docs of its projects on the server in step (TS-9).
    on_docs: Mutex<Option<DocsSink>>,
    /// Told a team was read (its projects too): the core brings the Kanban of its projects in step (TS-6).
    on_projects: Mutex<Option<ProjectsSink>>,
    /// The teams whose live feed told of a claim (a ticket taken, renewed, given back) since their Kanban last read the
    /// claims (TS-6).
    claim_frames: Mutex<std::collections::BTreeSet<String>>,
    /// What each team's circuit breakers say on this device, as last read (TS-15).
    breaker_views: Mutex<BTreeMap<String, escouade_team::breaker::View>>,
    /// Told the breakers in force of a team (None: the device left it): the core applies them.
    on_breakers: Mutex<Option<BreakersSink>>,
    /// The teams whose last reading failed: their breakers are read again before a start (`Core::team_gate`).
    stale: Mutex<std::collections::BTreeSet<String>>,
    /// What each team's costs are on this device, as last read (TS-16).
    cost_views: Mutex<BTreeMap<String, escouade_team::costs::View>>,
    /// This device's member in each team (hex), as last read.
    cost_members: Mutex<BTreeMap<String, String>>,
}

/// Where a reading of a team is told: the team's id (TS-9).
pub type DocsSink = Arc<dyn Fn(&str) + Send + Sync>;

/// Where a reading of a team is told: its id (the core's Kanban of its projects, TS-6).
pub type ProjectsSink = Arc<dyn Fn(&str) + Send + Sync>;

/// Where a team's settings go: the team's id, its name, what applies.
pub type SettingsSink =
    Arc<dyn Fn(&str, &str, Option<&escouade_team::settings::View>) + Send + Sync>;

#[path = "teams_settings.rs"]
mod team_files;
pub use team_files::{SettingsDraft, TeamSettingsView};

#[path = "teams_catalog.rs"]
mod team_catalog;
pub use team_catalog::{CatalogOf, MemberView};

#[path = "teams_breakers.rs"]
mod team_breakers;
pub use team_breakers::{
    BreakerAsk, BreakersSink, BreakersView, WithdrawnView, GATE as BREAKER_GATE,
};
#[path = "teams_costs.rs"]
mod team_costs;

#[path = "teams_plugins.rs"]
mod team_plugins;
pub use team_costs::month_now;
pub use team_plugins::StoreOf;

impl Teams {
    /// The teams of the data folder's `teams` (made if needed), each opened from its state and the keychain.
    pub fn open(
        root: PathBuf,
        store: Arc<dyn SecretStore>,
        settings: Arc<dyn Fn() -> Settings + Send + Sync>,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Teams {
        if let Err(e) = std::fs::create_dir_all(&root) {
            log::warn!("teams: {}: {e}", root.display());
        }
        let id_file = root.join("device.id");
        let install = std::fs::read_to_string(&id_file)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit()))
            .unwrap_or_else(|| {
                let id = random_hex::<8>();
                if let Err(e) = std::fs::write(&id_file, &id) {
                    log::warn!("teams: {}: {e}", id_file.display());
                }
                id
            });
        let known: Vec<Known> = std::fs::read(root.join("teams.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let teams = Teams {
            keychain: Arc::new(AppKeychain {
                store: store.clone(),
                prefix: format!("{install}."),
            }),
            rescue_keychain: Arc::new(AppKeychain {
                store,
                prefix: format!("{install}.rescue."),
            }),
            root,
            settings,
            changed,
            known: Mutex::new(known),
            open: Mutex::default(),
            words: Mutex::default(),
            pairings: Mutex::default(),
            live: Mutex::default(),
            settings_views: Mutex::default(),
            on_settings: Mutex::default(),
            on_docs: Mutex::default(),
            on_projects: Mutex::default(),
            claim_frames: Mutex::default(),
            breaker_views: Mutex::default(),
            on_breakers: Mutex::default(),
            stale: Mutex::default(),
            cost_views: Mutex::default(),
            cost_members: Mutex::default(),
        };
        let known = teams.known.lock().clone();
        for k in known {
            let Ok(id) = hex_id(&k.id) else { continue };
            let opened = teams
                .setup(&k.base, &k.host)
                .and_then(|setup| Team::open(setup, id).map_err(|e| say(&e)));
            match opened {
                Ok(team) => {
                    teams
                        .open
                        .lock()
                        .insert(k.id.clone(), Arc::new(tokio::sync::Mutex::new(team)));
                }
                Err(e) => {
                    log::warn!("teams: a team not opened: {e}");
                    teams.update(&k.id, |k| k.last_error = Some(e.clone()));
                }
            }
        }
        teams
    }

    fn transport(&self, base: &str) -> Result<Arc<dyn Transport>, String> {
        let settings = (self.settings)();
        let net = crate::tls::Net::of(&settings);
        let url = reqwest::Url::parse(base).map_err(|e| e.to_string())?;
        let host = url.host_str().unwrap_or_default().to_string();
        let http = crate::tls::app_builder(&settings)
            .map_err(|e| format!("Le proxy des réglages : {e}"))?
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let transport = HttpTransport::new(
            base,
            http,
            Some(crate::tls::config(&net)),
            crate::tls::proxy_for(&net, &host),
        )
        .map_err(|e| e.to_string())?;
        Ok(Arc::new(transport))
    }

    fn setup(&self, base: &str, host: &str) -> Result<Setup, String> {
        Ok(Setup {
            folder: self.root.clone(),
            vault: Vault::new(self.keychain.clone(), self.root.join("secrets")),
            transport: self.transport(base)?,
            server: host.to_string(),
            options: Options::default(),
        })
    }

    fn rescue_setup(&self, base: &str, host: &str) -> Result<Setup, String> {
        Ok(Setup {
            folder: self.root.join("rescue"),
            vault: Vault::new(
                self.rescue_keychain.clone(),
                self.root.join("rescue").join("secrets"),
            ),
            transport: self.transport(base)?,
            server: host.to_string(),
            options: Options::default(),
        })
    }

    fn save(&self) {
        let known = self.known.lock().clone();
        let path = self.root.join("teams.json");
        let tmp = path.with_extension("json.tmp");
        let written = serde_json::to_vec_pretty(&known)
            .map_err(|e| e.to_string())
            .and_then(|b| std::fs::write(&tmp, b).map_err(|e| e.to_string()))
            .and_then(|_| std::fs::rename(&tmp, &path).map_err(|e| e.to_string()));
        if let Err(e) = written {
            log::warn!("teams: {}: {e}", path.display());
        }
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut Known)) {
        {
            let mut known = self.known.lock();
            if let Some(k) = known.iter_mut().find(|k| k.id == id) {
                f(k);
            }
        }
        self.save();
    }

    fn known(&self, id: &str) -> Result<Known, String> {
        self.known
            .lock()
            .iter()
            .find(|k| k.id == id)
            .cloned()
            .ok_or_else(|| "Cette équipe n’est pas sur cet appareil.".to_string())
    }

    /// Where a team of this device is: its server's base, its host and its name (TS-7, the key `team` of
    /// `.escouade.yml`); none for a team this device is not in.
    pub(crate) fn place_of(&self, id: &str) -> Option<(String, String, String)> {
        let k = self.known(id).ok()?;
        Some((k.base, k.host, k.name))
    }

    pub(crate) fn team(&self, id: &str) -> Result<Shared, String> {
        self.open
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| "Cette équipe ne s’ouvre pas sur cet appareil.".to_string())
    }

    /// Refused while the team is under an alarm: nothing goes to the team then.
    fn unblocked(&self, id: &str) -> Result<(), String> {
        if self.known(id)?.alarm.is_some() {
            return Err(format!(
                "{FORK} Rien n’est envoyé à l’équipe depuis cet appareil tant que ce n’est pas réglé."
            ));
        }
        Ok(())
    }

    fn add(&self, known: Known, team: Team) {
        let id = known.id.clone();
        {
            let mut all = self.known.lock();
            all.retain(|k| k.id != id);
            all.push(known);
        }
        self.save();
        self.open
            .lock()
            .insert(id, Arc::new(tokio::sync::Mutex::new(team)));
        (self.changed)();
    }

    fn view_of(&self, known: &Known, team: Option<&Team>) -> TeamView {
        let joined = team.is_some_and(|t| t.epoch().is_ok());
        let words_pending = self.words.lock().contains_key(&known.id);
        let mut view = TeamView {
            id: known.id.clone(),
            name: known.name.clone(),
            server: known.host.clone(),
            joined,
            role: "none".into(),
            admins: 0,
            members: 0,
            epoch: None,
            fingerprint: known.fingerprint.clone(),
            head: Vec::new(),
            escrow: None,
            keys_in_file: team.is_some_and(|t| matches!(t.key_place(), KeyPlace::File(_))),
            recovery_confirmed: known.recovery_confirmed,
            words_pending,
            alarm: known.alarm.clone(),
            invitations: Vec::new(),
            last_error: known.last_error.clone(),
        };
        let Some(team) = team.filter(|_| joined) else {
            return view;
        };
        let me = team.identity();
        if let Ok(roster) = team.roster() {
            view.role = if me.kind == escouade_team::crypto::DeviceKind::Recovery {
                "recovery".into()
            } else {
                roster.role(&me.member).map_or("none".into(), role_name)
            };
            view.admins = roster.admins();
            view.members = roster.members.len();
            view.escrow = roster.escrow.map(|e| e.holder);
        }
        view.epoch = team.epoch().ok();
        if let Ok(f) = team.fingerprint() {
            view.fingerprint = words(f.words());
        }
        view.head = team.head_words().map(words).unwrap_or_default();
        view.invitations = team.invitations().iter().map(invitation_view).collect();
        view
    }

    async fn view(&self, id: &str) -> Result<TeamView, String> {
        let known = self.known(id)?;
        let team = self.open.lock().get(id).cloned();
        Ok(match team {
            Some(t) => {
                let t = t.lock().await;
                self.view_of(&known, Some(&t))
            }
            None => self.view_of(&known, None),
        })
    }

    pub async fn list(&self) -> Vec<TeamView> {
        let ids: Vec<String> = self.known.lock().iter().map(|k| k.id.clone()).collect();
        let mut out = Vec::new();
        for id in ids {
            if let Ok(v) = self.view(&id).await {
                out.push(v);
            }
        }
        out
    }

    // === Creating a team ===

    /// A new escrow key pair for the organisation (C7-B): its public half for the team, its secret to give to whoever
    /// holds it, shown once.
    pub fn escrow_new(&self, holder: &str) -> Result<EscrowPair, String> {
        let (public, secret) = escouade_team::escrow::new_pair(holder).map_err(|e| say(&e))?;
        Ok(EscrowPair {
            public: escouade_team::escrow::public_text(&public),
            secret: secret.to_string(),
            holder: public.holder,
        })
    }

    /// Creates a team on the server with the token of `escouade-team-server init`. Gives its 24 words, shown once.
    pub async fn create(
        &self,
        server: &str,
        token: &str,
        name: &str,
        escrow: Option<(String, String)>,
    ) -> Result<Created, String> {
        let (base, host) = address(server)?;
        let token = token.trim();
        if token.is_empty() {
            return Err("Donne le jeton que `escouade-team-server init` a affiché.".into());
        }
        let escrow: Option<EscrowPublic> = match escrow {
            Some((public, holder)) => {
                Some(escouade_team::escrow::parse_public(&public, &holder).map_err(|e| say(&e))?)
            }
            None => None,
        };
        let setup = self.setup(&base, &host)?;
        let (mut team, recovery) = Team::create(setup, token, MemberId(random_id()), escrow, now())
            .await
            .map_err(|e| say(&e))?;
        let _ = team.sync().await;
        let id = hex::encode(team.id());
        let words: Vec<String> = recovery.words().iter().map(|w| w.to_string()).collect();
        self.words.lock().insert(id.clone(), recovery);
        let known = Known {
            id: id.clone(),
            name: clean_name(name, &host),
            base,
            host,
            fingerprint: team
                .fingerprint()
                .map(|f| words_of(f.words()))
                .unwrap_or_default(),
            ..Known::default()
        };
        let view = self.view_of(&known, Some(&team));
        self.add(known, team);
        Ok(Created { team: view, words })
    }

    /// The owner typed three of the words back: true when they are right (the words are then let go).
    pub fn confirm_words(&self, id: &str, typed: &[(usize, String)]) -> Result<bool, String> {
        let typed: Vec<(usize, &str)> = typed.iter().map(|(i, w)| (*i, w.as_str())).collect();
        let ok = match self.words.lock().get(id) {
            Some(key) => key.confirms(&typed),
            None => {
                return Err(
                    "Ces mots ne sont plus en mémoire : fais une nouvelle clé de secours.".into(),
                )
            }
        };
        if ok {
            self.words.lock().remove(id);
            self.update(id, |k| k.recovery_confirmed = true);
            (self.changed)();
        }
        Ok(ok)
    }

    /// A new recovery key (an administrator's): the old one leaves the roster. Its 24 words, shown once.
    pub async fn new_recovery(&self, id: &str) -> Result<Vec<String>, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let key = t.replace_recovery().await.map_err(|e| self.failed(id, e))?;
        let words: Vec<String> = key.words().iter().map(|w| w.to_string()).collect();
        self.words.lock().insert(id.to_string(), key);
        self.update(id, |k| k.recovery_confirmed = false);
        (self.changed)();
        Ok(words)
    }

    // === Inviting ===

    pub async fn invite(
        &self,
        id: &str,
        role: &str,
        hours: Option<u64>,
    ) -> Result<InviteView, String> {
        self.unblocked(id)?;
        let role = match role {
            "admin" => Role::Admin,
            _ => Role::Member,
        };
        let lifetime = hours.map_or(DEFAULT_LIFETIME, |h| h.clamp(1, 168) * 3600);
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let invited = t
            .invite_for(Purpose::NewMember(role), now(), lifetime)
            .await
            .map_err(|e| self.failed(id, e))?;
        drop(t);
        (self.changed)();
        Ok(invite_view(&invited))
    }

    /// The QR code of another device of one's own: its link, an hour.
    pub async fn own_device(&self, id: &str) -> Result<InviteView, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let me = t.identity().member;
        let invited = t
            .invite_for(Purpose::OwnDevice(me), now(), OWN_DEVICE_LIFETIME)
            .await
            .map_err(|e| self.failed(id, e))?;
        drop(t);
        (self.changed)();
        Ok(invite_view(&invited))
    }

    /// What a link says before joining: its server, the team's six words, whether it is one's own device.
    pub fn join_preview(&self, link: &str) -> Result<JoinPreview, String> {
        let link = parse_link(link)?;
        Ok(JoinPreview {
            server: link.server.clone(),
            fingerprint: words_of(link.team().words()),
            own_device: link.member.is_some(),
        })
    }

    /// Joins by a link: the device enrols and asks; it is in once an administrator online let it in.
    pub async fn join(&self, link: &str, name: &str) -> Result<TeamView, String> {
        let link = parse_link(link)?;
        self.join_by(&link, name).await
    }

    async fn join_by(&self, link: &InviteLink, name: &str) -> Result<TeamView, String> {
        let host = link.server.clone();
        let base = base_of(&host);
        if let Some(k) = self
            .known
            .lock()
            .iter()
            .find(|k| k.fingerprint == words_of(link.team().words()))
        {
            return Err(format!(
                "Cet appareil est déjà dans l’équipe « {} ».",
                k.name
            ));
        }
        let setup = self.setup(&base, &host)?;
        let mut team = Team::join_link(setup, link).await.map_err(|e| say(&e))?;
        let _ = team.sync().await;
        let known = Known {
            id: hex::encode(team.id()),
            name: clean_name(name, &host),
            base,
            host,
            fingerprint: words_of(link.team().words()),
            recovery_confirmed: true,
            ..Known::default()
        };
        let view = self.view_of(&known, Some(&team));
        self.add(known, team);
        Ok(view)
    }

    // === Another device by a short code ===

    pub async fn pair_start(&self, id: &str) -> Result<PairView, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let pairing = t.pair_start(now()).await.map_err(|e| self.failed(id, e))?;
        let view = PairView {
            code: pairing.code.shown(),
            server: t.server().to_string(),
            expires_at: pairing.expires_at,
            state: pair_state(&pairing.state),
        };
        self.pairings
            .lock()
            .insert(id.to_string(), Arc::new(tokio::sync::Mutex::new(pairing)));
        Ok(view)
    }

    /// One step of the pairing under way: the new device's try read, the answer sent.
    pub async fn pair_step(&self, id: &str) -> Result<PairView, String> {
        let pairing = self
            .pairings
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| "Aucun code en cours pour cette équipe.".to_string())?;
        let team = self.team(id)?;
        let mut p = pairing.lock().await;
        let state = {
            let mut t = team.lock().await;
            t.pair_step(&mut p).await.map_err(|e| say(&e))?
        };
        if state == PairState::Sent {
            (self.changed)();
        }
        let server = team.lock().await.server().to_string();
        Ok(PairView {
            code: p.code.shown(),
            server,
            expires_at: p.expires_at,
            state: pair_state(&state),
        })
    }

    pub fn pair_cancel(&self, id: &str) {
        self.pairings.lock().remove(id);
    }

    /// The new device: the server's address and the code shown on the other; it then joins by the link it got.
    pub async fn pair_join(
        &self,
        server: &str,
        code: &str,
        name: &str,
    ) -> Result<TeamView, String> {
        let (base, _) = address(server)?;
        let code = PairCode::parse(code)
            .map_err(|_| "Un code fait 8 caractères, lettres et chiffres.".to_string())?;
        let transport = self.transport(&base)?;
        let link = escouade_team::pairing::fetch_link(&*transport, &code, PAIR_WAIT)
            .await
            .map_err(|e| say(&e))?;
        self.join_by(&link, name).await
    }

    // === Recovery ===

    /// The holder of the recovery key brings the team back on this device (every administrator lost theirs): the
    /// server's address, the operator's token (`escouade-team-server recovery-token`) and the 24 words. This device
    /// becomes an administrator, and the team gets a new recovery key, whose 24 words are shown once.
    pub async fn recover(
        &self,
        server: &str,
        token: &str,
        words: &str,
        name: &str,
    ) -> Result<Created, String> {
        let (base, host) = address(server)?;
        let rescue = self.rescue_setup(&base, &host)?;
        let setup = self.setup(&base, &host)?;
        let (team, key) = Team::recover_onto(rescue, setup, token.trim(), words, now())
            .await
            .map_err(|e| say(&e))?;
        let id = hex::encode(team.id());
        let new_words: Vec<String> = key.words().iter().map(|w| w.to_string()).collect();
        self.words.lock().insert(id.clone(), key);
        let known = Known {
            id: id.clone(),
            name: clean_name(name, &host),
            base,
            host,
            fingerprint: team
                .fingerprint()
                .map(|f| words_of(f.words()))
                .unwrap_or_default(),
            ..Known::default()
        };
        let view = self.view_of(&known, Some(&team));
        self.add(known, team);
        Ok(Created {
            team: view,
            words: new_words,
        })
    }

    // === Reading, the alarm, forgetting ===

    /// What an error does to the team: an alarm stops it on this device; any other is kept to show.
    fn failed(&self, id: &str, e: TeamError) -> String {
        let text = say(&e);
        let known = self.known(id).ok();
        match e.alarm() {
            Some(alarm) => {
                log::warn!("teams: an alarm: {e}");
                self.update(id, |k| k.alarm = Some(AlarmView::of(alarm)));
                (self.changed)();
            }
            // Said once: the watch reads again every few seconds, the same error (offline) is not news.
            None if known.is_some_and(|k| k.last_error.as_deref() == Some(text.as_str())) => {}
            None => {
                let shown = text.clone();
                self.update(id, |k| k.last_error = Some(shown));
                (self.changed)();
            }
        }
        text
    }

    /// Reads the team's stream and does an online device's work. Under an alarm, a reading that passes lifts it.
    pub async fn sync(&self, id: &str) -> Result<TeamView, String> {
        let team = self.team(id)?;
        let before = self.view(id).await?;
        let done = {
            let mut t = team.lock().await;
            let read = t.sync().await;
            if read.is_ok() {
                // The server answered: its circuit breakers first of all (TS-15), so that one set applies before
                // anything else of the reading.
                self.touch(id);
                self.read_breakers(id, &mut t).await;
            } else {
                self.mark_stale(id);
            }
            match read {
                Ok(_) => match t.serve(now()).await {
                    Ok(_) => match sync_projects(&mut t).await {
                        Ok(()) => {
                            self.read_settings(id, &mut t).await;
                            self.read_catalog(&mut t).await;
                            self.read_plugins(&mut t).await;
                            self.read_costs(id, &mut t).await;
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        };
        let read = done.is_ok();
        if read {
            let sink = self.on_projects.lock().clone();
            if let Some(sink) = sink {
                sink(id);
            }
        }
        match done {
            Ok(()) => {
                if self
                    .known(id)
                    .is_ok_and(|k| k.alarm.is_some() || k.last_error.is_some())
                {
                    self.update(id, |k| {
                        k.alarm = None;
                        k.last_error = None;
                    });
                }
            }
            Err(e) => {
                self.failed(id, e);
            }
        }
        if read {
            let sink = self.on_docs.lock().clone();
            if let Some(sink) = sink {
                sink(id);
            }
        }
        let after = self.view(id).await?;
        if after != before {
            (self.changed)();
        }
        Ok(after)
    }

    /// Where a reading of a team is told (the core's docs of its projects on the server, TS-9).
    pub fn set_docs_sink(&self, sink: DocsSink) {
        *self.on_docs.lock() = Some(sink);
    }

    /// Forgets the team on this device: its state and its keys. The group keeps its leaf until an administrator
    /// removes it (TS-4).
    pub async fn forget(&self, id: &str) -> Result<(), String> {
        self.pairings.lock().remove(id);
        if let Some(live) = self.live.lock().remove(id) {
            live.abort();
        }
        self.words.lock().remove(id);
        // Out of the list first: the watch takes it no more. A reading under way holds it a moment: waited for (its
        // state is closed before its folder goes).
        let mut held = self.open.lock().remove(id);
        for _ in 0..300 {
            let Some(team) = held.take() else { break };
            match Arc::try_unwrap(team) {
                Ok(team) => {
                    team.into_inner().forget().map_err(|e| say(&e))?;
                    break;
                }
                Err(team) => {
                    held = Some(team);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        if let Some(team) = held {
            self.open.lock().insert(id.to_string(), team);
            return Err("L’équipe est en cours d’usage : réessaie.".to_string());
        }
        self.forget_settings(id);
        self.forget_breakers(id);
        self.cost_views.lock().remove(id);
        self.cost_members.lock().remove(id);
        self.known.lock().retain(|k| k.id != id);
        self.save();
        (self.changed)();
        Ok(())
    }

    /// What the status bar says of a team (§2.5): whether its server answers, and how many changes of this device
    /// wait to go (« Hors ligne · 3 changements en attente »). TS-4 and TS-6 show it.
    pub async fn status(&self, id: &str) -> Result<SyncStatusView, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let state = t.sync_state();
        Ok(SyncStatusView::of(
            state.online,
            state.pending,
            t.projects().len(),
        ))
    }

    /// Whether a claim of the team came by its live feed since the last call (the Kanban reads the claims again then).
    pub fn take_claim_frame(&self, id: &str) -> bool {
        self.claim_frames.lock().remove(id)
    }

    /// Where a reading of a team is told (the core's Kanban of its projects, TS-6).
    pub fn set_projects_sink(&self, sink: ProjectsSink) {
        *self.on_projects.lock() = Some(sink);
    }

    /// Reads every team not under an alarm, and does what an online device does: forever, every `every`. Each team
    /// also has its live feed (TS-5): a frame of the server (an entry of the team or of a project, a connection come
    /// back, frames dropped) reads it at once; the watch is what is left when the feed cannot connect.
    pub async fn watch(self: Arc<Self>, every: Duration) {
        loop {
            let ids: Vec<String> = self
                .known
                .lock()
                .iter()
                .filter(|k| k.alarm.is_none())
                .map(|k| k.id.clone())
                .collect();
            for id in ids {
                if self.open.lock().contains_key(&id) {
                    self.ensure_live(&id).await;
                    let _ = self.sync(&id).await;
                }
            }
            tokio::time::sleep(every).await;
        }
    }

    /// Starts the live feed of a team, unless it runs. The task holds the feed alone, never the team: forgetting the
    /// team stops it.
    async fn ensure_live(self: &Arc<Self>, id: &str) {
        if self.live.lock().get(id).is_some_and(|h| !h.is_finished()) {
            return;
        }
        let Some(team) = self.open.lock().get(id).cloned() else {
            return;
        };
        let mut feed = team.lock().await.live();
        drop(team);
        let me = Arc::downgrade(self);
        let key = id.to_string();
        let handle = tokio::spawn(async move {
            while let Some(event) = feed.next().await {
                if event == escouade_team::LiveEvent::Revoked {
                    break;
                }
                let claim =
                    |e: &escouade_team::LiveEvent| matches!(e, escouade_team::LiveEvent::Claim(..));
                let mut claims = claim(&event);
                let mut others = !claims;
                // A burst of frames is read once.
                for _ in 0..50 {
                    match tokio::time::timeout(Duration::from_millis(100), feed.next()).await {
                        Ok(Some(escouade_team::LiveEvent::Revoked)) | Ok(None) => return,
                        Ok(Some(e)) => {
                            let c = claim(&e);
                            claims |= c;
                            others |= !c;
                        }
                        Err(_) => break,
                    }
                }
                let Some(teams) = me.upgrade() else { break };
                if claims {
                    teams.claim_frames.lock().insert(key.clone());
                }
                if !teams.known(&key).is_ok_and(|k| k.alarm.is_none()) {
                    continue;
                }
                if others {
                    let _ = teams.sync(&key).await;
                } else {
                    // Claims alone: nothing to read of the streams, the Kanban reads the claims (TS-6).
                    let sink = teams.on_projects.lock().clone();
                    if let Some(sink) = sink {
                        sink(&key);
                    }
                }
            }
        });
        self.live.lock().insert(id.to_string(), handle);
    }
}

/// How often the watch reads the teams when their live feed brings nothing: 15 s (the live feed reads them within a
/// second, TS-5), or `ESCOUADE_TEAMS_EVERY_MS` (the end-to-end tests) in a build of development (SEC-13).
pub fn watch_every() -> Duration {
    std::env::var("ESCOUADE_TEAMS_EVERY_MS")
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(Duration::from_secs(15), |ms| {
            Duration::from_millis(ms.max(200))
        })
}

/// Reads the projects of the team this device follows (TS-5) and sends what waits: the copies the window will read
/// (TS-6). A project kept to members this device is not among is left aside.
async fn sync_projects(t: &mut Team) -> Result<(), TeamError> {
    for project in t.projects() {
        match t.sync_project(project).await {
            Ok(_) | Err(TeamError::NoAccess) | Err(TeamError::NoProject) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// The synchronization of a team as the status bar says it (§2.5).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusView {
    /// The server answered the last call; None before the first.
    pub online: Option<bool>,
    /// The changes of this device waiting to go, every project together.
    pub pending: usize,
    /// The projects of the team this device follows.
    pub projects: usize,
    /// « Hors ligne · 3 changements en attente », « 2 changements en attente », or empty.
    pub label: String,
}

impl SyncStatusView {
    pub fn of(online: Option<bool>, pending: usize, projects: usize) -> SyncStatusView {
        let waiting = match pending {
            0 => String::new(),
            1 => "1 changement en attente".to_string(),
            n => format!("{n} changements en attente"),
        };
        let label = match (online, waiting.is_empty()) {
            (Some(false), true) => "Hors ligne".to_string(),
            (Some(false), false) => format!("Hors ligne · {waiting}"),
            _ => waiting,
        };
        SyncStatusView {
            online,
            pending,
            projects,
            label,
        }
    }
}

fn words_of(w: [&'static str; 6]) -> Vec<String> {
    words(w)
}

fn random_id() -> [u8; 16] {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("the system gives randomness");
    b
}

fn hex_id(text: &str) -> Result<[u8; 16], String> {
    let mut out = [0u8; 16];
    hex::decode_to_slice(text, &mut out).map_err(|_| "an id".to_string())?;
    Ok(out)
}
