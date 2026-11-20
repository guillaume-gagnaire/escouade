//! « Les agents peuvent répondre aux demandes des autres agents » (ESC-423, ESC-513): one rule for
//! the requests an agent stopped for (`answer_permission`, `answer_question`) and for the requests
//! of the box « À toi » the user leaves to an agent (`answer_ask`, docs/INBOX.md § 8.5). Nobody by
//! default; the squad lead the user chose for the project of the request alone; or every agent. Each
//! caller says its refusal in its own words, but the same agents are refused for the same reason.

use crate::model::AgentsAnswer;

/// Why an agent may not answer another agent's request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// « Personne »: no agent answers for another, the chef neither.
    Nobody,
    /// « Seulement le chef d’escouade », and this agent is not the project's.
    NotTheChef,
}

/// Whether `agent` may answer a request of a project whose squad lead is `chef`, as `setting` says.
pub fn may_answer(setting: AgentsAnswer, chef: Option<&str>, agent: &str) -> Result<(), Refusal> {
    match setting {
        AgentsAnswer::All => Ok(()),
        AgentsAnswer::Chef if chef == Some(agent) => Ok(()),
        AgentsAnswer::Chef => Err(Refusal::NotTheChef),
        AgentsAnswer::Nobody => Err(Refusal::Nobody),
    }
}
