//! Restoring a backup into the running application.

use super::RestoreReport;
use crate::config::paths::DataPaths;
use crate::error::{IoContext, PailuneError, Result};
use crate::mod_engine::install::unique_suffix;
use rusqlite::{Connection, MAIN_DB};
use std::fs;
use std::path::Path;

/// Restores `archive`. The caller guarantees nothing is deployed (the
/// restored database must not claim links that do not exist).
///
/// - the database is replaced through SQLite's online backup API, so open
///   connections stay valid; deployment records are cleared;
/// - `config.toml` is replaced after it was validated;
/// - with mod files, the current staging folder is moved aside (kept).
pub fn restore(
    paths: &DataPaths,
    conn: &mut Connection,
    archive: &Path,
    now: i64,
) -> Result<RestoreReport> {
    super::create::read_manifest(archive)?;
    let work = paths.backups.join(format!(".restore-{}", unique_suffix()));
    fs::create_dir_all(&work).at(&work)?;
    let result = (|| {
        zip::ZipArchive::new(fs::File::open(archive).at(archive)?)?.extract(&work)?;
        let database = work.join("pailune.sqlite3");
        if !database.is_file() {
            return Err(PailuneError::Invalid("the backup has no database".into()));
        }
        conn.restore(MAIN_DB, &database, None::<fn(rusqlite::backup::Progress)>)?;
        crate::db::schema_upgrade(conn)?;
        conn.execute_batch("DELETE FROM links; DELETE FROM created_dirs;")?;
        crate::db::mods::set_deployed_revision(conn, None)?;
        let config = work.join("config.toml");
        let settings = config.is_file();
        if settings {
            toml::from_str::<crate::config::Config>(&fs::read_to_string(&config).at(&config)?)?;
            fs::copy(&config, &paths.config_file).at(&paths.config_file)?;
        }
        let staged = work.join("staging");
        let previous_staging = match staged.is_dir() {
            false => None,
            true => {
                let aside = paths
                    .staging
                    .with_file_name(format!("staging-before-restore-{now}"));
                fs::rename(&paths.staging, &aside).at(&paths.staging)?;
                fs::rename(&staged, &paths.staging).at(&paths.staging)?;
                Some(aside)
            }
        };
        Ok(RestoreReport {
            settings,
            mod_files: previous_staging.is_some(),
            previous_staging,
        })
    })();
    let _ = fs::remove_dir_all(&work);
    result
}
