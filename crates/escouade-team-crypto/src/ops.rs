//! The operations of a project's journal (TS-5, `docs/TEAM-SERVER.md` §2.4 and §3.8): « créer le ticket X »,
//! « changer le titre de X », « déplacer X », « écrire la page P, version 4 ».
//!
//! - **An operation** ([`Operation`]) names its author (member, device and the signature key of their leaf), its
//!   place in its device's chain (`dseq` from 1, and the digest of the operation before it of the same device: the
//!   server can neither reorder a device's operations nor drop one in silence), the head of the members' journal its
//!   author knew (the team's epoch and head: a fork shows), its stamp ([`crate::hlc::Hlc`]) and what it does
//!   ([`OpBody`]).
//! - **Signed** by the author's leaf key, then **sealed** ([`Sealed`]) under the project's key of the epoch: the
//!   team's epoch for a project of the whole team, the epoch of the project's own MLS group for a project kept to some
//!   members (`project.rs`). The server sees the epoch, the size, the hour and the device that sent it (§3.8): never
//!   the project's content, the object, the author's leaf, the chain or the stamp.
//! - **A snapshot** of a project's state is sealed and signed the same way, under another label: it takes the place
//!   of the start of the journal.
//!
//! What a copy does with the operations (the last writer wins, field by field) is the client's (`escouade-team`,
//! `replica.rs`): the format is here, so that the fuzzing and the audit read it in one place.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::content::verify_ed25519;
use crate::hlc::Hlc;
use crate::identity::{DeviceId, DeviceKind, Identity, MemberId, ObjectId, ProjectId};
use crate::keys::{aead_open, aead_seal, expand, EpochRecord};
use crate::Error;

/// A field's value: JSON, as the interface reads it.
pub type Value = serde_json::Value;

/// The bound of an operation's plaintext: the server bounds an entry at 1 MiB.
pub const MAX_OPERATION: usize = 900 * 1024;
/// The bound of a snapshot's plaintext: the server bounds it as a file, 25 MiB.
pub const MAX_SNAPSHOT: usize = 24 << 20;

/// What an operation does to the project's state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OpBody {
    /// Creates an object (a ticket: `kind` "ticket") with its first fields.
    Create {
        object: ObjectId,
        kind: String,
        fields: BTreeMap<String, Value>,
    },
    /// Changes one field. `base` is the stamp of the value its author saw: two writes from the same base are
    /// concurrent, and the one that loses is kept as a conflict to show.
    Set {
        object: ObjectId,
        field: String,
        value: Value,
        #[serde(default)]
        base: Option<Hlc>,
    },
    /// Moves an object: its `column` field, and its `rank` (its place in the column) when given.
    Move {
        object: ObjectId,
        column: String,
        #[serde(default)]
        rank: Option<String>,
        #[serde(default)]
        base: Option<Hlc>,
    },
    /// Deletes an object: final, whatever writes its fields afterwards.
    Delete { object: ObjectId },
    /// A whole version of a document (a spec page), after the versions `bases`. Two versions after the same base
    /// are a conflict, kept for the three-way merge on the device (TS-9).
    Version {
        object: ObjectId,
        version: ObjectId,
        bases: Vec<ObjectId>,
        content: Value,
    },
}

impl OpBody {
    pub fn object(&self) -> ObjectId {
        match self {
            OpBody::Create { object, .. }
            | OpBody::Set { object, .. }
            | OpBody::Move { object, .. }
            | OpBody::Delete { object }
            | OpBody::Version { object, .. } => *object,
        }
    }
}

/// Who wrote: a device of a member, and the signature key of its leaf.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    pub member: MemberId,
    pub device: DeviceId,
    #[serde(with = "hex::serde")]
    pub key: Vec<u8>,
}

