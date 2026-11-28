//! The closed list of events (`docs/TEAM-AUDIT.md` §1): each with its stable name (`family.action`), its severity
//! (1, information, to 5, alert) and its `actor`, `target` and `detail`, closed structures (`deny_unknown_fields`).
//! The events of the kill switch (TS-15) and of the private store (TS-14) are here too: AU-4 and AU-5 emit them.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::words::{
    CommandPrint, Digest, Id, Ip, Name, Namespace, PluginId, PublicKey, Reason, SettingKey,
    SixWords, TicketKey, Time, Token, Version,
};

/// The most items a list of an event holds: what goes past it is split by the emitter (an event stays under 8 KiB in
/// syslog, `syslog.rs`).
pub const MAX_LIST: usize = 32;

/// A list of an event, of [`MAX_LIST`] items at most.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct List<T>(Vec<T>);

impl<T> List<T> {
    pub fn new(items: Vec<T>) -> Result<Self, crate::Refused> {
        if items.len() > MAX_LIST {
            return Err(crate::Refused::Shape("List"));
        }
        Ok(List(items))
    }
    pub fn items(&self) -> &[T] {
        &self.0
    }
}

impl<T: Serialize> Serialize for List<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for List<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        List::new(Vec::<T>::deserialize(d)?).map_err(de::Error::custom)
    }
}

/// Nothing: an empty object (`{}`), for an event without a target or a detail.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nothing {}

/// Who did it, as far as the source knows: S1 knows a device and an address, S2 the member and the names (A3-B).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<DeviceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<Ip>,
}

macro_rules! closed_enum {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $v:ident),* $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($(#[$vm])* $v),* }
    };
}

closed_enum!(
    /// A role in the team (the roster, `escouade-team-crypto/src/roster.rs`; `service` for the audit machine, A2-A).
    Role { Admin, Member, Service }
);
closed_enum!(
    /// What a device is.
    DeviceKind { Desktop, Phone, Machine, Recovery }
);
closed_enum!(
    /// Who admitted a device: an administrator, or a device of the same member.
    Admission { Admin, OwnDevice }
);
closed_enum!(
    /// Why an epoch opened.
    EpochCause { Departure, Revocation, Rotation }
);
closed_enum!(
    /// The token a device enrolled with (`/v1/enroll`).
    EnrollToken { Init, Invitation, Recovery }
);
closed_enum!(
    /// Why a session was refused.
    SessionRefusal { BadSignature, Revoked, Unknown }
);
closed_enum!(
    /// What went past its rate.
    RateScope { Address, Device }
);
closed_enum!(
    /// Who proposed a command.
    CommandOrigin { Team, Repository }
);
closed_enum!(
    /// Where the strategy of a managed station is read (ADMX, an MDM profile, `/etc`).
    PolicyStore { Admx, Mdm, Etc }
);
closed_enum!(
    /// By where the core was asked to write a locked setting.
    LockVia { Window, Mcp, Repository, Export }
);
closed_enum!(
    /// Where a plugin comes from.
    PluginOrigin { PublicStore, PrivateStore, Git }
);
closed_enum!(
    /// Whose policy refused a plugin.
    PolicyOf { Team, Device }
);
closed_enum!(
    /// Who decided: the member, the Overseer (under a directive), a rule.
    Decider { Member, Overseer, Rule }
);
closed_enum!(
    /// What a validation did.
    Outcome { Merge, PullRequest, None }
);
closed_enum!(
    /// The answer to a permission.
    Answer { AllowOnce, AllowAlways, Deny }
);
closed_enum!(
    /// What a directive of the Overseer covers.
    DirectiveScope { Team, Project, Ticket }
);

/// A member (`role.changed`, the signers).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Who {
    pub member: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

/// Whose role changed: a member of the roster (S2), or a device and its role on the server (S1, which knows no
/// member).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

/// A member and their role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberTarget {
    pub member: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    pub role: Role,
}

/// A device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceTarget {
    pub device: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<DeviceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

/// A project: its id, and its short name from S2 (never its logo nor its description).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTarget {
    pub project: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

