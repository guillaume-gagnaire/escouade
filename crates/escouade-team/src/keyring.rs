//! What the costs (TS-16, `costs.rs`) and the audit of the stations (AU-3, `audit.rs`) share: a stream of keys, where
//! the devices that hold a key of an epoch announce their own key of the use, draw the pair of each epoch, wrap it for
//! one another, and turn it when one of them leaves.
//!
//! - [`Ring`]: what a device read of such a stream (its own key, the devices' keys, the pairs of each epoch, the seeds
//!   it holds, its cursor, the last entry read for a rollback). Kept flattened in each use's state, under the same
//!   names as TS-16 wrote them.
//! - [`read_keys`]: reads the stream; each entry is signed under the team's key and judged by the use (`accept`: who may
//!   write it). The first key of an epoch in the stream's order wins.
//! - [`serve`]: what a holder's device does: its own key written, a pair drawn when there is none or the current one is
//!   wrapped for a device that is no longer a holder, the pair wrapped for the holders that lack it.
//! - [`is_stale`]: a key wrapped for a device that is no longer a holder: nothing is sealed to it.

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::epochkey::{EpochKey, Use};
use escouade_team_crypto::recovery::Sealed;
use escouade_team_crypto::{Identity, MemberId, ObjectId, ProjectId};
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::team::Team;
use crate::{wire, CryptoError, Error};

/// A wrap of a pair for a device.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Wrap {
    pub device: [u8; 16],
    pub sealed: Sealed,
}

/// The entries of a stream of keys that the ring reads; a use's own entries (the costs' summaries) are given back.
pub(crate) enum Step {
    /// The key of the use of the device that signs it, to receive the wraps.
    Device { public: Vec<u8> },
    /// The pair of `epoch` and its wraps.
    Key {
        epoch: u64,
        public: Vec<u8>,
        wraps: Vec<Wrap>,
    },
    /// Wraps of the pair of `epoch` for devices that came after it.
    Wrap { epoch: u64, wraps: Vec<Wrap> },
}

/// A use's entry of its stream of keys: the ring's steps, or the use's own.
pub(crate) trait Entry: Serialize + serde::de::DeserializeOwned + Sized {
    fn step(self) -> Result<Step, Self>;
    fn of(step: Step) -> Self;
}

/// A pair of an epoch, as this device read it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct KeyInfo {
    pub public: Vec<u8>,
    /// The devices it is wrapped for (hex).
    pub devices: BTreeSet<String>,
    pub seq: u64,
}

/// Why this device stopped reading a stream of keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Alarm {
    /// The last entry read is gone.
    Rollback,
    /// The last entry read changed.
    Rewritten,
}

/// What a device read of a stream of keys (flattened in a use's state: the names are TS-16's).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Ring {
    /// This device's own key of the use (a holder's device draws it), and whether it was written.
    pub own: Option<[u8; 32]>,
    pub announced: bool,
    /// The keys of the devices, as their `Device` entries say (device hex → (member hex, public)).
    pub devices: BTreeMap<String, (String, Vec<u8>)>,
    #[serde(deserialize_with = "by_epoch")]
    pub keys: BTreeMap<u64, KeyInfo>,
    /// The pairs this device holds, by epoch.
    #[serde(deserialize_with = "by_epoch")]
    pub seeds: BTreeMap<u64, [u8; 32]>,
    pub key_cursor: u64,
    pub key_last: Option<(u64, [u8; 32])>,
    pub alarm: Option<Alarm>,
    /// Entries refused at the reading (not written by whom the use allows, not signed, not opened).
    pub refused: usize,
    /// The highest epoch a key refused for its writer named (0: none): the next pair is drawn above it.
    #[serde(skip_serializing_if = "is_zero")]
    pub top: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// A map by epoch, read back from a state where it was flattened: its keys come as strings there (`"1"`, JSON's), as
/// numbers elsewhere.
fn by_epoch<'de, D, V>(d: D) -> Result<BTreeMap<u64, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    V: Deserialize<'de>,
{
    #[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    #[serde(untagged)]
    enum Key {
        N(u64),
        S(String),
    }
    BTreeMap::<Key, V>::deserialize(d)?
        .into_iter()
        .map(|(k, v)| match k {
            Key::N(n) => Ok((n, v)),
            Key::S(s) => s
                .parse()
                .map(|n| (n, v))
                .map_err(|_| serde::de::Error::custom(format!("an epoch, not « {s} »"))),
        })
        .collect()
}

impl Ring {
    /// The current key: the highest epoch read.
    pub fn current(&self) -> Option<(u64, &KeyInfo)> {
        self.keys.iter().next_back().map(|(e, k)| (*e, k))
    }

    /// This device holds the current pair.
    pub fn holds(&self) -> bool {
        self.current()
            .is_some_and(|(e, _)| self.seeds.contains_key(&e))
    }

    /// After an alarm, once a human looked: read from the start again.
    pub fn retry(&mut self) {
        self.alarm = None;
        self.key_last = None;
        self.key_cursor = 0;
        self.keys.clear();
        self.devices.clear();
        self.announced = false;
    }

