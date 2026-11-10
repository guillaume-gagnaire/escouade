//! The temperature of its cache (ESC-398, `crate::heat`): warmed by each request that read or wrote it, cold when it
//! breaks.

use super::*;
use crate::heat::{self, CacheHeat, Warming};

impl AgentRt {
    /// The turn that warms its cache, begun at its first request.
    fn warming(&mut self) -> &mut Warming {
        let before = self.meta.heat.as_ref();
        self.warming.get_or_insert_with(|| Warming::open(before))
    }

    /// A request of Claude Code's own (not a subagent's), its API `usage` as told: the cache served now.
    pub(super) fn heat_message(&mut self, model: &str, usage: &Value, fx: &mut Effects) {
        let served = heat::claude_served(model, usage);
        if served.cached == 0 {
            return;
        }
        let before = self.meta.heat.as_ref().and_then(|h| h.ttl);
        let w = self.warming();
        w.by_message = true;
        w.ttl = Some(heat::claude_ttl(usage, w.ttl.or(before)));
        w.model = model.to_string();
        w.add(served);
        self.heat_save(now_ms(), fx);
    }

    /// The end of a turn its harness tells by its tokens (`TurnUsage`): the cache served then, when it cached something.
    /// A compaction asked by `/compact` ends that way too: what it cached is not what the next turn reads.
    pub(super) fn heat_turn(&mut self, model: &str, fresh: u64, cache: u64, fx: &mut Effects) {
        let compacting = std::mem::take(&mut self.heat_compacting);
        let by_message = self.warming.as_ref().is_some_and(|w| w.by_message);
        if !by_message && !compacting && cache > 0 {
            let ttl = heat::vendor_ttl(model);
            let w = self.warming();
            w.ttl = ttl;
            w.model = model.to_string();
            w.add(heat::turn_served(model, fresh, cache));
            self.heat_save(now_ms(), fx);
        }
        self.warming = None;
    }

    /// The end of a turn, whatever told it.
    pub(super) fn heat_close(&mut self) {
        self.warming = None;
    }

    /// The cache broke (`heat::broken::*`): the next turn reads everything again.
    pub(crate) fn heat_break(&mut self, why: &str, fx: &mut Effects) {
        self.warming = None;
        if let Some(h) = self.meta.heat.as_mut() {
            if h.broken.is_none() {
                h.broken = Some(why.to_string());
                fx.agent_changed = true;
                fx.save = true;
            }
        }
    }

    /// A compaction asked of its harness (`/compact`, ESC-205): cold, and the turn that ends it does not warm it.
    pub(crate) fn heat_compacts(&mut self, fx: &mut Effects) {
        self.heat_compacting = true;
        self.heat_break(heat::broken::COMPACTION, fx);
    }

    fn heat_save(&mut self, at: i64, fx: &mut Effects) {
        let Some(w) = self.warming.as_ref() else {
            return;
        };
        let context = if self.context_tokens > 0 {
            self.context_tokens
        } else {
            w.served.cached + w.served.fresh
        };
        self.meta.heat = Some(CacheHeat::warmed(&self.meta, w, context, at));
        fx.agent_changed = true;
        fx.save = true;
    }

    /// As the window shows it (`CacheHeat::seen`).
    pub(super) fn heat_view(&self) -> Option<CacheHeat> {
        self.meta.heat.as_ref().map(|h| h.seen(&self.meta))
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
