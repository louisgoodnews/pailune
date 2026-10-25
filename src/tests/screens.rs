//! Renders key screens to PNG for visual review (ignored by default; needs a
//! GPU adapter). `PYLUNE_SHOTS=<dir> cargo test screens -- --ignored`.

use super::{Fixture, fixture};
use crate::config::paths::DataPaths;
use crate::config::prefs::Theme;
use crate::ui::guide::{GuideAction, GuidePage};
use crate::ui::{Action, ModAction, PyluneApp, SettingsPage, Tab};
use egui_kittest::Harness;
use std::path::PathBuf;

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
            pages.into_iter().for_each(|(name, tab)| {
                let app = h.state_mut();
                app.dispatch(Action::SwitchTab(tab));
                app.dispatch(Action::Mod(ModAction::Select("Cool Mod".into())));
                shoot(&mut h, &format!("{tag}-{name}"));
            });
            h.state_mut()
                .dispatch(Action::Ask(crate::ui::Confirm::CleanSlate));
            shoot(&mut h, &format!("{tag}-confirm"));
        });
}