    fn own_key<U: Use>(&self) -> Result<Option<EpochKey<U>>, Error> {
        let p = provider();
        self.own
            .map(|seed| EpochKey::<U>::from_seed(p.crypto(), seed))
            .transpose()
            .map_err(Error::from)
    }
}

pub(crate) fn provider() -> OpenMlsRustCrypto {
    OpenMlsRustCrypto::default()
}

fn digest(e: &wire::Entry) -> [u8; 32] {
    Sha256::digest(wire::encode(e)).into()
}

/// A key wrapped for a device that is no longer a holder (`holders`: device hex → member hex).
pub(crate) fn is_stale(key: &KeyInfo, holders: &BTreeMap<String, String>) -> bool {
    key.devices.iter().any(|d| !holders.contains_key(d))
}

pub(crate) enum Opened<T> {
    Ok(MemberId, [u8; 16], T),
    /// Signed by a device of the team that `accept` does not take: refused, its content said for what it names.
    Unallowed(T),
    Refused,
    Later,
}

/// An entry of one of the team's own streams, opened under the team's key: its signer (as of the entry's epoch), when
/// `accept` takes it (the signer, the entry's epoch), and its content.
pub(crate) fn open<T: serde::de::DeserializeOwned>(
    team: &Team,
    stream: ProjectId,
    payload: &[u8],
    accept: &(dyn Fn(&Team, u64, &Identity) -> bool + Sync),
) -> Opened<T> {
    let Ok(sealed) = wire::decode::<SealedObject>(payload) else {
        return Opened::Refused;
    };
    if sealed.header.project != stream {
        return Opened::Refused;
    }
    let plain = match team.device().open(&sealed) {
        Ok(p) => p,
        Err(CryptoError::Behind(_) | CryptoError::Fork { .. }) => return Opened::Later,
        Err(_) => return Opened::Refused,
    };
    let Some(who) = team.signer_at(sealed.header.epoch, &sealed.header.author) else {
        return Opened::Refused;
    };
    match wire::decode::<T>(&plain) {
        Ok(v) if accept(team, sealed.header.epoch, &who) => Opened::Ok(who.member, who.device.0, v),
        Ok(v) => Opened::Unallowed(v),
        Err(_) => Opened::Refused,
    }
}

/// `entry`, sealed under the team's key for `stream` and signed by this device's leaf.
pub(crate) fn seal_entry<T: Serialize>(
    team: &Team,
    stream: ProjectId,
    entry: &T,
) -> Result<Vec<u8>, Error> {
    let mut object = [0u8; 16];
    getrandom::fill(&mut object).map_err(|e| Error::Encoding(e.to_string()))?;
    let sealed = team
        .device()
        .seal(stream, ObjectId(object), 0, &wire::encode(entry))?;
    Ok(wire::encode(&sealed))
}

/// Reads `stream` from where `ring` stopped. `accept` says who may write an entry there; `other` takes a use's own
/// entry (false: refused).
pub(crate) async fn read_keys<U: Use, E: Entry>(
    team: &Team,
    stream: ProjectId,
    ring: &mut Ring,
    accept: &(dyn Fn(&Team, u64, &Identity) -> bool + Sync),
    other: &mut (dyn FnMut(E) -> bool + Send),
) -> Result<(), Error> {
    if ring.alarm.is_some() {
        return Ok(());
    }
    let client = team.client().clone();
    if let Some((seq, d)) = ring.key_last {
        let page = client.read_project(&stream.0, seq - 1, Some(1)).await?;
        match page.entries.first() {
            Some(e) if e.seq == seq => {
                if digest(e) != d {
                    ring.alarm = Some(Alarm::Rewritten);
                    return Ok(());
                }
            }
            // The very entry removed by the server's operator (SR2-02): a hole, the reading goes on after it.
            _ if client.was_dropped(Some(&stream.0), seq, &d).await? => ring.key_last = None,
            _ => {
                ring.alarm = Some(Alarm::Rollback);
                return Ok(());
            }
        }
    }
    let me = team.identity().device.0;
    let id = team.id();
    'read: loop {
        let page = client
            .read_project(&stream.0, ring.key_cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= ring.key_cursor {
                break 'read;
            }
            match open::<E>(team, stream, &e.payload, accept) {
                Opened::Ok(member, device, entry) => match entry.step() {
                    Ok(Step::Device { public }) => {
                        ring.devices
                            .insert(hex::encode(device), (hex::encode(member.0), public));
                        if device == me {
                            ring.announced = true;
                        }
                    }
                    Ok(Step::Key {
                        epoch,
                        public,
                        wraps,
                    }) => {
                        // The first key of an epoch in the stream's order wins: two holders who drew one at once agree
                        // on it.
                        if ring.keys.contains_key(&epoch)
                            || ring.current().is_some_and(|(c, _)| c >= epoch)
                        {
                            ring.refused += 1;
                        } else {
                            let mut info = KeyInfo {
                                public,
                                devices: BTreeSet::new(),
                                seq: e.seq,
                            };
                            take_wraps::<U>(ring, &mut info, &wraps, &id, epoch, me);
                            ring.keys.insert(epoch, info);
                        }
                    }
                    Ok(Step::Wrap { epoch, wraps }) => {
                        if let Some(mut info) = ring.keys.remove(&epoch) {
                            take_wraps::<U>(ring, &mut info, &wraps, &id, epoch, me);
                            ring.keys.insert(epoch, info);
                        } else {
                            ring.refused += 1;
                        }
                    }
                    Err(own) => {
                        if !other(own) {
                            ring.refused += 1;
                        }
                    }
                },
                // Refused, but the epoch it names is past: a pair drawn here goes above it, so that every device, which
                // took it or not as of its own history (a machine come after it does not), agrees on the next one.
                Opened::Unallowed(entry) => {
                    if let Ok(Step::Key { epoch, .. }) = entry.step() {
                        ring.top = ring.top.max(epoch);
                    }
                    ring.refused += 1;
                }
                Opened::Refused => ring.refused += 1,
                Opened::Later => break 'read,
            }
            ring.key_cursor = e.seq;
            ring.key_last = Some((e.seq, digest(e)));
        }
        if !page.more || page.entries.is_empty() {
            break;
        }
    }
    Ok(())
}

