//! The claims of a team's tickets (TS-6, `docs/TEAM-SERVER.md` §4.1, §4.2 and §2.6).
//!
//! - **The claim** of a ticket goes through the server, never offline: one demand passes, the others are told which
//!   device holds it. Its id is opaque (128 bits, a field of the encrypted ticket, never its object's id): the server
//!   knows that a thing of this project is held, by which device, until when, and nothing of what it is.
//! - **The lease** is renewed while the agent runs ([`Team::renew_claim`]) and given back when it stops
//!   ([`Team::release_claim`]). Each claim gives a new token; a write of the holder carries it
//!   ([`Team::write_fenced`]), and the server refuses one under a token that another device took over since.
//! - **The access** of a project on the server follows its keys: the whole team for a project of the whole team, the
//!   devices of its members for a project kept to some ([`Team::sync_access`], by an administrator).
//! - **The keys** of the tickets are numbered by a counter of the server ([`Team::next_key`]): it sees the counter
//!   alone, the prefix stays in the encrypted project, and the device forms the key.
//! - **The names** of the devices: each device writes its member's name in the project, encrypted
//!   ([`Team::set_name`]); « pris par l'appareil D » is said with it.

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::ops::{OpBody, Scope, Value};
use escouade_team_crypto::{DeviceId, MemberId, ObjectId, ProjectId};

use super::Team;
use crate::wire::{AccessWire, ClaimAnswer, Claims};
use crate::Error;

/// The kind of the objects that name a device of the project (its id is the device's).
pub const PERSON: &str = "person";

impl Team {
    /// Takes a ticket by its claim's opaque id, as it was at `expected` (the claim's version this device saw), when
    /// given. Offline it fails (`ClientError::Net`): a ticket of the team is never taken without the server.
    pub async fn take_claim(
        &mut self,
        project: ProjectId,
        claim: [u8; 16],
        expected: Option<u64>,
    ) -> Result<ClaimAnswer, Error> {
        let answer = self.client.claim(&project.0, &claim, expected).await;
        self.heard(answer)
    }

    /// Renews a lease, under its token: `ClientError::Claim` (`claim_lost`) once another device took the ticket.
    pub async fn renew_claim(
        &mut self,
        project: ProjectId,
        claim: [u8; 16],
        token: u64,
    ) -> Result<ClaimAnswer, Error> {
        let answer = self.client.renew_claim(&project.0, &claim, token).await;
        self.heard(answer)
    }

    /// Gives a ticket back.
    pub async fn release_claim(
        &mut self,
        project: ProjectId,
        claim: [u8; 16],
        token: u64,
    ) -> Result<ClaimAnswer, Error> {
        let answer = self.client.release_claim(&project.0, &claim, token).await;
        self.heard(answer)
    }

    /// Hands a ticket to another device of the project (TS-17, §11), under this device's token: the other device holds
    /// it at once under a new token, and this one's old token writes nothing more.
    pub async fn hand_claim(
        &mut self,
        project: ProjectId,
        claim: [u8; 16],
        token: u64,
        to: DeviceId,
    ) -> Result<ClaimAnswer, Error> {
        let answer = self
            .client
            .hand_claim(&project.0, &claim, token, &to.0)
            .await;
        self.heard(answer)
    }

    /// Takes back a ticket this device handed, under the token of the hand, while it was not taken (the passage
    /// abandoned): `claim_lost` once it was.
    pub async fn withdraw_claim(
        &mut self,
        project: ProjectId,
        claim: [u8; 16],
        token: u64,
    ) -> Result<ClaimAnswer, Error> {
        let answer = self.client.withdraw_claim(&project.0, &claim, token).await;
        self.heard(answer)
    }

    /// A project's claims, with the server's time.
    pub async fn claims(&mut self, project: ProjectId) -> Result<Claims, Error> {
        let answer = self.client.claims(&project.0).await;
        self.heard(answer)
    }

