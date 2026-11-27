//! The driving of the browsers (ESC-679, AB-7 with AP-6; docs/AGENT-BROWSERS.md § 2 bis.2, § 3.3, § 5, § 9): what
//! `drive_browser` may do on a row of « Navigateurs », and how it does it.
//!
//! - **Two tiers, three kinds of rows** (`decide`): to show (scroll, highlight, widths, reload, the history, wait; a
//!   version on an agent's page) goes without asking everywhere; to act (`tab`, `click`, `type`, `select`, `press`,
//!   `navigate`) goes without asking on the project's launch, with the user's leave outside it (choice C4: once per
//!   origin and per browser, the leave of `read_browser`), and on an agent's page `tab` alone.
//! - **What is never done** (`forbid`), even with a leave: a file sent, a download, a field of a form that goes to
//!   another origin, a secret of Escouade typed (`holds_secret`), a password outside the launch; on the launch, a
//!   password waits for the user's leave (choice C2, once per browser, its value never read back).
//! - **The targets** (`Target`): an `#id`, a bounded CSS selector, `text=` (the first visible clickable element of that
//!   text), or on an agent's page a key of its template (`criteria.2`), turned into a selector by its anchors.
//! - **The bounds** (`Pace`): 4 gestures a second and 600 an hour, per row.
//! - **The rule of the hand** (`Drives`): the agent drives a row on screen alone; the user's first gesture in it (an
//!   `isTrusted` event that is not the agent's, counted in Escouade's world and taken by polling, at each call and
//!   every 500 ms; under WebKit, whose isolated world keeps nothing between two calls, counted by a relay in the page,
//!   `browserdrive-gestures.js`) or « Reprendre la main » suspends the driving until « La rendre »; « Ne plus laisser piloter » ends
//!   it until the row closes. `wait_user` (choice C1) waits for « Continuer », 5 minutes at most.
//! - **One channel**: the fixed scripts of `browserdrive.js`, in the isolated world, by `browserread`'s bridge; under
//!   Windows the clicks, keys and text are real inputs of the DevTools protocol (`Input.*`, of the closed list); under
//!   WebKit, the isolated world's synthetic ones.
//!
//! What the page says is a third party's: nothing it holds triggers anything; the agent gets « found » or not, a box,
//! the address and the title.

use crate::devpage::PageMode;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tauri::{Runtime, Webview};

/// Escouade's library of the driving, run in its world.
pub(crate) const LIB: &str = include_str!("browserdrive.js");
/// The count of the user's gestures in the page's world under WebKit, whose isolated world keeps no listener from one
/// call to the next (ESC-692): an initialization script of every browser and agent's page there.
pub const GESTURES: &str = include_str!("browserdrive-gestures.js");

/// The gestures of a row in a second, and in an hour.
pub const PER_SECOND: usize = 4;
pub const PER_HOUR: usize = 600;
/// How long `wait_user` waits for « Continuer ».
pub const WAIT_USER: Duration = Duration::from_secs(300);
/// How long `wait` waits at most.
pub const WAIT_MAX: Duration = Duration::from_secs(30);
/// How long a gesture waits for the page to settle.
pub const SETTLE: Duration = Duration::from_millis(1500);
/// How often the core takes the user's gestures of a driven row.
pub const POLL: Duration = Duration::from_millis(500);
/// The tail of an agent's gesture: what the page makes of it in that time is not the user's. Its real inputs (CDP)
/// are Windows's alone; under WebKit the gesture is a script of the isolated world.
#[cfg(windows)]
pub const TAIL_MS: u64 = 400;
/// How long after an agent's gesture on a page a choice it proposes is taken for the agent's (AP-4).
pub const CHOICE_QUIET: Duration = Duration::from_secs(2);
/// The most characters `type` takes, a label, a target.
pub const TEXT_MAX: usize = 2000;
pub const LABEL_MAX: usize = 60;
pub const TARGET_MAX: usize = 200;
/// How far `scroll` goes, at most, either way.
pub const SCROLL_MAX: i64 = 5000;