/// The wraps of a key: the devices it is for, and its seed when one is for this device.
fn take_wraps<U: Use>(
    ring: &mut Ring,
    info: &mut KeyInfo,
    wraps: &[Wrap],
    team: &[u8; 16],
    epoch: u64,
    me: [u8; 16],
) {
    let p = provider();
    for w in wraps {
        info.devices.insert(hex::encode(w.device));
        if w.device == me && !ring.seeds.contains_key(&epoch) {
            if let Ok(Some(own)) = ring.own_key::<U>() {
                if let Ok(k) = own.unwrap_seed(p.crypto(), team, epoch, &me, &w.sealed) {
                    ring.seeds.insert(epoch, *k.seed());
                }
            }
        }
    }
}

/// What a holder's device does (`holders`: the devices that must hold the pair, device hex → member hex; this one is
/// among them): its own key written, a pair drawn when there is none or the current one is stale, the pair wrapped for
/// the holders whose key was read and that have none. Whether it wrote.
pub(crate) async fn serve<U: Use, E: Entry>(
    team: &Team,
    stream: ProjectId,
    ring: &mut Ring,
    holders: &BTreeMap<String, String>,
) -> Result<bool, Error> {
    if ring.alarm.is_some() {
        return Ok(false);
    }
    let p = provider();
    let client = team.client().clone();
    let mut wrote = false;
    if ring.own.is_none() {
        ring.own = Some(*EpochKey::<U>::generate(p.crypto(), p.rand())?.seed());
        ring.announced = false;
    }
    let own = ring.own_key::<U>()?.expect("drawn above");
    if !ring.announced {
        let entry = E::of(Step::Device {
            public: own.public(),
        });
        client
            .append_project(&stream.0, &seal_entry(team, stream, &entry)?)
            .await?;
        wrote = true;
    }
    // The holders' devices whose key was read, this one's included (its entry may not be read yet).
    let mut ready: BTreeMap<String, Vec<u8>> = ring
        .devices
        .iter()
        .filter(|(d, (m, _))| holders.get(*d) == Some(m))
        .map(|(d, (_, k))| (d.clone(), k.clone()))
        .collect();
    let me = team.identity().device.0;
    ready.insert(hex::encode(me), own.public());
    let id = team.id();
    let wraps_for = |key: &EpochKey<U>, epoch: u64, to: &BTreeMap<String, Vec<u8>>| {
        to.iter()
            .filter_map(|(d, public)| {
                let mut device = [0u8; 16];
                hex::decode_to_slice(d, &mut device).ok()?;
                let sealed = key
                    .wrap_seed(p.crypto(), public, &id, epoch, &device)
                    .ok()?;
                Some(Wrap { device, sealed })
            })
            .collect::<Vec<_>>()
    };
    match ring.current() {
        Some((epoch, key)) if !is_stale(key, holders) => {
            if let Some(seed) = ring.seeds.get(&epoch) {
                let missing: BTreeMap<String, Vec<u8>> = ready
                    .iter()
                    .filter(|(d, _)| !key.devices.contains(*d))
                    .map(|(d, k)| (d.clone(), k.clone()))
                    .collect();
                if !missing.is_empty() {
                    let k = EpochKey::<U>::from_seed(p.crypto(), *seed)?;
                    let entry = E::of(Step::Wrap {
                        epoch,
                        wraps: wraps_for(&k, epoch, &missing),
                    });
                    client
                        .append_project(&stream.0, &seal_entry(team, stream, &entry)?)
                        .await?;
                    wrote = true;
                }
            }
        }
        other => {
            let epoch = other.map_or(0, |(e, _)| e).max(ring.top).saturating_add(1);
            let key = EpochKey::<U>::generate(p.crypto(), p.rand())?;
            let entry = E::of(Step::Key {
                epoch,
                public: key.public(),
                wraps: wraps_for(&key, epoch, &ready),
            });
            client
                .append_project(&stream.0, &seal_entry(team, stream, &entry)?)
                .await?;
            wrote = true;
        }
    }
    Ok(wrote)
}
