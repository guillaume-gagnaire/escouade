//! The record `escouade.audit/v1` (`docs/TEAM-AUDIT.md` §4.1), its signature and its chain (§5.2, links 1 and 2), and
//! the emitter that seals it.

use std::collections::BTreeMap;

use base64::Engine as _;
use ed25519_dalek::{Signer as _, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::{Actor, Fact, FactError};
use crate::words::{Digest, Id, Name, PublicKey, Signature, Time, Token, Ulid};
use crate::{jcs, never, syslog};

/// The model's name and version.
pub const SCHEMA: &str = "escouade.audit/v1";

/// The team an event is of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Team {
    pub id: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
}

/// Who emits (§3): the blind server (S1), a machine of the team that decrypts (S2), a station's fact it opened (S3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Server,
    Machine,
    Device,
}

/// The emitter's chain at this event: its device, its number, the fingerprint of the one before (none for the first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmitterRef {
    pub device: Id,
    pub seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<Digest>,
}

/// The entry of the server's journal an event comes from (§5.2, link 3): its stream, its number, its epoch, the
/// SHA-256 of its encrypted bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub stream: Token,
    pub n: u64,
    pub epoch: u64,
    pub ct: Digest,
}

/// An event as it goes out.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub id: Ulid,
    pub fact: Fact,
    pub severity: u8,
    pub at: Time,
    pub seen: Time,
    pub team: Team,
    pub source: Source,
    pub emitter: EmitterRef,
    pub anchor: Option<Anchor>,
    pub sig: Option<Signature>,
}

/// The record's JSON, its fields in the order of §4.1, closed.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: String,
    id: Ulid,
    event: String,
    severity: u8,
    at: Time,
    seen: Time,
    team: Team,
    actor: Value,
    target: Value,
    detail: Value,
    source: Source,
    emitter: EmitterRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anchor: Option<Anchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sig: Option<Signature>,
}

/// Why a line is not a record of the model.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("not JSON of the model: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema « {0} », not {SCHEMA}")]
    Schema(String),
    #[error(transparent)]
    Fact(#[from] FactError),
    #[error("severity {got} where {event} has {want}")]
    Severity {
        event: &'static str,
        got: u8,
        want: u8,
    },
    #[error(transparent)]
    Jcs(#[from] jcs::JcsError),
}

impl Record {
    /// Its JSON.
    pub fn to_value(&self) -> Value {
        let (actor, target, detail) = self.fact.parts();
        serde_json::to_value(Wire {
            schema: SCHEMA.to_owned(),
            id: self.id.clone(),
            event: self.fact.name().to_owned(),
            severity: self.severity,
            at: self.at,
            seen: self.seen,
            team: self.team.clone(),
            actor,
            target,
            detail,
            source: self.source,
            emitter: self.emitter.clone(),
            anchor: self.anchor.clone(),
            sig: self.sig.clone(),
        })
        .expect("a record is JSON")
    }

    /// Its line of JSON lines.
    pub fn to_line(&self) -> String {
        serde_json::to_string(&self.to_value()).expect("a record is JSON")
    }

    /// The record of a JSON value, closed at every level, its severity the one of its event.
    pub fn from_value(v: Value) -> Result<Record, ModelError> {
        let w: Wire = serde_json::from_value(v)?;
        if w.schema != SCHEMA {
            return Err(ModelError::Schema(w.schema));
        }
        let fact = Fact::from_parts(&w.event, w.actor, w.target, w.detail)?;
        if w.severity != fact.severity() {
            return Err(ModelError::Severity {
                event: fact.name(),
                got: w.severity,
                want: fact.severity(),
            });
        }
        Ok(Record {
            id: w.id,
            fact,
            severity: w.severity,
            at: w.at,
            seen: w.seen,
            team: w.team,
            source: w.source,
            emitter: w.emitter,
            anchor: w.anchor,
            sig: w.sig,
        })
    }

    pub fn from_line(line: &str) -> Result<Record, ModelError> {
        Record::from_value(serde_json::from_str(line)?)
    }

    pub fn actor(&self) -> &Actor {
        self.fact.actor()
    }
}

/// What is signed of a record's JSON: its canonical form (RFC 8785) without `sig`.
pub fn signed_bytes(v: &Value) -> Result<Vec<u8>, jcs::JcsError> {
    let mut v = v.clone();
    if let Value::Object(m) = &mut v {
        m.remove("sig");
    }
    jcs::canonical(&v)
}

/// The fingerprint a record is chained by (`emitter.prev` of the next): SHA-256 of its whole canonical form, `sig`
/// included.
pub fn chain_digest(v: &Value) -> Result<Digest, jcs::JcsError> {
    Ok(Digest::of(&jcs::canonical(v)?))
}

/// Whether the record's JSON `v` is signed by `key`.
pub fn signature_holds(v: &Value, key: &VerifyingKey) -> bool {
    let Some(sig) = v.get("sig").and_then(Value::as_str) else {
        return false;
    };
    let Some(bytes) = sig
        .strip_prefix("ed25519:")
        .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
    else {
        return false;
    };
    let Ok(sig) = ed25519_dalek::Signature::from_slice(&bytes) else {
        return false;
    };
    match signed_bytes(v) {
        Ok(msg) => key.verify(&msg, &sig).is_ok(),
        Err(_) => false,
    }
}

impl PublicKey {
    /// The public key of `key`.
    pub fn of(key: &VerifyingKey) -> PublicKey {
        PublicKey::new(format!(
            "ed25519:{}",
            base64::engine::general_purpose::STANDARD.encode(key.as_bytes())
        ))
        .expect("an Ed25519 key in base64")
    }

    /// The key it says.
    pub fn key(&self) -> Option<VerifyingKey> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(self.as_str().strip_prefix("ed25519:")?)
            .ok()?;
        VerifyingKey::from_bytes(&bytes.try_into().ok()?).ok()
    }
}