// ---------- the actions and their tiers ----------

/// A key `press` gives: the closed list (Enter, Escape, Tab, the arrows, Page up and down, Home, End).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
}

impl Key {
    /// Its `key`, its `code` and its virtual key code of Windows.
    pub fn codes(self) -> (&'static str, &'static str, i64) {
        match self {
            Key::Enter => ("Enter", "Enter", 13),
            Key::Escape => ("Escape", "Escape", 27),
            Key::Tab => ("Tab", "Tab", 9),
            Key::ArrowUp => ("ArrowUp", "ArrowUp", 38),
            Key::ArrowDown => ("ArrowDown", "ArrowDown", 40),
            Key::ArrowLeft => ("ArrowLeft", "ArrowLeft", 37),
            Key::ArrowRight => ("ArrowRight", "ArrowRight", 39),
            Key::PageUp => ("PageUp", "PageUp", 33),
            Key::PageDown => ("PageDown", "PageDown", 34),
            Key::Home => ("Home", "Home", 36),
            Key::End => ("End", "End", 35),
        }
    }

    /// The key of this name, of the closed list alone.
    pub fn parse(name: &str) -> Option<Key> {
        const ALL: [Key; 11] = [
            Key::Enter,
            Key::Escape,
            Key::Tab,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::PageUp,
            Key::PageDown,
            Key::Home,
            Key::End,
        ];
        ALL.into_iter().find(|k| k.codes().0 == name)
    }
}

/// An element of a row, as the core aims at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A CSS selector (an `#id` is one), checked.
    Css(String),
    /// `text=…`: the first visible clickable element of that text.
    Text(String),
}

impl Target {
    /// What the library takes.
    pub fn arg(&self) -> Value {
        match self {
            Target::Css(c) => json!({ "css": c }),
            Target::Text(t) => json!({ "text": t }),
        }
    }
}

/// What `drive_browser` does, one per call.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Navigate(String),
    Back,
    Forward,
    Reload,
    /// The load, or an element (`Some`), `timeout` at most.
    Wait(Option<Target>, Duration),
    ScrollTo(Target),
    Scroll(i64),
    Highlight(Target, String),
    Clear,
    Width(PageMode),
    Tab(Target),
    Click(Target),
    Type {
        target: Target,
        text: String,
        submit: bool,
    },
    Select(Target, String),
    Press(Key),
    Version(u32),
}

/// The tier of a gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Show,
    Act,
}

impl Act {
    pub fn tier(&self) -> Tier {
        match self {
            Act::Navigate(_)
            | Act::Tab(_)
            | Act::Click(_)
            | Act::Type { .. }
            | Act::Select(..)
            | Act::Press(_) => Tier::Act,
            _ => Tier::Show,
        }
    }

    /// Its name, as the tool takes it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn name(&self) -> &'static str {
        match self {
            Act::Navigate(_) => "navigate",
            Act::Back => "back",
            Act::Forward => "forward",
            Act::Reload => "reload",
            Act::Wait(..) => "wait",
            Act::ScrollTo(_) => "scroll_to",
            Act::Scroll(_) => "scroll",
            Act::Highlight(..) => "highlight",
            Act::Clear => "clear",
            Act::Width(_) => "width",
            Act::Tab(_) => "tab",
            Act::Click(_) => "click",
            Act::Type { .. } => "type",
            Act::Select(..) => "select",
            Act::Press(_) => "press",
            Act::Version(_) => "version",
        }
    }

    /// The element it aims at, if any.
    pub fn target(&self) -> Option<&Target> {
        match self {
            Act::Wait(t, _) => t.as_ref(),
            Act::ScrollTo(t)
            | Act::Highlight(t, _)
            | Act::Tab(t)
            | Act::Click(t)
            | Act::Type { target: t, .. }
            | Act::Select(t, _) => Some(t),
            _ => None,
        }
    }
}