impl Author {
    pub fn identity(&self) -> Identity {
        Identity {
            member: self.member,
            device: self.device,
            kind: DeviceKind::Device,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub project: ProjectId,
    pub author: Author,
    /// Its place in its device's chain, from 1.
    pub dseq: u64,
    /// The digest of the operation `dseq - 1` of the same device (zeros for the first).
    #[serde(with = "hex::serde")]
    pub prev: [u8; 32],
    /// The head of the members' journal its author knew: the team's epoch and its head.
    pub team_epoch: u64,
    #[serde(with = "hex::serde")]
    pub team_head: [u8; 32],
    pub hlc: Hlc,
    pub body: OpBody,
}

impl Operation {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("an operation always serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| Error::Encoding(format!("operation: {e}")))
    }

    /// What the next operation of the same device cites.
    pub fn digest_of(bytes: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"escouade/op-digest/v1\0");
        h.update(bytes);
        h.finalize().into()
    }
}

/// The state of a project at a number of its stream, made by a device of the team: it takes the place of the
/// operations before it. `state` is the copy's own form (`escouade-team`, `replica.rs`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub project: ProjectId,
    pub author: Author,
    /// The last number of the project's stream it holds.
    pub upto: u64,
    pub team_epoch: u64,
    #[serde(with = "hex::serde")]
    pub team_head: [u8; 32],
    /// Each device's chain at `upto`: its last `dseq` and that operation's digest.
    pub chains: BTreeMap<DeviceId, Chain>,
    #[serde(with = "hex::serde")]
    pub state: Vec<u8>,
}

impl Snapshot {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a snapshot always serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| Error::Encoding(format!("snapshot: {e}")))
    }
}

/// Where a device's chain is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chain {
    pub dseq: u64,
    #[serde(with = "hex::serde")]
    pub digest: [u8; 32],
}

/// Whose key seals: the team's (a project of the whole team) or the project's own MLS group's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Team,
    Project,
}

/// What is sealed: an operation or a snapshot (in the associated data only, never in the clear).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    Operation,
    Snapshot,
}

impl What {
    fn max(self) -> usize {
        match self {
            What::Operation => MAX_OPERATION,
            What::Snapshot => MAX_SNAPSHOT,
        }
    }

    fn label(self) -> &'static [u8] {
        match self {
            What::Operation => b"op",
            What::Snapshot => b"snapshot",
        }
    }
}

/// What the server keeps of an operation or a snapshot: its scope and epoch, the rest encrypted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    pub scope: Scope,
    pub epoch: u64,
    #[serde(with = "hex::serde")]
    pub nonce: [u8; 24],
    #[serde(with = "hex::serde")]
    pub ciphertext: Vec<u8>,
}

/// What is inside: the body (an operation or a snapshot, JSON) and its author's signature.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Signed {
    #[serde(with = "hex::serde")]
    body: Vec<u8>,
    #[serde(with = "hex::serde")]
    signature: Vec<u8>,
}

fn seal_key(record: &EpochRecord, project: &ProjectId, what: What) -> [u8; 32] {
    let mut label = b"escouade/sealed-".to_vec();
    label.extend_from_slice(what.label());
    label.extend_from_slice(b"/v1");
    expand(&record.project_key(project), &label, &[])
}

fn aad(what: What, project: &ProjectId, scope: Scope, epoch: u64) -> Vec<u8> {
    let mut out = b"escouade/sealed/v1\0".to_vec();
    out.extend_from_slice(what.label());
    out.push(0);
    out.extend_from_slice(&project.0);
    out.push(match scope {
        Scope::Team => 0,
        Scope::Project => 1,
    });
    out.extend_from_slice(&epoch.to_be_bytes());
    out
}

fn signed_bytes(what: What, project: &ProjectId, body: &[u8]) -> Vec<u8> {
    let mut out = b"escouade/signed/v1\0".to_vec();
    out.extend_from_slice(what.label());
    out.push(0);
    out.extend_from_slice(&project.0);
    out.extend_from_slice(body);
    out
}

