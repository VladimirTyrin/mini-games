use std::sync::Arc;

use tokio::sync::{Semaphore, mpsc};

type PushJob = Box<dyn FnOnce() + Send>;

/// Runs push sends (blocking HTTP) a few at a time. The blocking pool is shared with the database,
/// bot searches and Google sign-ins, so a slow push service or a burst of notifications must not
/// fill it: when the queue is full, new sends are dropped and logged.
#[derive(Clone, Debug)]
pub(crate) struct PushQueue {
    sender: mpsc::Sender<PushJob>,
}

impl PushQueue {
    const CAPACITY: usize = 256;
    const CONCURRENT_SENDS: usize = 2;

    /// Starts the worker on the current runtime.
    pub(crate) fn start() -> Self {
        let (sender, mut jobs) = mpsc::channel::<PushJob>(Self::CAPACITY);
        tokio::spawn(async move {
            let permits = Arc::new(Semaphore::new(Self::CONCURRENT_SENDS));
            while let Some(job) = jobs.recv().await {
                let Ok(permit) = permits.clone().acquire_owned().await else {
                    return;
                };
                tokio::task::spawn_blocking(move || {
                    job();
                    drop(permit);
                });
            }
        });
        Self { sender }
    }

    pub(crate) fn submit(&self, job: impl FnOnce() + Send + 'static) {
        if self.sender.try_send(Box::new(job)).is_err() {
            tracing::warn!("push queue full: a notification was dropped");
        }
    }
}
