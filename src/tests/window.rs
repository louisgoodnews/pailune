//! Window commands requested through actions: copying the log and quitting.

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
