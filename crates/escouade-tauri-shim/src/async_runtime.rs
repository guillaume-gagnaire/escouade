//! `tauri::async_runtime`, without Tauri: one runtime of Tokio for the whole process, made at its first use (as
//! Tauri's), or the one a program gives it before (`set`: `escouade-server` gives its own).

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use tokio::runtime::{Handle, Runtime};

enum Global {
    Owned(Runtime),
    Given(Handle),
}

static RUNTIME: OnceLock<Global> = OnceLock::new();

fn global() -> &'static Global {
    RUNTIME.get_or_init(|| {
        Global::Owned(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("the async runtime"),
        )
    })
}

fn handle_of(g: &Global) -> Handle {
    match g {
        Global::Owned(r) => r.handle().clone(),
        Global::Given(h) => h.clone(),
    }
}

/// The runtime is `handle`'s, the one a program runs on. Panics when one is already set or made.
pub fn set(handle: Handle) {
    RUNTIME
        .set(Global::Given(handle))
        .unwrap_or_else(|_| panic!("runtime already initialized"))
}

/// A handle of the runtime.
pub fn handle() -> RuntimeHandle {
    RuntimeHandle(handle_of(global()))
}

/// A handle of the runtime, as Tauri's.
#[derive(Clone)]
pub struct RuntimeHandle(Handle);

impl RuntimeHandle {
    pub fn inner(&self) -> &Handle {
        &self.0
    }

    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        JoinHandle(self.0.spawn(task))
    }

    #[track_caller]
    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        JoinHandle(self.0.spawn_blocking(func))
    }

    #[track_caller]
    pub fn block_on<F: Future>(&self, task: F) -> F::Output {
        self.0.block_on(task)
    }
}

/// Runs a future to completion on the runtime.
#[track_caller]
pub fn block_on<F: Future>(task: F) -> F::Output {
    match global() {
        Global::Owned(r) => r.block_on(task),
        Global::Given(h) => h.block_on(task),
    }
}

/// Spawns a future onto the runtime.
#[track_caller]
pub fn spawn<F>(task: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    JoinHandle(match global() {
        Global::Owned(r) => r.spawn(task),
        Global::Given(h) => h.spawn(task),
    })
}

/// Runs `func` on the runtime's threads for blocking work.
#[track_caller]
pub fn spawn_blocking<F, R>(func: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    JoinHandle(match global() {
        Global::Owned(r) => r.spawn_blocking(func),
        Global::Given(h) => h.spawn_blocking(func),
    })
}

/// A task of the runtime: awaited, its output, or the error of a task that panicked or was aborted (Tauri's).
pub struct JoinHandle<T>(tokio::task::JoinHandle<T>);

impl<T> JoinHandle<T> {
    pub fn inner(&self) -> &tokio::task::JoinHandle<T> {
        &self.0
    }

    /// Aborts the task.
    pub fn abort(&self) {
        self.0.abort()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = crate::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(cx).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_and_blocks_on_its_own_runtime() {
        let task = spawn(async { 5 });
        assert_eq!(block_on(task).unwrap(), 5);
        let blocking = spawn_blocking(|| 7);
        assert_eq!(block_on(blocking).unwrap(), 7);
        let aborted = spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        });
        aborted.abort();
        assert!(block_on(aborted).is_err());
    }
}
