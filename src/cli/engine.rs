//! Running changing operations headlessly: a `PyluneApp` without a window,
//! driven through the same actions and jobs as the buttons, so validation
//! and safety rules are identical.

use super::ops::{Op, Report};
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::error::{PyluneError, Result};
use crate::instance::{self, InstanceLock};
use crate::mod_engine::model::MoveTo;
use crate::modlist;
use crate::ui::StatusLine;
use crate::ui::{Action, BackupAction, DeployAction, JobState, Level, ModAction, NexusAction};
use crate::ui::{ProfileAction, PyluneApp};
use std::time::{Duration, Instant};

/// Longest wait for background jobs of one operation.
const JOB_TIMEOUT: Duration = Duration::from_secs(3600);

/// A headless application holding the instance lock.
pub struct Engine {
    app: PyluneApp,
    _lock: InstanceLock,
}

impl Engine {
    /// Takes the instance lock and starts the application without a window.
    pub fn open(paths: DataPaths, config: Config) -> Result<Self> {
        let lock = instance::try_lock(&paths)?.ok_or_else(|| {
            PyluneError::Invalid(
                "the Pylune window is open — make changes there, or close it first".into(),
            )
        })?;
        let conn = crate::db::open(&paths.database)?;
        let ctx = eframe::egui::Context::default();
        let mut app = PyluneApp::build(ctx, paths, config, conn, None);
        app.guide.open = false;
        app.headless = true;
        settle(&mut app);
        Ok(Self { app, _lock: lock })
    }

    /// Runs one changing operation to completion, including its jobs.
    pub fn run(&mut self, op: &Op) -> Result<Report> {
        let app = &mut self.app;
        let known = |name: &str| app.registry.mods.iter().any(|m| m.name == name);
        let unknown: Vec<&String> = match op {
            Op::SetEnabled { names, .. } => names.iter().filter(|n| !known(n)).collect(),
            Op::Move { name, .. } | Op::SetVariant { name, .. } | Op::Delete { name } => {
                std::iter::once(name).filter(|n| !known(n)).collect()
            }
            _ => Vec::new(),
        };
        if let Some(name) = unknown.first() {
            return Err(PyluneError::Invalid(format!("no mod named '{name}'")));
        }
        match op {
            Op::ExportList { path } => return export(app, path),
            Op::ImportList { path } => return import(app, path),
            _ => {}
        }
        app.process = crate::launch::run::game_process();
        app.status = StatusLine::new(Level::Info, "");
        app.job_failures.clear();
        actions(op).into_iter().for_each(|a| app.dispatch(a));
        settle(app);
        let failures = std::mem::take(&mut app.job_failures);
        match (
            app.status.level,
            failures.is_empty(),
            app.status.text.is_empty(),
        ) {
            (Level::Error, _, _) => Err(PyluneError::Invalid(app.status.text.clone())),
            (_, false, _) => Err(PyluneError::Invalid(failures.join("; "))),
            (_, true, true) => Ok(Report::text("nothing to do")),
            _ => Ok(Report::text(app.status.text.clone())),
        }
    }
}

/// The UI actions an operation corresponds to.
fn actions(op: &Op) -> Vec<Action> {
    let m = Action::Mod;
    match op.clone() {
        Op::SetEnabled { names, enabled } => names
            .into_iter()
            .map(|n| m(ModAction::SetEnabled(n, enabled)))
            .collect(),
        Op::SetAll(enabled) => vec![m(ModAction::SetAll(enabled))],
        Op::Move { name, position } => vec![m(ModAction::Move(name, MoveTo::Position(position)))],
        Op::SetVariant { name, variant } => vec![m(ModAction::SetVariant(name, variant))],
        Op::Install { paths } => vec![m(ModAction::ImportPaths(paths))],
        Op::Delete { name } => vec![m(ModAction::Delete(name))],
        Op::Apply => vec![Action::Deploy(DeployAction::Apply)],
        Op::Unlink => vec![Action::Deploy(DeployAction::UnlinkAll)],
        Op::CleanSlate => vec![Action::Deploy(DeployAction::CleanSlate)],
        Op::Launch => vec![Action::Deploy(DeployAction::Launch)],
        Op::ProfileSave { name } => vec![Action::Profile(ProfileAction::Save(name))],
        Op::ProfileLoad { name } => vec![Action::Profile(ProfileAction::Load(name))],
        Op::Backup { full } => vec![Action::Backup(BackupAction::Create { full })],
        Op::RestoreBackup { path } => vec![Action::Backup(BackupAction::Restore(path))],
        Op::BackupSaves => vec![Action::Backup(BackupAction::BackUpSaves)],
        Op::CheckUpdates => vec![Action::Nexus(NexusAction::CheckUpdates)],
        _ => Vec::new(),
    }
}

/// Polls until no job is queued or running (or the timeout passes).
fn settle(app: &mut PyluneApp) {
    let start = Instant::now();
    loop {
        app.poll();
        let busy = app
            .jobs
            .iter()
            .any(|j| matches!(j.state, JobState::Queued | JobState::Running));
        if !busy || start.elapsed() > JOB_TIMEOUT {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Writes the mod list to `path`.
fn export(app: &PyluneApp, path: &std::path::Path) -> Result<Report> {
    let list = modlist::capture(
        &app.registry,
        &app.nexus.collections,
        &app.config.nexus.domain,
        crate::db::now(),
    );
    modlist::write(path, &list)?;
    Ok(Report::text(format!(
        "Exported {} mod(s) to {}",
        list.mods.len(),
        path.display()
    )))
}

/// Applies a mod list; missing mods are reported in the data.
fn import(app: &mut PyluneApp, path: &std::path::Path) -> Result<Report> {
    let list = modlist::read(path, &app.config.nexus.domain)?;
    let line = app.apply_list(list)?;
    let missing: Vec<serde_json::Value> = app
        .mods
        .list_import
        .iter()
        .flat_map(|i| i.missing.iter())
        .map(|m| serde_json::json!({"name": m.name, "source": m.source, "nexus": m.nexus.as_ref().map(|n| serde_json::json!({"mod_id": n.mod_id, "file_id": n.file_id}))}))
        .collect();
    Ok(Report {
        text: line.text,
        data: serde_json::json!({ "missing": missing }),
    })
}