impl Sealed {
    /// Signs `body` with `sign` (the author's leaf key), then encrypts it under the project's key of `record`.
    pub fn seal(
        what: What,
        record: &EpochRecord,
        scope: Scope,
        project: &ProjectId,
        nonce: [u8; 24],
        body: Vec<u8>,
        sign: impl FnOnce(&[u8]) -> Result<Vec<u8>, Error>,
    ) -> Result<Self, Error> {
        if body.len() > what.max() {
            return Err(Error::Encoding(
                "an operation or a snapshot too large".into(),
            ));
        }
        let signature = sign(&signed_bytes(what, project, &body))?;
        let plain = serde_json::to_vec(&Signed { body, signature })
            .map_err(|e| Error::Encoding(e.to_string()))?;
        let ciphertext = aead_seal(
            &seal_key(record, project, what),
            &nonce,
            &aad(what, project, scope, record.epoch),
            &plain,
        );
        Ok(Self {
            scope,
            epoch: record.epoch,
            nonce,
            ciphertext,
        })
    }

    /// Decrypts under `record` (the epoch's of its scope), and checks the signature by `author_of(body)`, which must
    /// be a leaf of that epoch with that key. Gives the body.
    pub fn open(
        &self,
        what: What,
        record: &EpochRecord,
        project: &ProjectId,
        author_of: impl FnOnce(&[u8]) -> Result<Author, Error>,
    ) -> Result<Vec<u8>, Error> {
        if self.epoch != record.epoch {
            return Err(Error::UnknownEpoch(self.epoch));
        }
        let plain = aead_open(
            &seal_key(record, project, what),
            &self.nonce,
            &aad(what, project, self.scope, self.epoch),
            &self.ciphertext,
        )?;
        let signed: Signed =
            serde_json::from_slice(&plain).map_err(|e| Error::Encoding(format!("sealed: {e}")))?;
        let author = author_of(&signed.body)?;
        let identity = author.identity().to_bytes();
        if !record
            .members
            .iter()
            .any(|m| m.identity == identity && m.signature_key == author.key)
        {
            return Err(Error::BadSignature);
        }
        verify_ed25519(
            &author.key,
            &signed_bytes(what, project, &signed.body),
            &signed.signature,
        )?;
        Ok(signed.body)
    }

    /// Opens an operation: its author a leaf of the epoch, its signature good, of this project, its stamp its
    /// device's. The chain and the members' head are the caller's to check (they need what the device saw).
    pub fn open_operation(
        &self,
        record: &EpochRecord,
        project: &ProjectId,
    ) -> Result<Operation, Error> {
        Ok(self.open_operation_bytes(record, project)?.0)
    }

    /// [`Self::open_operation`], with the bytes its author signed (its digest is theirs).
    pub fn open_operation_bytes(
        &self,
        record: &EpochRecord,
        project: &ProjectId,
    ) -> Result<(Operation, Vec<u8>), Error> {
        let mut op = None;
        let bytes = self.open(What::Operation, record, project, |body| {
            let o = Operation::from_bytes(body)?;
            let author = o.author.clone();
            op = Some(o);
            Ok(author)
        })?;
        let op = op.expect("set when opened");
        if op.project != *project {
            return Err(Error::Encoding("an operation of another project".into()));
        }
        if op.hlc.device != op.author.device {
            return Err(Error::Encoding("a stamp of another device".into()));
        }
        if op.dseq == 0 || (op.dseq == 1) != (op.prev == [0u8; 32]) {
            return Err(Error::Encoding("a chain that does not start at 1".into()));
        }
        Ok((op, bytes))
    }

    pub fn open_snapshot(
        &self,
        record: &EpochRecord,
        project: &ProjectId,
    ) -> Result<Snapshot, Error> {
        let mut snap = None;
        self.open(What::Snapshot, record, project, |body| {
            let s = Snapshot::from_bytes(body)?;
            let author = s.author.clone();
            snap = Some(s);
            Ok(author)
        })?;
        let snap = snap.expect("set when opened");
        if snap.project != *project {
            return Err(Error::Encoding("a snapshot of another project".into()));
        }
        Ok(snap)
    }
}
