//! Passing an agent to a teammate (TS-17, `docs/TEAM-SERVER.md` §11): what goes through the team's server.
//!
//! - **The offer** is an object of the project's journal ([`HANDOFF`]), encrypted by the project's key and signed like
//!   every operation: which ticket, from which device to which, the branch pushed, whether a patch goes with it, and
//!   the deposit that holds the rest. It is deleted once taken, or when the passage is abandoned.
//! - **The deposit** is an encrypted file of the server ([`Team::deposit`], sealed as an object of the project): the
//!   agent's conversation as Escouade keeps it, and the patch of the uncommitted work when it goes as a patch. It is
//!   erased with the offer. The server sees a file of some size, never what it holds.
//! - **The claim** moves by the server's hand ([`Team::hand_claim`]): an offer is only trusted with the claim the
//!   server says was handed to this device, by the device the offer names (`Claim::handed_by`).
//! - **The machines**: a device in server mode says so in the project ([`Team::set_machine`]), so that the passage
//!   offers it as « la machine de l'équipe » beside the teammates.

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::ops::{OpBody, Value};
use escouade_team_crypto::{DeviceId, ObjectId, ProjectId};
use serde::{Deserialize, Serialize};

use crate::team::claims::PERSON;
use crate::team::Team;
use crate::Error;

/// The kind of the objects that offer a ticket's agent to a device.
pub const HANDOFF: &str = "handoff";

/// An offer to take a ticket's agent, as the journal holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    /// The ticket's object (hex) and its claim's opaque id (hex).
    pub ticket: String,
    pub claim: String,
    /// The device that passes it, and the one it goes to (hex).
    pub from: String,
    pub to: String,
    /// The deposit (hex): the conversation, and the patch when there is one.
    pub file: String,
    /// The branch pushed, and the remote it was pushed to.
    pub branch: String,
    pub remote: String,
    /// The uncommitted work goes as a patch in the deposit (else it was committed « en cours » on the branch, or there
    /// was none).
    pub patch: bool,
    /// The harness the agent ran on (its id), and its name.
    pub engine: String,
    pub agent: String,
    /// When it was offered (ms).
    pub at: i64,
}

impl Offer {
    fn fields(&self) -> BTreeMap<String, Value> {
        match serde_json::to_value(self) {
            Ok(Value::Object(m)) => m.into_iter().collect(),
            _ => BTreeMap::new(),
        }
    }

    fn from_fields(fields: &BTreeMap<String, Value>) -> Option<Offer> {
        let map: serde_json::Map<String, Value> =
            fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        serde_json::from_value(Value::Object(map)).ok()
    }
}

fn random16() -> Result<[u8; 16], Error> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    Ok(b)
}

impl Team {
    /// Deposits `bytes` on the server for a passage, encrypted as an object of `project`: its id.
    pub async fn deposit(&mut self, project: ProjectId, bytes: &[u8]) -> Result<ObjectId, Error> {
        let id = ObjectId(random16()?);
        self.put_file(project, id, bytes).await?;
        Ok(id)
    }

    /// Writes an offer in the project's journal (sent at the next step of the project): its object.
    pub fn offer_handoff(&mut self, project: ProjectId, offer: &Offer) -> Result<ObjectId, Error> {
        let object = ObjectId(random16()?);
        self.write(
            project,
            OpBody::Create {
                object,
                kind: HANDOFF.into(),
                fields: offer.fields(),
            },
        )?;
        Ok(object)
    }

    /// The offers of the project's journal, as this device last read it.
    pub fn handoffs(&mut self, project: ProjectId) -> Result<Vec<(ObjectId, Offer)>, Error> {
        Ok(self
            .replica(&project)?
            .objects(Some(HANDOFF))?
            .into_iter()
            .filter_map(|o| Offer::from_fields(&o.fields).map(|f| (o.id, f)))
            .collect())
    }

    /// The offer is over (taken or abandoned): out of the journal (sent at the next step).
    pub fn end_handoff(&mut self, project: ProjectId, object: ObjectId) -> Result<(), Error> {
        self.write(project, OpBody::Delete { object })
    }

    /// Says in the project whether this device is a machine in server mode: a passage offers it as the team's machine.
    /// Its name must be written first ([`Team::set_name`]). True when it was written.
    pub fn set_machine(&mut self, project: ProjectId, machine: bool) -> Result<bool, Error> {
        let object = ObjectId(self.device_id().0);
        let Some(current) = self.replica(&project)?.object(&object)? else {
            return Ok(false);
        };
        let now = current
            .fields
            .get("machine")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if now == machine {
            return Ok(false);
        }
        self.set_field(project, object, "machine", Value::Bool(machine))?;
        Ok(true)
    }

    /// The devices of the project that said they are machines in server mode.
    pub fn machines(&mut self, project: ProjectId) -> Result<BTreeSet<DeviceId>, Error> {
        Ok(self
            .replica(&project)?
            .objects(Some(PERSON))?
            .into_iter()
            .filter(|o| o.fields.get("machine").and_then(Value::as_bool) == Some(true))
            .map(|o| DeviceId(o.id.0))
            .collect())
    }
}
