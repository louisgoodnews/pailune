//! Window-level features driven through actions: copying the log, quitting,
//! notifications.

use super::fixture;
use super::ui::{app, settle};
use crate::ui::{Action, ModAction};

#[test]
fn copy_log_and_quit_are_requested_for_the_window() {
    let fx = fixture();
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Cool Mod"),
    ])));
    settle(&mut app);
    app.dispatch(Action::RefreshLog);
    assert!(!app.log.events.is_empty());
    app.dispatch(Action::CopyLog);
    let copied = app
        .window
        .clipboard
        .clone()
        .expect("text for the clipboard");
    assert_eq!(copied.lines().count(), app.log.events.len());
    assert!(copied.contains("Cool Mod: installed from"), "{copied}");
    assert!(app.status.text.starts_with("Copied"), "{:?}", app.status);
    assert!(!app.window.quit);
    app.dispatch(Action::Quit);
    assert!(app.window.quit, "the frame closes the window");
}

#[test]
fn jobs_and_scans_notify_and_the_center_marks_them_read() {
    use crate::ui::Level;
    use crate::ui::notify::NoticeAction;
    let fx = fixture();
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Cool Mod"),
        fx.sources.join("Does Not Exist"),
    ])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::Rescan));
    let titles: Vec<(&str, Level)> = app
        .notices
        .items
        .iter()
        .map(|n| (n.title.as_str(), n.level))
        .collect();
    assert!(
        titles.contains(&("Mod installed", Level::Success)),
        "{titles:?}"
    );
    assert!(
        titles.contains(&("Something failed", Level::Error)),
        "{titles:?}"
    );
    assert_eq!(titles.last(), Some(&("Scan complete", Level::Info)));
    assert!(
        !titles.iter().any(|(t, _)| t.contains("onflict")),
        "routine analysis stays quiet"
    );
    let installed = app
        .notices
        .items
        .iter()
        .find(|n| n.title == "Mod installed")
        .unwrap();
    assert!(installed.text.contains("Cool Mod"), "{}", installed.text);
    assert_eq!(app.notices.unread(), titles.len());
    app.dispatch(Action::Notice(NoticeAction::ToggleCenter));
    assert_eq!(app.notices.unread(), 0, "opening the center reads them");
    assert!(app.notices.toasts(std::time::Instant::now()).is_empty());
    app.dispatch(Action::Notice(NoticeAction::Clear));
    assert!(app.notices.items.is_empty());
}
