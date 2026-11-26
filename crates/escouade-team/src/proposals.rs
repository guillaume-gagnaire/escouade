//! What a member proposes to the administrators (TS-7): the Kanban of a project of the team's list. Only an
//! administrator writes the list ([`crate::catalog`]); a member who shares the Kanban of a project of the list proposes
//! it here, and an administrator's screen offers « Inscrire le Kanban » (the administrator writes the entry).
//!
//! - **Where it lives**: a stream of the server of its own ([`PROPOSAL_STREAM`], a project id no client draws), open
//!   to every device of the team. Each entry is an object sealed under the team's key of its epoch and signed by the
//!   leaf of its author: the server sees that a proposal was written, its size and its hour, never what it is about.
//! - **Not trusted blindly**: a proposal is only a proposal. An entry signed by no leaf of the team is refused at the
//!   reading; the board it names is shown to an administrator, who decides. A server that hides a proposal only keeps
//!   an administrator from seeing it: the member's screen still says « proposé à un administrateur », and the
//!   administrator can still write the Kanban by hand.

use std::collections::BTreeMap;

use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::{ObjectId, ProjectId};
use serde::{Deserialize, Serialize};

use crate::team::Team;
use crate::{wire, CryptoError, Error};

/// The stream of the proposals: a project id no client draws (16 bytes).
pub const PROPOSAL_STREAM: ProjectId = ProjectId(*b"escouade/propose");

/// Where this device keeps what it read of the stream, in the team's encrypted base.
const STATE: &str = "proposals";

/// An entry of the stream, sealed and signed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Entry {
    /// « Ce Kanban est celui de ce projet de la liste »: the id of the project in the list, and of the journal.
    Board {
        item: [u8; 16],
        board: [u8; 16],
        at: u64,
    },
}

/// A proposal, as this device read it: the last one of each project of the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    /// The project of the list (hex), and the journal of the Kanban proposed (hex).
    pub item: String,
    pub board: String,
    /// The member who proposed it (hex).
    pub by: String,
    pub at: u64,
    pub seq: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    cursor: u64,
    by_item: BTreeMap<String, Proposal>,
    /// The entries refused at the reading (not signed by the team).
    refused: usize,
}

/// What this device knows of the proposals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposals {
    pub proposals: Vec<Proposal>,
    pub refused: usize,
}

impl Proposals {
    /// The Kanban proposed for a project of the list.
    pub fn of(&self, item: &str) -> Option<&Proposal> {
        self.proposals.iter().find(|p| p.item == item)
    }
}

fn load(team: &Team) -> Result<State, Error> {
    Ok(team.load_extra(STATE)?.unwrap_or_default())
}

fn view_of(state: &State) -> Proposals {
    Proposals {
        proposals: state.by_item.values().cloned().collect(),
        refused: state.refused,
    }
}

enum Opened {
    Refused,
    /// An epoch this device has not reached yet (or a fork, said by the team's own reading): read again later.
    Later,
}

fn open(team: &Team, payload: &[u8]) -> Result<(String, Entry), Opened> {
    let sealed: SealedObject = wire::decode(payload).map_err(|_| Opened::Refused)?;
    if sealed.header.project != PROPOSAL_STREAM {
        return Err(Opened::Refused);
    }
    let plain = match team.device().open(&sealed) {
        Ok(p) => p,
        Err(CryptoError::Behind(_) | CryptoError::Fork { .. }) => return Err(Opened::Later),
        Err(_) => return Err(Opened::Refused),
    };
    let who = team
        .signer_at(sealed.header.epoch, &sealed.header.author)
        .ok_or(Opened::Refused)?;
    // A machine of the team proposes nothing to the team (OE-1): the server keeps it off this stream, the devices too.
    if team.is_machine(&who.member) {
        return Err(Opened::Refused);
    }
    let entry: Entry = wire::decode(&plain).map_err(|_| Opened::Refused)?;
    Ok((who.member.to_string(), entry))
}

/// Reads what is new in the proposals.
pub async fn sync(team: &mut Team) -> Result<Proposals, Error> {
    let mut state = load(team)?;
    let client = team.client().clone();
    'read: loop {
        let page = client
            .read_project(&PROPOSAL_STREAM.0, state.cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.cursor {
                break 'read;
            }
            match open(team, &e.payload) {
                Ok((by, Entry::Board { item, board, at })) => {
                    let item = hex::encode(item);
                    state.by_item.insert(
                        item.clone(),
                        Proposal {
                            item,
                            board: hex::encode(board),
                            by,
                            at,
                            seq: e.seq,
                        },
                    );
                }
                Err(Opened::Refused) => state.refused += 1,
                Err(Opened::Later) => break 'read,
            }
            state.cursor = e.seq;
        }
        if !page.more || page.entries.is_empty() {
            break;
        }
    }
    team.save_extra(STATE, &state)?;
    Ok(view_of(&state))
}

/// The proposals as this device read them last, without reading the server.
pub fn view(team: &Team) -> Result<Proposals, Error> {
    Ok(view_of(&load(team)?))
}

fn id16(hex_id: &str) -> Result<[u8; 16], Error> {
    let mut b = [0u8; 16];
    hex::decode_to_slice(hex_id.trim(), &mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    Ok(b)
}

/// The entry sealed by this device, as it goes to the server: a fresh object id each time.
pub fn seal(team: &Team, entry: &Entry) -> Result<Vec<u8>, Error> {
    let mut object = [0u8; 16];
    getrandom::fill(&mut object).map_err(|e| Error::Encoding(e.to_string()))?;
    let sealed = team
        .device()
        .seal(PROPOSAL_STREAM, ObjectId(object), 0, &wire::encode(entry))?;
    Ok(wire::encode(&sealed))
}

/// Any device of the team proposes the journal `board` as the Kanban of the project `item` of the list.
pub async fn propose(
    team: &mut Team,
    item: &str,
    board: &str,
    at: u64,
) -> Result<Proposals, Error> {
    if team.is_service() {
        return Err(CryptoError::NotAdmin.into());
    }
    let entry = Entry::Board {
        item: id16(item)?,
        board: id16(board)?,
        at,
    };
    let payload = seal(team, &entry)?;
    team.client()
        .append_project(&PROPOSAL_STREAM.0, &payload)
        .await?;
    sync(team).await
}