/// The kind of a row, for the tiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// An agent's page (mode 1).
    AgentPage,
    /// A tab on the project's launch.
    Launch,
    /// A tab outside the launch, on this origin.
    Outside(String),
}

/// Why a gesture is refused before anything is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// To act on an agent's page: `tab` alone.
    ActOnPage,
    /// The history or an address on an agent's page: `version` there.
    TabOnly,
    /// `version` on a tab.
    PageOnly,
}

/// What `decide` says of a gesture on a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Go,
    /// Outside the launch, to act: the user's leave first (C4).
    Leave,
    Refused(Refusal),
}

/// The tiers of § 9: what a gesture of this kind may do on this row, `leave` the user's leave to act there.
pub fn decide(line: &Line, act: &Act, leave: bool) -> Verdict {
    match (line, act) {
        (Line::AgentPage, Act::Version(_) | Act::Tab(_)) => Verdict::Go,
        (Line::AgentPage, Act::Navigate(_) | Act::Back | Act::Forward) => {
            Verdict::Refused(Refusal::TabOnly)
        }
        (Line::AgentPage, a) if a.tier() == Tier::Act => Verdict::Refused(Refusal::ActOnPage),
        (Line::AgentPage, _) => Verdict::Go,
        (_, Act::Version(_)) => Verdict::Refused(Refusal::PageOnly),
        (_, a) if a.tier() == Tier::Show => Verdict::Go,
        (Line::Launch, _) => Verdict::Go,
        (Line::Outside(_), _) if leave => Verdict::Go,
        (Line::Outside(_), _) => Verdict::Leave,
    }
}

// ---------- what is never done ----------

/// What the library says of an element before a gesture on it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Kind {
    pub tag: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub password: bool,
    pub file: bool,
    pub download: bool,
    pub cross_form: bool,
    pub editable: bool,
    pub select: bool,
    pub tab: bool,
}

/// Why a gesture on an element is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forbidden {
    File,
    Download,
    CrossForm,
    /// A password outside the launch: never, even with a leave.
    PasswordOutside,
    /// A password on the launch without the user's leave (C2): asked.
    PasswordAsk,
    Secret,
    /// `type` in what is not a text field.
    NotText,
    /// `select` on what is not a list.
    NotSelect,
    /// `tab` on what is not a declared tab.
    NotTab,
}

/// What is refused of `act` on an element of `kind` on `line`: `password` the user's leave to fill a password in this
/// browser (C2). Checked after the tiers.
pub fn forbid(line: &Line, act: &Act, kind: &Kind, password: bool) -> Option<Forbidden> {
    let acts = matches!(
        act,
        Act::Click(_) | Act::Tab(_) | Act::Type { .. } | Act::Select(..) | Act::Press(_)
    );
    if !acts {
        return None;
    }
    if kind.file {
        return Some(Forbidden::File);
    }
    if kind.download && matches!(act, Act::Click(_) | Act::Tab(_) | Act::Press(_)) {
        return Some(Forbidden::Download);
    }
    if kind.cross_form {
        return Some(Forbidden::CrossForm);
    }
    match act {
        Act::Type { .. } => {
            if !kind.editable {
                return Some(Forbidden::NotText);
            }
            if kind.password {
                return match line {
                    Line::Launch if password => None,
                    Line::Launch => Some(Forbidden::PasswordAsk),
                    _ => Some(Forbidden::PasswordOutside),
                };
            }
            None
        }
        Act::Select(..) if !kind.select => Some(Forbidden::NotSelect),
        Act::Tab(_) if !kind.tab => Some(Forbidden::NotTab),
        _ => None,
    }
}

/// Whether `text` holds a secret of Escouade: one of `secrets` (its values of 6 characters or more), or what has the
/// form of a key or a token (`escouade_base::secrets`).
pub fn holds_secret(text: &str, secrets: &[String]) -> bool {
    if secrets
        .iter()
        .map(|s| s.trim())
        .any(|s| s.chars().count() >= 6 && text.contains(s))
    {
        return true;
    }
    escouade_base::secrets::mask_secrets(text) != text
}