    /// The next number of a project's tickets.
    pub async fn next_key(&mut self, project: ProjectId) -> Result<u64, Error> {
        let answer = self.client.next_key(&project.0).await;
        self.heard(answer)
    }

    /// Puts the project's access on the server in step with its keys: the whole team for a project of the whole team,
    /// the devices of its members for one kept to some. A device that is no administrator sets it for a project the
    /// server does not know yet (its creation) alone. True when it was sent.
    pub async fn sync_access(&mut self, project: ProjectId) -> Result<bool, Error> {
        let scope = self.open_replica(&project)?.meta()?.scope;
        let wanted = match scope {
            Some(Scope::Team) => AccessWire {
                everyone: true,
                devices: Vec::new(),
            },
            Some(Scope::Project) => {
                let members: BTreeSet<MemberId> = self.project_members(&project)?;
                let devices = self
                    .members()?
                    .into_iter()
                    .filter(|(m, _, _)| members.contains(m))
                    .flat_map(|(_, _, d)| d)
                    .map(|d| serde_bytes::ByteBuf::from(d.0.to_vec()))
                    .collect();
                AccessWire {
                    everyone: false,
                    devices,
                }
            }
            None => return Ok(false),
        };
        let now = self.client.access(&project.0).await;
        let now = self.heard(now)?;
        let same = now.everyone == wanted.everyone
            && now.devices.iter().collect::<BTreeSet<_>>()
                == wanted.devices.iter().collect::<BTreeSet<_>>();
        if same {
            return Ok(false);
        }
        let set = self.client.set_access(&project.0, &wanted).await;
        match self.heard(set) {
            Ok(()) => Ok(true),
            // A member's device does not change it once it exists: an administrator's will.
            Err(Error::Client(e)) if e.code() == Some("not_admin") => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Writes the name of this device's member in the project, encrypted, when it is not there yet or changed:
    /// « pris par Alice » is said with it on the other devices. True when it was written.
    pub fn set_name(&mut self, project: ProjectId, name: &str) -> Result<bool, Error> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(false);
        }
        let me = self.device.author();
        let object = ObjectId(me.device.0);
        let current = self.open_replica(&project)?.object(&object)?;
        match current {
            Some(o) if o.fields.get("name").and_then(Value::as_str) == Some(name) => Ok(false),
            Some(_) => {
                self.set_field(project, object, "name", Value::String(name.to_string()))?;
                Ok(true)
            }
            None => {
                let mut fields = BTreeMap::new();
                fields.insert("name".to_string(), Value::String(name.to_string()));
                fields.insert(
                    "member".to_string(),
                    Value::String(hex::encode(me.member.0)),
                );
                self.write(
                    project,
                    OpBody::Create {
                        object,
                        kind: PERSON.into(),
                        fields,
                    },
                )?;
                Ok(true)
            }
        }
    }

    /// The name of a device of the team, as its member wrote it in the project; else that of another device of the
    /// same member.
    pub fn name_of(&mut self, project: ProjectId, device: &DeviceId) -> Option<String> {
        let names = self.names(project).ok()?;
        if let Some(n) = names.get(device) {
            return Some(n.clone());
        }
        let member = self.member_of(device)?;
        let members = self.members().ok()?;
        let (_, _, devices) = members.into_iter().find(|(m, _, _)| *m == member)?;
        devices.iter().find_map(|d| names.get(d).cloned())
    }

    /// The names the devices wrote in the project.
    pub fn names(&mut self, project: ProjectId) -> Result<BTreeMap<DeviceId, String>, Error> {
        Ok(self
            .open_replica(&project)?
            .objects(Some(PERSON))?
            .into_iter()
            .filter_map(|o| {
                let name = o.fields.get("name")?.as_str()?.to_string();
                Some((DeviceId(o.id.0), name))
            })
            .collect())
    }

    /// The member a device of the team is of.
    pub fn member_of(&self, device: &DeviceId) -> Option<MemberId> {
        self.members()
            .ok()?
            .into_iter()
            .find(|(_, _, d)| d.contains(device))
            .map(|(m, _, _)| m)
    }
}
