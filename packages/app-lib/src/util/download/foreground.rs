//! Admission for background media while foreground requests are active.

use std::sync::LazyLock;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

#[cfg(not(test))]
const IDLE_GRACE: Duration = Duration::from_millis(500);
#[cfg(test)]
const IDLE_GRACE: Duration = Duration::from_millis(10);

pub(crate) static FOREGROUND_REQUESTS: LazyLock<ForegroundRequests> =
    LazyLock::new(ForegroundRequests::default);

#[derive(Clone, Copy)]
struct RequestStatus {
    active: usize,
    quiet_since: Instant,
}

pub(crate) struct ForegroundRequests {
    status: watch::Sender<RequestStatus>,
}

impl Default for ForegroundRequests {
    fn default() -> Self {
        Self {
            status: watch::channel(RequestStatus {
                active: 0,
                quiet_since: Instant::now(),
            })
            .0,
        }
    }
}

impl ForegroundRequests {
    pub(crate) fn start(&self) -> RequestGuard<'_> {
        self.status.send_modify(|status| status.active += 1);
        RequestGuard { requests: self }
    }

    pub(crate) async fn wait_for_idle(&self) {
        let mut changes = self.status.subscribe();
        loop {
            let status = *changes.borrow_and_update();
            if status.active == 0 {
                tokio::select! {
                    _ = tokio::time::sleep_until(status.quiet_since + IDLE_GRACE) => return,
                    _ = changes.changed() => {}
                }
            } else {
                let _ = changes.changed().await;
            }
        }
    }
}

pub(crate) struct RequestGuard<'a> {
    requests: &'a ForegroundRequests,
}

impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        self.requests.status.send_modify(|status| {
            status.active -= 1;
            if status.active == 0 {
                status.quiet_since = Instant::now();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn background_waits_until_every_foreground_request_finishes() {
        let requests = ForegroundRequests::default();
        let first = requests.start();
        let second = requests.start();
        let idle = requests.wait_for_idle();
        tokio::pin!(idle);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut idle)
                .await
                .is_err()
        );
        drop(first);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut idle)
                .await
                .is_err()
        );
        drop(second);
        tokio::time::timeout(Duration::from_secs(1), idle)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn new_requests_preempt_background_transfers_between_reads() {
        let requests = ForegroundRequests::default();
        requests.wait_for_idle().await;
        let request = requests.start();
        let next_read = requests.wait_for_idle();
        tokio::pin!(next_read);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut next_read)
                .await
                .is_err()
        );
        drop(request);
        tokio::time::timeout(Duration::from_secs(1), next_read)
            .await
            .unwrap();
    }
}