/// A ticket: its opaque id on the server (S1), its project and its key from S2, never its title (A3-B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TicketTarget {
    pub item: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<TicketKey>,
}

/// An invitation, never its link nor its secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvitationTarget {
    pub invitation: Id,
}

/// Keys of `.escouade.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeysTarget {
    pub keys: List<SettingKey>,
}

/// A command proposed by the team or the repository, by its key, its program and its fingerprint (A8-A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandTarget {
    pub key: SettingKey,
    pub command: CommandPrint,
}

/// A plugin, at a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginTarget {
    pub plugin: PluginId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
}

/// The namespace of the private store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamespaceTarget {
    pub namespace: Namespace,
}

/// A directive of the Overseer, by its id (never its text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectiveTarget {
    pub directive: Id,
}

/// A memory, or all those a ticket's agent wrote (« tout ce qu'a écrit DEM-41 »), never their text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<TicketKey>,
}

/// A budget of TS-16.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetTarget {
    pub budget: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

/// A stream of the server (`team`, a project's id, `audit`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamTarget {
    pub stream: Token,
}

/// What a kill switch stops (TS-15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    Team,
    Project(ProjectTarget),
    Harness(Token),
    Model(Token),
    Plugin(PluginId),
    Agents,
}

/// The target of a kill switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeTarget {
    pub scope: Scope,
}

/// The value of a plain setting (to restrict, a value): a flag, a number, a word or words, never a free text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Plain {
    Flag(bool),
    Number(i64),
    Word(Token),
    Words(List<Token>),
}

