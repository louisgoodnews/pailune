//! Background thread that performs Nexus requests off the UI thread.

use super::api::Client;
use super::download::{self, DownloadJob, Installed};
use super::{NexusFile, NexusMod, NexusRef, NexusUser};
use crate::error::Result;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

/// Work the UI can ask for.
#[derive(Debug, Clone)]
pub enum Request {
    /// Replace the credentials used for following requests.
    Configure {
        key: String,
        domain: String,
    },
    Validate,
    FetchMod(u64),
    Download(DownloadJob),
    /// Compare staged mods (name, origin) with their current Nexus version.
    CheckUpdates(Vec<(String, NexusRef)>),
}

/// Results delivered back to the UI.
#[derive(Debug, Clone)]
pub enum Response {
    User(NexusUser),
    Mod(NexusMod, Vec<NexusFile>),
    Progress {
        file: String,
        done: u64,
        total: Option<u64>,
    },
    Installed(Installed),
    /// `(mod name, newer version)` for every outdated mod.
    Updates(Vec<(String, String)>),
    Failed(String),
}

/// Channels connecting the UI to the worker thread.
pub struct WorkerHandle {
    pub requests: Sender<Request>,
    pub responses: Receiver<Response>,
}

/// Static context the worker needs for downloads.
struct Context {
    staging: PathBuf,
    downloads: PathBuf,
    notify: Box<dyn Fn() + Send>,
}

/// Starts the worker; `notify` is called after every response (e.g. repaint).
pub fn spawn(staging: PathBuf, downloads: PathBuf, notify: Box<dyn Fn() + Send>) -> WorkerHandle {
    let (req_tx, req_rx) = channel::<Request>();
    let (res_tx, res_rx) = channel::<Response>();
    let ctx = Context {
        staging,
        downloads,
        notify,
    };
    thread::spawn(move || {
        // `fold` threads the current client through the request stream.
        req_rx.iter().fold(None::<Client>, |client, request| {
            handle(&ctx, &res_tx, client, request)
        });
    });
    WorkerHandle {
        requests: req_tx,
        responses: res_rx,
    }
}

/// Sends a response and pokes the UI.
fn emit(ctx: &Context, tx: &Sender<Response>, response: Response) {
    let _ = tx.send(response);
    (ctx.notify)();
}

/// Processes one request, returning the (possibly new) client.
fn handle(
    ctx: &Context,
    tx: &Sender<Response>,
    client: Option<Client>,
    request: Request,
) -> Option<Client> {
    let result: Result<Response> = match (&client, request) {
        (_, Request::Configure { key, domain }) => {
            return (!key.trim().is_empty()).then(|| Client::new(&key, &domain));
        }
        (None, _) => Ok(Response::Failed("set a Nexus API key first".into())),
        (Some(c), Request::Validate) => c.validate().map(Response::User),
        (Some(c), Request::FetchMod(id)) => c
            .mod_info(id)
            .and_then(|info| c.files(id).map(|list| Response::Mod(info, list.files))),
        (Some(c), Request::Download(job)) => {
            let progress = |file: &str, done: u64, total: Option<u64>| {
                emit(
                    ctx,
                    tx,
                    Response::Progress {
                        file: file.to_string(),
                        done,
                        total,
                    },
                );
            };
            download::run(c, &job, &ctx.staging, &ctx.downloads, &progress).map(Response::Installed)
        }
        (Some(c), Request::CheckUpdates(refs)) => refs
            .into_iter()
            .map(|(name, r)| {
                c.mod_info(r.mod_id)
                    .map(|info| (name, r.version, info.version))
            })
            .collect::<Result<Vec<_>>>()
            .map(|all| {
                all.into_iter()
                    .filter(|(_, have, latest)| !latest.is_empty() && have != latest)
                    .map(|(name, _, latest)| (name, latest))
                    .collect()
            })
            .map(Response::Updates),
    };
    match result {
        Ok(response) => emit(ctx, tx, response),
        Err(err) => emit(ctx, tx, Response::Failed(err.to_string())),
    }
    client
}

impl WorkerHandle {
    /// Queues a request; a dead worker is reported as a failure response.
    pub fn send(&self, request: Request) -> bool {
        self.requests.send(request).is_ok()
    }

    /// Drains every response that has arrived so far.
    pub fn poll(&self) -> Vec<Response> {
        std::iter::from_fn(|| self.responses.try_recv().ok()).collect()
    }
}
