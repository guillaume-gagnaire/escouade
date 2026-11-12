//! A turn a safeguard of the model's provider refused (ESC-416): Anthropic's classifiers read the whole request, so a
//! conversation that holds the refused content is refused again at each turn, however harmless its next message. In
//! Claude Code's own interface, a double Escape goes back before the message; here the agent's next start leaves the
//! refused turn out instead: it forks its session at the end of its last whole turn (`--resume <it> --fork-session
//! --resume-session-at <its last entry>`), before the message that opened the refused turn, whose content may have come
//! from a tool (a file read, a page) rather than from the message itself. A refusal that follows cuts at the same point.

use super::*;
use crate::model::RefusalCut;

impl AgentRt {
    /// Where its conversation stands now, at the start of a turn: the end of its last whole turn in its session, or, for a
    /// copy (or a cut) whose session is not its own yet, where its next start forks. A session with no whole turn yet (its
    /// first, a compaction just made) has nothing to keep: a new conversation.
    pub(super) fn cut_here(&self) -> RefusalCut {
        match (&self.meta.session_id, &self.meta.last_entry) {
            (Some(of), Some(at)) => RefusalCut {
                of: Some(of.clone()),
                at: Some(at.clone()),
            },
            (Some(_), None) => RefusalCut::default(),
            (None, _) => RefusalCut {
                of: self.meta.fork_of.clone(),
                at: self.meta.fork_at.clone(),
            },
        }
    }

    /// The running turn was refused (`Item::Refused`): whether its end is a refusal, taken once.
    pub(super) fn take_refused(&mut self, interrupted: bool) -> bool {
        std::mem::take(&mut self.refused) && !interrupted
    }

    /// The end of a turn a safeguard refused, in a process of its own (stream-json): the next start leaves it out, from
    /// where a refusal just before was cut if there was one (never further back), else from where this turn began. The
    /// process holds the refused turn in its memory: it stops (`Effects::refused`).
    pub(super) fn cut_after_refusal(&mut self, fx: &mut Effects) {
        let cut = self
            .meta
            .refusal_cut
            .clone()
            .or_else(|| self.turn_cut.take())
            .unwrap_or_default();
        self.meta.session_id = None;
        self.meta.last_entry = None;
        self.meta.fork_at = cut.of.as_ref().and(cut.at.clone());
        self.meta.fork_of = cut.of.clone();
        self.meta.refusal_cut = Some(cut);
        fx.refused = true;
        fx.save = true;
    }

    /// A turn ended well: a refusal after it cuts from its own end, not from an older one.
    pub(super) fn forget_refusal(&mut self, fx: &mut Effects) {
        if self.meta.refusal_cut.take().is_some() {
            fx.save = true;
        }
    }
}