// ---------- the targets ----------

/// Why a target is not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadTarget {
    Empty,
    TooLong,
    Selector,
    /// A key of a template, but not one of this page's: its keys.
    Key(String),
}

/// Whether `raw` reads as a key of a template (`criteria.2`, `verdict.a`, `subject`).
pub fn is_key(raw: &str) -> bool {
    let mut parts = raw.splitn(2, '.');
    let head = parts.next().unwrap_or("");
    let ok_head = !head.is_empty() && head.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    let ok_tail = parts.next().is_none_or(|t| {
        !t.is_empty()
            && t.len() <= 64
            && t.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    });
    ok_head && ok_tail
}

/// The selector of each key of a page's template, or the keys there are.
pub type Keys<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// The target `raw` names on a row: `keys` the selector of each key of the page's template, on an agent's page (none
/// elsewhere). A key is tried before a selector, so `criteria` is the template's part, not an element `<criteria>`.
pub fn target(raw: &str, keys: Option<Keys<'_>>) -> Result<Target, BadTarget> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(BadTarget::Empty);
    }
    if raw.chars().count() > TARGET_MAX {
        return Err(BadTarget::TooLong);
    }
    if let Some(text) = raw.strip_prefix("text=") {
        let text = text.trim().trim_matches(|c| c == '"' || c == '\'');
        if text.is_empty() {
            return Err(BadTarget::Empty);
        }
        return Ok(Target::Text(text.to_string()));
    }
    if let Some(keys) = keys {
        if is_key(raw) {
            return match keys(raw) {
                Ok(sel) => Ok(Target::Css(sel)),
                Err(e) => {
                    // A bare word that is no key of the page may still be an element's name.
                    if raw.contains('.') || crate::browserread::check_selector(raw).is_err() {
                        Err(BadTarget::Key(e))
                    } else {
                        Ok(Target::Css(raw.to_string()))
                    }
                }
            };
        }
    }
    crate::browserread::check_selector(raw).map_err(|_| BadTarget::Selector)?;
    Ok(Target::Css(raw.to_string()))
}

// ---------- the bounds ----------

/// The gestures of a row: 4 in a second, 600 in an hour.
#[derive(Debug, Default)]
pub struct Pace {
    times: VecDeque<Instant>,
}

impl Pace {
    /// One more gesture at `now`, or the seconds to wait.
    pub fn take(&mut self, now: Instant) -> Result<(), u64> {
        let hour = Duration::from_secs(3600);
        while self
            .times
            .front()
            .is_some_and(|t| now.saturating_duration_since(*t) >= hour)
        {
            self.times.pop_front();
        }
        if self.times.len() >= PER_HOUR {
            let oldest = *self.times.front().unwrap_or(&now);
            return Err(hour
                .saturating_sub(now.saturating_duration_since(oldest))
                .as_secs()
                .max(1));
        }
        let second = Duration::from_secs(1);
        let last_second = self
            .times
            .iter()
            .rev()
            .take_while(|t| now.saturating_duration_since(**t) < second)
            .count();
        if last_second >= PER_SECOND {
            return Err(1);
        }
        self.times.push_back(now);
        Ok(())
    }
}

// ---------- the rule of the hand ----------

/// Who has the hand on a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Hand {
    #[default]
    Agent,
    User,
}

/// How a `wait_user` ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitEnd {
    Continued,
    TookHand,
    Timeout,
}

/// The driving of one row.
#[derive(Default)]
struct Drive {
    agent: Option<String>,
    hand: Hand,
    /// « Ne plus laisser piloter »: until the row closes.
    stopped: bool,
    step: u32,
    what: String,
    pace: Pace,
    /// C2: the agents the user let fill a password in this browser, asked, refused.
    password_ok: Vec<String>,
    password_asked: Vec<String>,
    password_refused: Vec<String>,
    /// A `wait_user` waiting for « Continuer ».
    waiter: Option<tokio::sync::oneshot::Sender<WaitEnd>>,
    /// The hand just came back to the agent: what the user did while they had it is theirs, not counted.
    handed_back: bool,
    /// The core polls the user's gestures of this row.
    polling: bool,
    /// When the agent's last gesture began or ended: what the page does then is the agent's, not the user's.
    acted: Option<Instant>,
}