/// A key changed in the team's `.escouade.yml`, by its sort: a sensitive key goes out by the fingerprints of its
/// values, an executable one by its program and fingerprint (A8-A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Restrict(PlainChange),
    Value(PlainChange),
    Sensitive(DigestChange),
    Executable(CommandChange),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlainChange {
    pub key: SettingKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Plain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Plain>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigestChange {
    pub key: SettingKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Digest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandChange {
    pub key: SettingKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<CommandPrint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<CommandPrint>,
}

/// Where a ticket's agent is handed (TS-17): a colleague or a machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipient {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

macro_rules! detail {
    ($(#[$m:meta])* $name:ident { $($(#[$fm:meta])* $f:ident : $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $($(#[$fm])* pub $f: $t),* }
    };
}

detail!(TeamCreated {
    fingerprint: SixWords
});
detail!(
    /// The epoch the change opened.
    MemberChange { epoch: u64 }
);
detail!(DeviceAdded {
    admitted_by: Admission
});
detail!(DeviceRevoked {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<Reason>,
});
detail!(RoleChanged {
    from: Role,
    to: Role
});
detail!(EpochOpened {
    epoch: u64,
    cause: EpochCause
});
detail!(
    /// What an invitation offers, never its link.
    InvitationDetail {
        role: Role,
        projects: List<Id>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expires: Option<Time>,
    }
);
detail!(
    /// The fingerprint of the escrow's public key (C7-B).
    EscrowDetail { key: Digest }
);
detail!(
    /// The heads a device saw for one epoch (a fork), or the head it went back to.
    ForkDetail {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        epoch: Option<u64>,
        heads: List<Digest>,
    }
);
detail!(DeviceEnrolled { token: EnrollToken });
detail!(SessionRefused {
    cause: SessionRefusal
});
detail!(RateLimited {
    scope: RateScope,
    count: u64
});
detail!(ClientTooOld { version: Version });
detail!(
    /// A body refused for its size (413): the most bytes the route takes.
    TooLarge { max: u64 }
);
detail!(
    /// Open to the team, or a list of devices.
    AccessChanged { open: bool, devices: List<Id> }
);
detail!(
    /// The new version of the team's `.escouade.yml`.
    SettingsChanged {
        version: u64,
        changes: List<Change>,
        signers: List<Who>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<Reason>,
    }
);
detail!(SignersChanged { signers: List<Who>, quorum: u32 });
detail!(SensitiveAnswered { accepted: bool });
detail!(CommandDetail {
    origin: CommandOrigin
});
detail!(
    /// A strategy of the managed station: its keys are the target, never the values of a proxy.
    PolicyApplied { store: PolicyStore }
);
detail!(LockRefused { via: LockVia });
detail!(KillSwitch {
    serial: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<Reason>,
});
detail!(
    /// How many agents a station stopped.
    KillApplied { serial: u64, stopped: u32 }
);
detail!(NamespaceDeclared { key: PublicKey });
detail!(
    /// The archive's fingerprint, the permissions asked, the key that signed.
    PluginPublished {
        archive: Digest,
        permissions: List<Token>,
        key: PublicKey,
    }
);
detail!(StoreRevoked {
    serial: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<Reason>,
});
detail!(
    /// On a station: the origin, the version it came from, the permissions granted.
    PluginDetail {
        origin: PluginOrigin,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<Version>,
        permissions: List<Token>,
    }
);
detail!(PluginRefused {
    origin: PluginOrigin,
    by: PolicyOf
});
detail!(
    /// The fencing token of a claim (never its renewals).
    Claim { token: u64 }
);
detail!(
    /// A write refused under a stale token (`stale_token`).
    ClaimFenced { token: u64, current: u64 }
);
detail!(
    /// The branch goes out by its fingerprint: its name may say what the ticket is.
    Handoff {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Recipient>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<Digest>,
    }
);
detail!(
    /// A validation: by whom, what it did, into which branch (its fingerprint).
    Validation {
        by: Decider,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        directive: Option<Id>,
        outcome: Outcome,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        into: Option<Digest>,
    }
);
detail!(
    /// A permission answered: by whom, the tool, the command's program and fingerprint (A8-A).
    Permission {
        by: Decider,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        directive: Option<Id>,
        tool: Token,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<CommandPrint>,
        answer: Answer,
    }
);
detail!(ModeChanged {
    from: Token,
    to: Token
});
detail!(
    /// What a directive allows, its scope, the message that gave it (its id, never its text).
    Directive {
        allows: List<Token>,
        scope: DirectiveScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        project: Option<Id>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<Id>,
    }
);
detail!(
    /// The provenance (the device or agent that wrote), how many memories.
    MemoryDetail {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provenance: Option<Id>,
        count: u32,
    }
);
detail!(BudgetCrossed {
    percent: u8,
    autopilot_stopped: bool
});
detail!(
    /// Events the buffer dropped: how many, of when to when (their `seen`), and their numbers in the emitter's chain,
    /// which `verify` takes as a gap said.
    Dropped { count: u64, from: Time, to: Time, first: u64, last: u64 }
);
detail!(
    /// The head of the emitter's chain (§5.2, link 4): its `seq` and its last event's fingerprint. AU-5 writes it in
    /// the output and in the team's stream every hour and at each closing; `verify` checks it.
    Checkpoint { seq: u64, head: Digest }
);

/// Why an event's three parts were not read.
#[derive(Debug, thiserror::Error)]
pub enum FactError {
    #[error("unknown event « {0} »")]
    Unknown(String),
    #[error("{part} of {event}: {error}")]
    Part {
        event: &'static str,
        part: &'static str,
        error: serde_json::Error,
    },
}

macro_rules! events {
    ($( $(#[$m:meta])* $variant:ident = $name:literal, $sev:literal, $target:ty, $detail:ty; )*) => {
        /// An event of the closed list, its actor, target and detail.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Fact {
            $( $(#[$m])* $variant { actor: Actor, target: $target, detail: $detail }, )*
        }

        impl Fact {
            /// The names of every event, in the order of `docs/TEAM-AUDIT.md` §1.
            pub const NAMES: &'static [&'static str] = &[$($name),*];

            /// The event's stable name (`settings.changed`).
            pub fn name(&self) -> &'static str {
                match self { $(Fact::$variant { .. } => $name,)* }
            }

            /// The severity of the event of `name` (`permission.answered` says 2; 4 when « Toujours autoriser »).
            pub fn severity_of(name: &str) -> Option<u8> {
                match name { $($name => Some($sev),)* _ => None }
            }

            pub fn actor(&self) -> &Actor {
                match self { $(Fact::$variant { actor, .. } => actor,)* }
            }

            /// Its actor, target and detail, as JSON.
            pub fn parts(&self) -> (Value, Value, Value) {
                fn v<T: Serialize>(t: &T) -> Value {
                    serde_json::to_value(t).expect("an event's part is JSON")
                }
                match self {
                    $(Fact::$variant { actor, target, detail } => (v(actor), v(target), v(detail)),)*
                }
            }

            /// The event `name` of these parts, each closed: a field it does not know is refused.
            pub fn from_parts(name: &str, actor: Value, target: Value, detail: Value) -> Result<Fact, FactError> {
                match name {
                    $($name => Ok(Fact::$variant {
                        actor: serde_json::from_value(actor)
                            .map_err(|error| FactError::Part { event: $name, part: "actor", error })?,
                        target: serde_json::from_value(target)
                            .map_err(|error| FactError::Part { event: $name, part: "target", error })?,
                        detail: serde_json::from_value(detail)
                            .map_err(|error| FactError::Part { event: $name, part: "detail", error })?,
                    }),)*
                    other => Err(FactError::Unknown(other.to_owned())),
                }
            }

            /// The event `name` read from `d` (the trapped data's test fills every slot by it).
            #[cfg(test)]
            pub(crate) fn fill<'de, D>(name: &str, d: D) -> Result<Fact, D::Error>
            where
                D: Deserializer<'de> + Clone,
            {
                match name {
                    $($name => Ok(Fact::$variant {
                        actor: Deserialize::deserialize(d.clone())?,
                        target: Deserialize::deserialize(d.clone())?,
                        detail: Deserialize::deserialize(d)?,
                    }),)*
                    other => Err(de::Error::custom(format!("unknown event {other}"))),
                }
            }
        }
    };
}

events! {
    // 1.1 The members, the devices and the roles (the signed journal).
    TeamCreated = "team.created", 3, Nothing, TeamCreated;
    MemberAdded = "member.added", 3, MemberTarget, MemberChange;
    MemberRemoved = "member.removed", 3, MemberTarget, MemberChange;
    DeviceAdded = "device.added", 2, DeviceTarget, DeviceAdded;
    /// S1's own word of an enrolment (`/v1/enroll`, §3.1), its token's sort: S2 says `team.created` or `device.added`.
    DeviceEnrolled = "device.enrolled", 2, DeviceTarget, DeviceEnrolled;
    DeviceRevoked = "device.revoked", 3, DeviceTarget, DeviceRevoked;
    RoleChanged = "role.changed", 4, RoleTarget, RoleChanged;
    EpochOpened = "epoch.opened", 2, Nothing, EpochOpened;
    InvitationCreated = "invitation.created", 2, InvitationTarget, InvitationDetail;
    InvitationUsed = "invitation.used", 2, InvitationTarget, InvitationDetail;
    InvitationReused = "invitation.reused", 5, InvitationTarget, Nothing;
    RecoveryUsed = "recovery.used", 5, Nothing, Nothing;
    EscrowSet = "escrow.set", 4, Nothing, EscrowDetail;
    EscrowRemoved = "escrow.removed", 4, Nothing, EscrowDetail;
    TeamFork = "team.fork", 5, Nothing, ForkDetail;
    TeamRollback = "team.rollback", 5, Nothing, ForkDetail;
    // 1.2 The accesses to the server.
    SessionOpened = "session.opened", 1, Nothing, Nothing;
    SessionRefused = "session.refused", 3, Nothing, SessionRefused;
    RateLimited = "rate.limited", 3, Nothing, RateLimited;
    ClientTooOld = "client.too_old", 1, Nothing, ClientTooOld;
    /// A body the server refused for its size (AU-2, §3.1: « les tailles refusées »).
    RequestTooLarge = "request.too_large", 2, Nothing, TooLarge;
    ProjectAccessChanged = "project.access.changed", 3, ProjectTarget, AccessChanged;
    /// S1's word of an entry written to a stream (§3.1): the anchor holds its number, its epoch and the fingerprint of
    /// its encrypted bytes, which S2's event of the same entry repeats (§5.3).
    StreamWritten = "stream.written", 1, StreamTarget, Nothing;
    // 1.3 The imposed settings and the stations' answers.
    SettingsChanged = "settings.changed", 4, KeysTarget, SettingsChanged;
    SettingsSignersChanged = "settings.signers.changed", 5, Nothing, SignersChanged;
    SettingsSensitiveAnswered = "settings.sensitive.answered", 4, KeysTarget, SensitiveAnswered;
    CommandApproved = "command.approved", 3, CommandTarget, CommandDetail;
    CommandRevoked = "command.revoked", 3, CommandTarget, CommandDetail;
    PolicyApplied = "policy.applied", 2, KeysTarget, PolicyApplied;
    LockRefused = "lock.refused", 4, KeysTarget, LockRefused;
    // 1.4 The kill switch (TS-15).
    KillswitchRaised = "killswitch.raised", 5, ScopeTarget, KillSwitch;
    KillswitchLifted = "killswitch.lifted", 5, ScopeTarget, KillSwitch;
    KillswitchApplied = "killswitch.applied", 3, ScopeTarget, KillApplied;
    // 1.5 The private store (TS-14).
    StoreNamespaceDeclared = "store.namespace.declared", 4, NamespaceTarget, NamespaceDeclared;
    StorePluginPublished = "store.plugin.published", 3, PluginTarget, PluginPublished;
    StorePluginRevoked = "store.plugin.revoked", 4, PluginTarget, StoreRevoked;
    PluginInstalled = "plugin.installed", 2, PluginTarget, PluginDetail;
    PluginRemoved = "plugin.removed", 2, PluginTarget, PluginDetail;
    PluginUpdated = "plugin.updated", 2, PluginTarget, PluginDetail;
    PluginRefused = "plugin.refused", 4, PluginTarget, PluginRefused;
    // 1.6 The tickets, the agents and their permissions.
    ClaimTaken = "claim.taken", 1, TicketTarget, Claim;
    ClaimReleased = "claim.released", 1, TicketTarget, Claim;
    ClaimExpired = "claim.expired", 1, TicketTarget, Claim;
    ClaimFenced = "claim.fenced", 3, TicketTarget, ClaimFenced;
    HandoffOffered = "handoff.offered", 2, TicketTarget, Handoff;
    HandoffTaken = "handoff.taken", 2, TicketTarget, Handoff;
    HandoffWithdrawn = "handoff.withdrawn", 2, TicketTarget, Handoff;
    TicketApproved = "ticket.approved", 2, TicketTarget, Validation;
    TicketRejected = "ticket.rejected", 2, TicketTarget, Validation;
    PermissionAnswered = "permission.answered", 2, TicketTarget, Permission;
    AgentModeChanged = "agent.mode.changed", 3, TicketTarget, ModeChanged;
    DirectiveAdded = "directive.added", 3, DirectiveTarget, Directive;
    DirectiveRevoked = "directive.revoked", 3, DirectiveTarget, Directive;
    MemoryAccepted = "memory.accepted", 3, MemoryTarget, MemoryDetail;
    MemoryQuarantined = "memory.quarantined", 3, MemoryTarget, MemoryDetail;
    BudgetCrossed = "budget.crossed", 3, BudgetTarget, BudgetCrossed;
    // The audit's own (§4.4, §5.2).
    AuditDropped = "audit.dropped", 4, Nothing, Dropped;
    AuditCheckpoint = "audit.checkpoint", 1, Nothing, Checkpoint;
}

impl Fact {
    /// The event's severity, 1 to 5: its name's, but a permission « Toujours autoriser », 4.
    pub fn severity(&self) -> u8 {
        match self {
            Fact::PermissionAnswered { detail, .. } if detail.answer == Answer::AllowAlways => 4,
            _ => Fact::severity_of(self.name()).expect("a name of the list"),
        }
    }
}
