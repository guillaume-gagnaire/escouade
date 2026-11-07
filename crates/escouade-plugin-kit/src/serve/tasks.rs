//! The tasks of a module, on its one thread (docs/PLUGINS.md §8.3): what a turn polls, for the kit's two ways of serving
//! (`reactor`, `crate::host`).
//!
//! A turn polls every task once: most wait for a line, and what they wait for was set by the turn itself (an answer, a
//! timer that rang). Then it polls again those that were woken meanwhile, so that a task that waits for what a later one
//! sets goes on in the same turn: nothing else would bring it back when no line follows.

use parking_lot::Mutex;
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Wake, Waker};

/// A task to its end.
pub(crate) type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

/// How many times a turn goes over the woken tasks. A chain that long is a task that wakes itself, or two that wake each
/// other: a turn has a limit of its own, past which the host kills the module, so the rest goes to the next turn.
const PASSES: usize = 128;

/// Those to poll again.
type Woken = Arc<Mutex<HashSet<u64>>>;

struct Wakes {
    id: u64,
    woken: Woken,
}

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.woken.lock().insert(self.id);
    }
}

struct Running {
    id: u64,
    task: Task,
    waker: Waker,
}

impl Running {
    /// Whether it still has work.
    fn poll(&mut self) -> bool {
        let mut cx = Context::from_waker(&self.waker);
        self.task.as_mut().poll(&mut cx).is_pending()
    }
}

/// The tasks of a module.
#[derive(Default)]
pub(crate) struct Tasks {
    running: Mutex<Vec<Running>>,
    /// Those started while the others are polled.
    born: Mutex<Vec<Running>>,
    woken: Woken,
    next: AtomicU64,
    /// Something changed that any task may wait for.
    stirred: AtomicBool,
}

impl Tasks {
    /// Starts `task`: it is polled in this turn.
    pub(crate) fn spawn(&self, task: Task) {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let woken = self.woken.clone();
        let waker = Waker::from(Arc::new(Wakes { id, woken }));
        self.woken.lock().insert(id);
        self.born.lock().push(Running { id, task, waker });
    }

    /// What a task waits for changed and nothing wakes it: every task is polled once more in this turn.
    pub(crate) fn stir(&self) {
        self.stirred.store(true, Ordering::Release);
    }

    /// One turn. True when tasks are still awake at the end of it: only a turn more will poll them, so whoever calls this
    /// has to bring one.
    #[must_use]
    pub(crate) fn run(&self) -> bool {
        let mut running = self.running.lock();
        for pass in 0..PASSES {
            let woken = std::mem::take(&mut *self.woken.lock());
            running.append(&mut self.born.lock());
            let all = self.stirred.swap(false, Ordering::AcqRel) || pass == 0;
            running.retain_mut(|r| !(all || woken.contains(&r.id)) || r.poll());
            if self.woken.lock().is_empty() && !self.stirred.load(Ordering::Acquire) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;
    use std::task::Poll;

    /// A task that counts its polls, ends at `end` and wakes itself until then.
    fn busy(polls: Arc<AtomicU64>, end: u64) -> Task {
        Box::pin(poll_fn(move |task| {
            if polls.fetch_add(1, Ordering::Relaxed) + 1 == end {
                return Poll::Ready(());
            }
            task.waker().wake_by_ref();
            Poll::Pending
        }))
    }

    #[test]
    fn a_task_that_wakes_itself_does_not_hold_the_turn_for_ever() {
        let tasks = Tasks::default();
        let polls = Arc::new(AtomicU64::new(0));
        tasks.spawn(busy(polls.clone(), u64::MAX));
        assert!(tasks.run(), "it asks for another turn");
        assert_eq!(polls.load(Ordering::Relaxed), PASSES as u64);
        assert!(tasks.run(), "and goes on at that turn");
        assert_eq!(polls.load(Ordering::Relaxed), 2 * PASSES as u64);
    }

    #[test]
    fn a_task_that_is_not_woken_is_polled_once_a_turn() {
        let tasks = Tasks::default();
        let polls = Arc::new(AtomicU64::new(0));
        let counted = polls.clone();
        tasks.spawn(Box::pin(poll_fn(move |_| {
            counted.fetch_add(1, Ordering::Relaxed);
            Poll::Pending
        })));
        assert!(!tasks.run());
        assert!(!tasks.run());
        assert_eq!(polls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_task_that_wakes_itself_until_it_ends_ends_in_its_turn() {
        let tasks = Tasks::default();
        let polls = Arc::new(AtomicU64::new(0));
        tasks.spawn(busy(polls.clone(), 3));
        assert!(!tasks.run(), "it ended in this turn");
        assert_eq!(polls.load(Ordering::Relaxed), 3);
        assert!(tasks.running.lock().is_empty());
    }

    #[test]
    fn a_task_born_in_a_turn_is_polled_in_it() {
        let tasks = Arc::new(Tasks::default());
        let polls = Arc::new(AtomicU64::new(0));
        let (t, p) = (tasks.clone(), polls.clone());
        tasks.spawn(Box::pin(async move {
            t.spawn(busy(p, 1));
        }));
        assert!(!tasks.run());
        assert_eq!(polls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_stir_polls_them_all_again() {
        let tasks = Arc::new(Tasks::default());
        let polls = Arc::new(AtomicU64::new(0));
        let counted = polls.clone();
        tasks.spawn(Box::pin(poll_fn(move |_| {
            counted.fetch_add(1, Ordering::Relaxed);
            Poll::<()>::Pending
        })));
        let stirs = tasks.clone();
        tasks.spawn(Box::pin(async move { stirs.stir() }));
        assert!(!tasks.run());
        assert_eq!(polls.load(Ordering::Relaxed), 2);
    }
}