/// What the window shows of a row's driving (AB-8 draws it).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveView {
    pub browser: String,
    pub agent: Option<String>,
    pub hand: Hand,
    pub stopped: bool,
    pub step: u32,
    pub what: String,
    /// « Continuer » waits for the user.
    pub waiting: bool,
    /// An agent asks to fill a password (C2).
    pub password_asks: Vec<String>,
}

/// What the user did with the hand of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandMove {
    /// « Reprendre la main ».
    Take,
    /// « La rendre ».
    Give,
    /// « Ne plus laisser piloter ».
    Stop,
    /// « Continuer ».
    Continue,
}

/// Whether a gesture may go on a row, by the hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum May {
    Go,
    UserHasHand,
    Stopped,
    /// The pace: the seconds to wait.
    Slow(u64),
}

/// The driving of every row, by key: falls with the row.
#[derive(Default)]
pub struct Drives {
    rows: Mutex<HashMap<String, Drive>>,
}

impl Drives {
    /// Whether `agent` may make one more gesture on `key` at `now`, taken when it may.
    pub fn take(&self, key: &str, agent: &str, now: Instant) -> May {
        let mut rows = self.rows.lock();
        let d = rows.entry(key.to_string()).or_default();
        if d.stopped {
            return May::Stopped;
        }
        if d.hand == Hand::User {
            return May::UserHasHand;
        }
        if let Err(wait) = d.pace.take(now) {
            return May::Slow(wait);
        }
        d.agent = Some(agent.to_string());
        d.acted = Some(now);
        May::Go
    }

    /// Whether the hand came back to the agent since its last gesture: the user's gestures counted meanwhile are
    /// theirs, to forget (once).
    pub fn take_handed_back(&self, key: &str) -> bool {
        self.rows
            .lock()
            .get_mut(key)
            .is_some_and(|d| std::mem::take(&mut d.handed_back))
    }

    /// Whether a gesture of the agent's on `key` is under way or ended less than `within` before `now`: a choice the page
    /// proposes then (`_/choose` of an agent's page, AP-4) is the agent's doing, never the user's, and is refused.
    pub fn acted_lately(&self, key: &str, within: Duration, now: Instant) -> bool {
        self.rows
            .lock()
            .get(key)
            .and_then(|d| d.acted)
            .is_some_and(|t| now.saturating_duration_since(t) < within)
    }

    /// Why no gesture goes on `key` now, by the hand: the user's, or the driving ended.
    pub fn blocked(&self, key: &str) -> Option<May> {
        let rows = self.rows.lock();
        let d = rows.get(key)?;
        if d.stopped {
            Some(May::Stopped)
        } else if d.hand == Hand::User {
            Some(May::UserHasHand)
        } else {
            None
        }
    }

    /// A gesture done: its number, and what the band says of it.
    pub fn done(&self, key: &str, what: &str) -> u32 {
        let mut rows = self.rows.lock();
        let d = rows.entry(key.to_string()).or_default();
        d.step += 1;
        d.what = what.to_string();
        d.acted = Some(Instant::now());
        d.step
    }

    /// The user's gesture in the row (or « Reprendre la main »): the hand is theirs, a `wait_user` ends. False when it
    /// was already.
    pub fn user_took(&self, key: &str) -> bool {
        let mut rows = self.rows.lock();
        let d = rows.entry(key.to_string()).or_default();
        if d.hand == Hand::User {
            return false;
        }
        d.hand = Hand::User;
        d.polling = false;
        if let Some(w) = d.waiter.take() {
            let _ = w.send(WaitEnd::TookHand);
        }
        true
    }

