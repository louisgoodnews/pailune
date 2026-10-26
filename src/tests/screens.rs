//! Renders key screens to PNG for visual review (ignored by default; needs a
//! GPU adapter). `PYLUNE_SHOTS=<dir> cargo test screens -- --ignored`.

use super::{Fixture, fixture};
use crate::config::paths::DataPaths;
use crate::config::prefs::Theme;
use crate::ui::guide::{GuideAction, GuidePage};
use crate::ui::{Action, ModAction, PyluneApp, SettingsPage, Tab};
use egui_kittest::Harness;
use std::path::PathBuf;

/// A JSON mod like the first in-game test.
const ARROWS: &str = r#"{"modinfo": {"title": "Arrow Stacks"}, "patches": [{
  "game_file": "gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody", "changes": [
    {"entry": "Arrow", "rel_offset": 1, "original": "6400000000000000",
     "patched": "e703000000000000", "label": "Arrow max stack 100 -> 999"},
    {"entry": "Pyeonjeon_Arrow", "rel_offset": 1, "original": "6400000000000000",
     "patched": "e703000000000000", "label": "Pyeonjeon_Arrow max stack 100 -> 999"}]}]}"#;

fn harness(fx: &Fixture, theme: Theme) -> Harness<'static, PyluneApp> {
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let mut config = fx.config.clone();
    config.ui.theme = theme;
    Harness::builder()
        .with_size([1200.0, 760.0])
        .with_theme(match theme {
            Theme::Light => eframe::egui::Theme::Light,
            _ => eframe::egui::Theme::Dark,
        })
        .wgpu()
        .build_eframe(move |cc| {
            let conn = crate::db::open(&paths.database).unwrap();
            PyluneApp::new(cc, paths, config, conn, None)
        })
}

fn shoot(harness: &mut Harness<'static, PyluneApp>, name: &str) {
    let dir = std::env::var_os("PYLUNE_SHOTS")
        .map_or_else(|| PathBuf::from("target/screens"), PathBuf::from);
    std::fs::create_dir_all(&dir).unwrap();
    harness.run_steps(4);
    let image = harness.render().expect("render");
    image.save(dir.join(format!("{name}.png"))).unwrap();
}

#[test]
#[ignore = "renders with the GPU; run on demand"]
fn screens() {
    [(Theme::Dark, "dark"), (Theme::Light, "light")]
        .into_iter()
        .for_each(|(theme, tag)| {
            let fx = fixture();
            let mut h = harness(&fx, theme);
            shoot(&mut h, &format!("{tag}-first-start"));
            let app = h.state_mut();
            app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
                fx.sources.join("Cool Mod"),
                fx.sources.join("Overlay"),
            ])));
            std::thread::sleep(std::time::Duration::from_millis(500));
            app.poll();
            app.dispatch(Action::Guide(GuideAction::Open(GuidePage::Topics)));
            app.guide.query = "undo".into();
            shoot(&mut h, &format!("{tag}-mods-guide"));
            let app = h.state_mut();
            app.dispatch(Action::Guide(GuideAction::Close));
            app.dispatch(Action::SwitchTab(Tab::Settings));
            shoot(&mut h, &format!("{tag}-settings-appearance"));
            h.state_mut().draft.page = SettingsPage::Backups;
            shoot(&mut h, &format!("{tag}-settings-backups"));
            let pages = [
                ("details", Tab::Mods),
                ("profiles", Tab::Profiles),
                ("nexus", Tab::Nexus),
                ("log", Tab::Log),
            ];
            h.state_mut()
                .dispatch(Action::Mod(ModAction::SetEnabled("Cool Mod".into(), true)));
            pages.into_iter().for_each(|(name, tab)| {
                let app = h.state_mut();
                app.dispatch(Action::SwitchTab(tab));
                app.dispatch(Action::Mod(ModAction::Select("Cool Mod".into())));
                shoot(&mut h, &format!("{tag}-{name}"));
            });
            let fomod = fx.sources.join("Better Things");
            crate::fomod::tests::write_mod(&fomod);
            let app = h.state_mut();
            app.dispatch(Action::SwitchTab(Tab::Mods));
            app.dispatch(Action::Mod(ModAction::ImportPaths(vec![fomod])));
            std::thread::sleep(std::time::Duration::from_millis(500));
            app.poll();
            app.dispatch(Action::Mod(ModAction::Select("Better Things".into())));
            shoot(&mut h, &format!("{tag}-installer"));
            let json = fx.sources.join("Arrow Stacks");
            std::fs::create_dir_all(&json).unwrap();
            std::fs::write(json.join("arrow_stacks.json"), ARROWS).unwrap();
            let app = h.state_mut();
            app.dispatch(Action::Mod(ModAction::ImportPaths(vec![json])));
            std::thread::sleep(std::time::Duration::from_millis(500));
            app.poll();
            app.dispatch(Action::Mod(ModAction::Select("Arrow Stacks".into())));
            let stacks = fx.sources.join("Fat Stacks");
            std::fs::create_dir_all(&stacks).unwrap();
            [2, 10].iter().for_each(|f| {
                let file = stacks.join(format!("fat_stacks_{f}x.field.json"));
                std::fs::write(
                    file,
                    ARROWS.replace("Arrow Stacks", &format!("Fat Stacks {f}X")),
                )
                .unwrap();
            });
            let app = h.state_mut();
            app.dispatch(Action::Mod(ModAction::ImportPaths(vec![stacks])));
            std::thread::sleep(std::time::Duration::from_millis(500));
            app.poll();
            app.dispatch(Action::Mod(ModAction::Select("Fat Stacks".into())));
            shoot(&mut h, &format!("{tag}-json-options"));
            h.state_mut().dispatch(Action::Notice(
                crate::ui::notify::NoticeAction::ToggleCenter,
            ));
            shoot(&mut h, &format!("{tag}-notifications"));
            h.state_mut().dispatch(Action::Notice(
                crate::ui::notify::NoticeAction::ToggleCenter,
            ));
            h.state_mut()
                .dispatch(Action::Mod(ModAction::Select("Arrow Stacks".into())));
            // Tall enough to show the editor under the details.
            h.set_size(eframe::egui::vec2(1200.0, 1100.0));
            shoot(&mut h, &format!("{tag}-json-editor"));
            h.set_size(eframe::egui::vec2(1200.0, 760.0));
            let app = h.state_mut();
            let saves = crate::save_backup::save_dir(app.info.prefix_dir.as_deref()).unwrap();
            let slot = saves.join("7/slot1");
            std::fs::create_dir_all(&slot).unwrap();
            let save = crate::savefile::edit::tests::pouch_save();
            std::fs::write(slot.join("save.save"), save).unwrap();
            app.dispatch(Action::Backup(crate::ui::BackupAction::InspectSave(slot)));
            let at = app.save_view.as_ref().unwrap().tree[0].children[0]
                .edit
                .as_ref()
                .unwrap()
                .at;
            let view = app.save_view.as_mut().unwrap();
            view.edits.insert(at, "350".into());
            view.query = "money".into();
            shoot(&mut h, &format!("{tag}-save-editor"));
            h.state_mut()
                .dispatch(Action::Backup(crate::ui::BackupAction::CloseSaveView));
            h.state_mut()
                .dispatch(Action::Ask(crate::ui::Confirm::CleanSlate));
            shoot(&mut h, &format!("{tag}-confirm"));
        });
}
