//! What the core keeps of the protocol of `copilot --server --stdio`, for the servers it starts itself for an account
//! (`terminal`): the agents' own are its plugin's (`dev.escouade.copilot-cli`, INT-10.3). Messages framed by
//! `Content-Length` headers, as `plugins::harness` frames them for its plugin.

pub(super) use super::INHERITED_TOKENS;

/// Whether the variables of a process give it a folder of its own (an account other than its Principal).
pub(super) fn own_home(env: &[(String, String)]) -> bool {
    env.iter()
        .any(|(k, v)| k == "COPILOT_HOME" && !v.is_empty())
}

#[cfg(test)]
mod tests {}