    /// A move of the user's: whether something changed, and the agent to tell.
    pub fn moved(&self, key: &str, m: HandMove) -> (bool, Option<String>) {
        match m {
            HandMove::Take => {
                let agent = self.rows.lock().get(key).and_then(|d| d.agent.clone());
                (self.user_took(key), agent)
            }
            HandMove::Give => {
                let mut rows = self.rows.lock();
                let Some(d) = rows.get_mut(key) else {
                    return (false, None);
                };
                if d.hand == Hand::Agent || d.stopped {
                    return (false, None);
                }
                d.hand = Hand::Agent;
                d.handed_back = true;
                (true, d.agent.clone())
            }
            HandMove::Stop => {
                let mut rows = self.rows.lock();
                let d = rows.entry(key.to_string()).or_default();
                let changed = !d.stopped;
                d.stopped = true;
                d.hand = Hand::User;
                d.polling = false;
                if let Some(w) = d.waiter.take() {
                    let _ = w.send(WaitEnd::TookHand);
                }
                (changed, d.agent.clone())
            }
            HandMove::Continue => {
                let mut rows = self.rows.lock();
                let w = rows.get_mut(key).and_then(|d| d.waiter.take());
                match w {
                    Some(w) => (w.send(WaitEnd::Continued).is_ok(), None),
                    None => (false, None),
                }
            }
        }
    }

    /// A `wait_user` on `key`: what ends it.
    pub fn wait(&self, key: &str) -> tokio::sync::oneshot::Receiver<WaitEnd> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut rows = self.rows.lock();
        let d = rows.entry(key.to_string()).or_default();
        if let Some(old) = d.waiter.replace(tx) {
            let _ = old.send(WaitEnd::Timeout);
        }
        rx
    }

    /// The `wait_user` of `key` timed out.
    pub fn unwait(&self, key: &str) {
        if let Some(d) = self.rows.lock().get_mut(key) {
            d.waiter = None;
        }
    }

    /// The core starts polling the user's gestures of `key`: false when it does already, or should not.
    pub fn start_polling(&self, key: &str) -> bool {
        let mut rows = self.rows.lock();
        let d = rows.entry(key.to_string()).or_default();
        if d.polling || d.hand == Hand::User || d.stopped {
            return false;
        }
        d.polling = true;
        true
    }

    /// Whether the core keeps polling `key`.
    pub fn polls(&self, key: &str) -> bool {
        self.rows.lock().get(key).is_some_and(|d| d.polling)
    }

    /// The polling of `key` stopped (its webview went).
    pub fn stop_polling(&self, key: &str) {
        if let Some(d) = self.rows.lock().get_mut(key) {
            d.polling = false;
        }
    }

    /// The hand on `key`, while someone drives it.
    pub fn hand(&self, key: &str) -> Option<Hand> {
        self.rows
            .lock()
            .get(key)
            .filter(|d| d.agent.is_some() || d.stopped)
            .map(|d| d.hand)
    }

    /// Whether the user let `agent` fill a password in `key` (C2); asked when not, unless refused or asked already.
    /// `Ok(true)`: given; `Ok(false)`: asked now or still waiting; `Err(())`: refused.
    pub fn password(&self, key: &str, agent: &str) -> Result<bool, ()> {
        let mut rows = self.rows.lock();
        let d = rows.entry(key.to_string()).or_default();
        let a = agent.to_string();
        if d.password_ok.contains(&a) {
            return Ok(true);
        }
        if d.password_refused.contains(&a) {
            return Err(());
        }
        if !d.password_asked.contains(&a) {
            d.password_asked.push(a);
        }
        Ok(false)
    }

    /// The user's answer to `agent`'s request to fill a password in `key`: false when none waits.
    pub fn answer_password(&self, key: &str, agent: &str, allow: bool) -> bool {
        let mut rows = self.rows.lock();
        let Some(d) = rows.get_mut(key) else {
            return false;
        };
        let Some(at) = d.password_asked.iter().position(|x| x == agent) else {
            return false;
        };
        let a = d.password_asked.remove(at);
        if allow {
            d.password_ok.push(a);
        } else {
            d.password_refused.push(a);
        }
        true
    }

    /// What the window shows of `key`.
    pub fn view(&self, key: &str) -> DriveView {
        let rows = self.rows.lock();
        let Some(d) = rows.get(key) else {
            return DriveView {
                browser: key.to_string(),
                ..Default::default()
            };
        };
        DriveView {
            browser: key.to_string(),
            agent: d.agent.clone(),
            hand: d.hand,
            stopped: d.stopped,
            step: d.step,
            what: d.what.clone(),
            waiting: d.waiter.is_some(),
            password_asks: d.password_asked.clone(),
        }
    }

    /// The row `key` closed: its driving goes, a `wait_user` ends.
    pub fn forget(&self, key: &str) {
        if let Some(mut d) = self.rows.lock().remove(key) {
            if let Some(w) = d.waiter.take() {
                let _ = w.send(WaitEnd::TookHand);
            }
        }
    }
}

