//! Several calls at once. `futures-util` did this; its hundreds of files make a command line too long for Windows when
//! `escouade-plugin build` compiles it (one `--remap-path-prefix` a source file).

use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::Poll;

/// The answers of `futures`, in their order, all of them polled together.
pub async fn join_all<F: Future>(futures: impl IntoIterator<Item = F>) -> Vec<F::Output> {
    let mut pending: Vec<Pin<Box<F>>> = futures.into_iter().map(Box::pin).collect();
    let mut done: Vec<Option<F::Output>> = pending.iter().map(|_| None).collect();
    poll_fn(|cx| {
        let mut waiting = false;
        for (future, answer) in pending.iter_mut().zip(done.iter_mut()) {
            if answer.is_none() {
                match future.as_mut().poll(cx) {
                    Poll::Ready(value) => *answer = Some(value),
                    Poll::Pending => waiting = true,
                }
            }
        }
        if waiting {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
    done.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn the_answers_keep_their_order_and_the_calls_run_together() {
        let wait = |ms: u64| async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            ms
        };
        let started = std::time::Instant::now();
        assert_eq!(join_all([wait(80), wait(10), wait(40)]).await, [80, 10, 40]);
        assert!(started.elapsed() < Duration::from_millis(125));
    }

    #[tokio::test]
    async fn nothing_to_wait_for_is_an_empty_list() {
        let none: Vec<std::future::Ready<u8>> = Vec::new();
        assert!(join_all(none).await.is_empty());
    }
}
