//! Start-up helpers: the repaint ticker and the first status line.

use super::{Level, StatusLine};
use crate::backup::snapshot;
use crate::config::detect::Discovery;
use crate::steam::{GameStatus, SteamInfo, locate};
use eframe::egui;
use std::time::Duration;

/// Wakes the app once per second, even while its window is hidden or
/// occluded, so the game process and the nxm queue keep being polled.
pub fn spawn_ticker(ctx: egui::Context) {
    std::thread::spawn(move || {
        std::iter::repeat_with(|| std::thread::sleep(Duration::from_secs(1)))
            .for_each(|()| ctx.request_repaint());
    });
}

/// Takes the first-run `.bak` snapshot when the game is present.
pub fn initial_status(info: &SteamInfo, discovery: &Discovery) -> StatusLine {
    let ready = locate::game_status(info) == GameStatus::Ready;
    match (ready.then(|| snapshot::ensure_meta_backup(info)), discovery) {
        (None, _) => StatusLine::new(
            Level::Warning,
            "Choose the Crimson Desert folder to get started",
        ),
        (Some(Ok(_)), Discovery::Detected(dir)) => StatusLine::new(
            Level::Success,
            format!("Found Crimson Desert via Steam: {}", dir.display()),
        ),
        (Some(Ok(0)), _) => StatusLine::new(Level::Info, "Ready"),
        (Some(Ok(n)), _) => StatusLine::new(
            Level::Success,
            format!("Backed up {n} metadata file(s) to .bak"),
        ),
        (Some(Err(err)), _) => StatusLine::new(Level::Error, format!("Backup failed: {err}")),
    }
}