// ---------- in the page ----------

/// The expression of the library called with `args`.
pub fn expression(args: &Value) -> String {
    format!("({LIB})({args})")
}

/// Runs the library with `args` in Escouade's world of `webview`: its answer, an error it gave said as one.
pub fn run<R: Runtime>(webview: &Webview<R>, args: Value) -> Result<Value, String> {
    let text = crate::browserread::eval_expression(webview, &expression(&args))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| format!("not an answer of Escouade's script: {e}"))?;
    match v["error"].as_str() {
        Some("selector") => Err("selector".into()),
        Some("target") => Err("target".into()),
        Some(e) => Err(format!(
            "{e}{}",
            v["message"]
                .as_str()
                .map(|m| format!(": {m}"))
                .unwrap_or_default()
        )),
        None => Ok(v),
    }
}

/// An agent's gesture is under way for `ms`: what the page makes of it is not the user's.
#[cfg(windows)]
pub fn arm<R: Runtime>(webview: &Webview<R>, ms: u64) {
    let _ = run(webview, json!({ "run": "arm", "ms": ms }));
}

/// The user's gestures in `webview` since the last time.
pub fn gestures<R: Runtime>(webview: &Webview<R>) -> u64 {
    run(webview, json!({ "run": "gestures" }))
        .ok()
        .and_then(|v| v["gestures"].as_u64())
        .unwrap_or(0)
}