/// Where an emitter's chain stands: its last number, the fingerprint of its last event, and the position it read
/// each stream to (the number of the last entry it emitted from), to take up again there without a gap nor a double.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    pub seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<Digest>,
    #[serde(default)]
    pub positions: BTreeMap<String, u64>,
}

impl Chain {
    /// The chain after the record `v` (its `emitter.seq`, its fingerprint, its anchor's position).
    pub fn after(&self, v: &Value) -> Option<Chain> {
        let seq = v.pointer("/emitter/seq")?.as_u64()?;
        let mut positions = self.positions.clone();
        if let (Some(stream), Some(n)) = (
            v.pointer("/anchor/stream").and_then(Value::as_str),
            v.pointer("/anchor/n").and_then(Value::as_u64),
        ) {
            positions.insert(stream.to_owned(), n);
        }
        Some(Chain {
            seq,
            prev: Some(chain_digest(v).ok()?),
            positions,
        })
    }
}

/// An event to seal: the fact, its hour, its anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub fact: Fact,
    pub at: Time,
    pub anchor: Option<Anchor>,
}

/// A sealed event: the record, its line, and how many free strings of it lost a secret or a path (§2: « l'émetteur le
/// dit »).
#[derive(Debug, Clone, PartialEq)]
pub struct Sealed {
    pub record: Record,
    pub line: String,
    pub redacted: usize,
}

/// Why an event was not sealed.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error("the event is over 8 KiB in syslog ({0} bytes): split it")]
    TooLong(usize),
    #[error(transparent)]
    Jcs(#[from] jcs::JcsError),
}

/// Who signs and chains: a device of the team (its key, its id), for one team, as one source.
pub struct Emitter {
    key: SigningKey,
    device: Id,
    source: Source,
    team: Team,
    chain: Chain,
}

impl Emitter {
    /// The emitter of this key, taking its chain up where `chain` says.
    pub fn new(key: SigningKey, device: Id, source: Source, team: Team, chain: Chain) -> Emitter {
        Emitter {
            key,
            device,
            source,
            team,
            chain,
        }
    }

    /// A key drawn from the system's randomness (the server's audit key, drawn at `init`: AU-2).
    pub fn generate_key() -> SigningKey {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("the system's randomness");
        SigningKey::from_bytes(&seed)
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::of(&self.key.verifying_key())
    }

    pub fn device(&self) -> &Id {
        &self.device
    }

    pub fn chain(&self) -> &Chain {
        &self.chain
    }

    /// Signs and chains `draft`, seen at `seen`: its id drawn, its number the next of the chain.
    pub fn seal(&mut self, draft: Draft, seen: Time) -> Result<Sealed, SealError> {
        let team = self.team.clone();
        self.seal_for(team, draft, seen)
    }

    /// [`Emitter::seal`] for another team than the emitter's: the emitter of a server that holds several teams (AU-2)
    /// keeps one chain, each event saying its team.
    pub fn seal_for(&mut self, team: Team, draft: Draft, seen: Time) -> Result<Sealed, SealError> {
        let mut record = Record {
            id: Ulid::new(seen),
            severity: draft.fact.severity(),
            fact: draft.fact,
            at: draft.at,
            seen,
            team,
            source: self.source,
            emitter: EmitterRef {
                device: self.device.clone(),
                seq: self.chain.seq + 1,
                prev: self.chain.prev.clone(),
            },
            anchor: draft.anchor,
            sig: None,
        };
        let msg = signed_bytes(&record.to_value())?;
        let sig = self.key.sign(&msg);
        record.sig = Some(
            Signature::new(format!(
                "ed25519:{}",
                base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())
            ))
            .expect("an Ed25519 signature in base64"),
        );
        let longest = syslog::longest(&record);
        if longest > syslog::MAX_MESSAGE {
            return Err(SealError::TooLong(longest));
        }
        let value = record.to_value();
        let line = serde_json::to_string(&value).expect("a record is JSON");
        let redacted = [never::SECRET_MARK, never::PATH_MARK, never::ADDRESS_MARK]
            .iter()
            .map(|m| line.matches(m).count())
            .sum();
        self.chain = self
            .chain
            .after(&value)
            .expect("a sealed record has its seq");
        Ok(Sealed {
            record,
            line,
            redacted,
        })
    }
}
