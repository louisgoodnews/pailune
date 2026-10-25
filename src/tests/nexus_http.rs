//! Nexus flows against a local mock of the API: download, install,
//! update check and rate-limit tracking — no real network.

use super::{fixture, wait_all_jobs};
use crate::config::paths::DataPaths;
use crate::db;
use crate::jobs::pool::JobPool;
use crate::jobs::{JobKind, JobOutput, tasks};
use crate::nexus::download::DownloadJob;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

/// A zip archive with one ASI plugin.
fn mod_archive() -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    zip.start_file(
        "Test Mod/plugin.asi",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"asi bytes").unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

/// Serves canned answers until the test ends; returns the API root.
fn mock_api(archive: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let cdn = format!("{root}/cdn/test.zip");
    let size = archive.len();
    std::thread::spawn(move || {
        listener.incoming().filter_map(|s| s.ok()).for_each(|mut stream| {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap_or_default();
            // Drain the headers (and fail requests without an API key).
            let headers: Vec<String> = std::iter::from_fn(|| {
                let mut line = String::new();
                reader.read_line(&mut line).ok().filter(|_| line.trim() != "").map(|_| line.to_lowercase())
            })
            .collect();
            let keyed = headers.iter().any(|h| h.starts_with("apikey: secret"));
            let path = request.split_whitespace().nth(1).unwrap_or_default().to_string();
            let (status, body): (&str, Vec<u8>) = match path.as_str() {
                _ if path.starts_with("/v1") && !keyed => ("401 Unauthorized", b"{}".to_vec()),
                "/v1/games/crimsondesert/mods/5.json" => ("200 OK", br#"{"mod_id":5,"name":"Test Mod","version":"1.0"}"#.to_vec()),
                "/v1/games/crimsondesert/mods/5/files.json" => ("200 OK", format!(
                    r#"{{"files":[{{"file_id":50,"name":"Main","version":"1.0","file_name":"test.zip","size_kb":1,"size_in_bytes":{size}}}],
                       "file_updates":[{{"old_file_id":50,"new_file_id":51,"new_file_name":"test-1.1.zip"}}]}}"#
                ).into_bytes()),
                "/v1/games/crimsondesert/mods/5/files/50/download_link.json" => ("200 OK", format!(r#"[{{"short_name":"local","URI":"{cdn}"}}]"#).into_bytes()),
                "/cdn/test.zip" => ("200 OK", archive.clone()),
                _ => ("404 Not Found", b"{}".to_vec()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nX-RL-Hourly-Remaining: 77\r\nX-RL-Daily-Remaining: 7777\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&body));
            let _ = reader.read(&mut [0u8; 0]);
        });
    });
    root
}

#[test]
fn downloads_installs_and_finds_updates_through_the_api() {
    let _serial = crate::nexus::limits::TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let fx = fixture();
    let mut config = fx.config.clone();
    config.nexus.api_url = Some(mock_api(mod_archive()));
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    crate::nexus::key::save(&paths.nexus_key, "secret").unwrap();
    let pool = JobPool::spawn(2, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));

    let job = DownloadJob {
        mod_id: 5,
        file_id: 50,
        nxm: None,
        replace: None,
        collection: None,
    };
    let download = pool.submit(JobKind::Download(job), &config);
    let result = wait_all_jobs(&pool, &[download]).remove(0);
    assert!(result.is_ok(), "{result:?}");
    assert!(
        fx.staging.join("Test Mod/plugin.asi").is_file(),
        "wrapper folder hoisted"
    );
    let conn = db::open(&paths.database).unwrap();
    let entry = db::mods::load(&conn)
        .unwrap()
        .mods
        .into_iter()
        .find(|m| m.name == "Test Mod")
        .unwrap();
    let origin = entry.nexus.expect("nexus origin recorded");
    assert_eq!((origin.mod_id, origin.file_id), (5, 50));
    assert_eq!(
        crate::nexus::limits::current().hourly,
        Some(77),
        "rate limit headers recorded"
    );

    let check = pool.submit(JobKind::CheckUpdates, &config);
    match wait_all_jobs(&pool, &[check]).remove(0) {
        Ok(JobOutput::Updates { outdated, failed }) => {
            assert_eq!(
                (outdated, failed),
                (
                    vec![("Test Mod".to_string(), "test-1.1.zip".to_string())],
                    0
                )
            );
        }
        other => panic!("{other:?}"),
    }

    // Without the key the API refuses, and the error says why.
    crate::nexus::key::save(&paths.nexus_key, "wrong").unwrap();
    let denied = pool.submit(JobKind::ValidateKey, &config);
    let err = wait_all_jobs(&pool, &[denied]).remove(0).unwrap_err();
    assert!(err.contains("rejected the API key"), "{err}");
}