/// Waits for the page to settle after a gesture: loaded, its scroll still, `SETTLE` at most. Where it is then.
pub fn settle<R: Runtime>(webview: &Webview<R>) -> Value {
    let start = Instant::now();
    let mut last: Option<i64> = None;
    let mut seen = json!({});
    std::thread::sleep(Duration::from_millis(120));
    while start.elapsed() < SETTLE {
        if let Ok(v) = run(webview, json!({ "run": "where" })) {
            let y = v["scrollY"].as_i64();
            let ready = v["ready"].as_str() == Some("complete");
            let still = last.is_some() && last == y;
            seen = v;
            if ready && still {
                break;
            }
            last = y;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    seen
}

/// A real click at the centre of `bx` (CSS pixels of the view): the DevTools protocol's inputs under Windows (heard
/// as the user's, `isTrusted`), the isolated world's synthetic click elsewhere.
pub fn click_at<R: Runtime>(
    webview: &Webview<R>,
    target: &Target,
    bx: &Value,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let _ = target;
        let x = bx["x"].as_f64().unwrap_or(0.0) + bx["width"].as_f64().unwrap_or(0.0) / 2.0;
        let y = bx["y"].as_f64().unwrap_or(0.0) + bx["height"].as_f64().unwrap_or(0.0) / 2.0;
        arm(webview, 1500);
        for t in ["mouseMoved", "mousePressed", "mouseReleased"] {
            crate::browserread::input(
                webview,
                crate::browserread::Method::InputDispatchMouseEvent,
                json!({ "type": t, "x": x, "y": y, "button": "left", "buttons": if t == "mousePressed" { 1 } else { 0 }, "clickCount": 1 }),
            )?;
        }
        arm(webview, TAIL_MS);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = bx;
        run(webview, json!({ "run": "click", "target": target.arg() })).map(|_| ())
    }
}

/// Types `text` in the field `target`, what it held replaced: `Input.insertText` under Windows (the field focused and
/// its text selected first), the native setter and its events elsewhere. `submit`: Enter after.
pub fn type_in<R: Runtime>(
    webview: &Webview<R>,
    target: &Target,
    text: &str,
    submit: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let f = run(webview, json!({ "run": "focus", "target": target.arg() }))?;
        if f["found"] != json!(true) {
            return Err("gone".into());
        }
        arm(webview, 1500);
        if text.is_empty() {
            run(
                webview,
                json!({ "run": "fill", "target": target.arg(), "text": "" }),
            )?;
        } else {
            crate::browserread::input(
                webview,
                crate::browserread::Method::InputInsertText,
                json!({ "text": text }),
            )?;
        }
        if submit {
            key(webview, Key::Enter)?;
        }
        arm(webview, TAIL_MS);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        run(
            webview,
            json!({ "run": "fill", "target": target.arg(), "text": text }),
        )?;
        if submit {
            key(webview, Key::Enter)?;
        }
        Ok(())
    }
}

/// A key of the closed list in the focused element: real under Windows, synthetic elsewhere.
pub fn key<R: Runtime>(webview: &Webview<R>, k: Key) -> Result<(), String> {
    let (name, code, vk) = k.codes();
    #[cfg(windows)]
    {
        arm(webview, 1500);
        let text = if k == Key::Enter { "\r" } else { "" };
        let down = if text.is_empty() {
            "rawKeyDown"
        } else {
            "keyDown"
        };
        let mut d = json!({ "type": down, "key": name, "code": code, "windowsVirtualKeyCode": vk, "nativeVirtualKeyCode": vk });
        if !text.is_empty() {
            d["text"] = json!(text);
            d["unmodifiedText"] = json!(text);
        }
        crate::browserread::input(
            webview,
            crate::browserread::Method::InputDispatchKeyEvent,
            d,
        )?;
        crate::browserread::input(
            webview,
            crate::browserread::Method::InputDispatchKeyEvent,
            json!({ "type": "keyUp", "key": name, "code": code, "windowsVirtualKeyCode": vk, "nativeVirtualKeyCode": vk }),
        )?;
        arm(webview, TAIL_MS);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = vk;
        run(webview, json!({ "run": "key", "key": name, "code": code })).map(|_| ())
    }
}

/// An agent's page at another width: emulated under Windows (`full` clears it); not under WebKit yet.
pub fn emulate_width<R: Runtime>(webview: &Webview<R>, mode: PageMode) -> Result<(), String> {
    #[cfg(windows)]
    {
        let width = match mode {
            PageMode::Full => {
                return crate::browserread::input(
                    webview,
                    crate::browserread::Method::EmulationClearDeviceMetricsOverride,
                    json!({}),
                )
                .map(|_| ())
            }
            PageMode::Tablet => 768,
            PageMode::Mobile => 390,
        };
        let h = crate::browserread::eval(
            webview,
            &crate::browserread::Script::Rect { selector: None },
        )
        .ok()
        .and_then(|v| v["view"]["innerHeight"].as_f64())
        .unwrap_or(700.0);
        crate::browserread::input(
            webview,
            crate::browserread::Method::EmulationSetDeviceMetricsOverride,
            json!({ "width": width, "height": h.round() as i64, "deviceScaleFactor": 0, "mobile": mode == PageMode::Mobile }),
        )
        .map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let _ = (webview, mode);
        Err("webkit-width".into())
    }
}
