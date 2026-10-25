//! A fixed-size worker pool plus one dedicated lane for game-directory jobs.
//!
//! Exclusive jobs get their own single thread and queue, so they run in the
//! order they were submitted (FIFO) and never occupy the shared workers that
//! run downloads, imports and analyses.

use super::context::JobContext;
use super::{Access, JobEvent, JobId, JobKind, JobOutput, JobUpdate};
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::error::Result;
use crate::logging::{Event, EventKind, Severity};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

/// Executes a job; injectable for tests.
pub type Runner = Arc<dyn Fn(&JobContext, JobKind) -> Result<JobOutput> + Send + Sync>;

/// A submitted job waiting for a worker.
struct Queued {
    id: JobId,
    kind: JobKind,
    config: Config,
}

/// Handle owned by the UI.
pub struct JobPool {
    queue: Sender<Queued>,
    exclusive: Sender<Queued>,
    events: Receiver<JobEvent>,
    next_id: AtomicU64,
}

/// State shared by all worker threads.
struct Shared {
    queue: Mutex<Receiver<Queued>>,
    events: Sender<JobEvent>,
    paths: DataPaths,
    runner: Runner,
    notify: Arc<dyn Fn() + Send + Sync>,
    game_lock: Arc<Mutex<()>>,
}

impl JobPool {
    /// Starts `workers` threads; `notify` is called after every event.
    pub fn spawn(
        workers: usize,
        paths: DataPaths,
        runner: Runner,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let (queue_tx, queue_rx) = channel();
        let (exclusive_tx, exclusive_rx) = channel();
        let (events_tx, events_rx) = channel();
        let shared = Arc::new(Shared {
            queue: Mutex::new(queue_rx),
            events: events_tx,
            paths,
            runner,
            notify,
            game_lock: Arc::new(Mutex::new(())),
        });
        (0..workers.max(1)).for_each(|n| {
            let shared = Arc::clone(&shared);
            let _ = thread::Builder::new()
                .name(format!("pylune-job-{n}"))
                .spawn(move || work(&shared, &shared.queue));
        });
        let lane = Arc::clone(&shared);
        let exclusive_rx = Mutex::new(exclusive_rx);
        let _ = thread::Builder::new()
            .name("pylune-game".into())
            .spawn(move || work(&lane, &exclusive_rx));
        Self {
            queue: queue_tx,
            exclusive: exclusive_tx,
            events: events_rx,
            next_id: AtomicU64::new(1),
        }
    }

    /// Queues a job with a snapshot of the configuration; returns its id.
    pub fn submit(&self, kind: JobKind, config: &Config) -> JobId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let lane = match kind.access() {
            Access::Exclusive => &self.exclusive,
            Access::Shared => &self.queue,
        };
        let _ = lane.send(Queued {
            id,
            kind,
            config: config.clone(),
        });
        id
    }

    /// Drains every event that has arrived so far.
    pub fn poll(&self) -> Vec<JobEvent> {
        std::iter::from_fn(|| self.events.try_recv().ok()).collect()
    }
}

/// Worker loop: take the next job from `queue` until the pool is dropped.
fn work(shared: &Shared, queue: &Mutex<Receiver<Queued>>) {
    // The receiver lock is released as soon as one job has been taken.
    let next = || queue.lock().ok().and_then(|rx| rx.recv().ok());
    std::iter::from_fn(next).for_each(|job| run_one(shared, job));
}

/// Runs one job: reports start, executes it (under the game lock if
/// exclusive), journals failures and reports the result.
fn run_one(shared: &Shared, job: Queued) {
    let ctx = JobContext {
        id: job.id,
        paths: shared.paths.clone(),
        config: job.config,
        events: shared.events.clone(),
        notify: Arc::clone(&shared.notify),
        game_lock: Arc::clone(&shared.game_lock),
    };
    ctx.send(JobUpdate::Started);
    let title = job.kind.title();
    tracing::debug!(job = job.id, %title, "job started");
    let outcome = catch_unwind(AssertUnwindSafe(|| match job.kind.access() {
        Access::Exclusive => {
            let _guard = ctx.lock_game();
            (shared.runner)(&ctx, job.kind)
        }
        Access::Shared => (shared.runner)(&ctx, job.kind),
    }));
    let result = match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err(format!("{title} crashed — see the log file")),
    };
    if let Err(err) = &result {
        // Failures are journaled so they show up in the Log tab as well.
        let event = Event::new(
            Severity::Error,
            EventKind::Job,
            None,
            format!("{title} failed: {err}"),
        );
        match ctx.db() {
            Ok(conn) => ctx.journal(&conn, event),
            Err(db_err) => tracing::error!(job = job.id, %title, %err, %db_err, "job failed"),
        }
    }
    ctx.send(JobUpdate::Finished(result));
}
