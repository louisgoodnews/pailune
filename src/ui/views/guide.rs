//! Guide window: the getting-started checklist and the "How do I…" topics.
//! A normal window rather than a modal, so users can act while reading
//! (learning by doing, R-16).

use super::clicked;
use crate::ui::guide::steps::{CONCEPTS, Facts, STEPS, Step};
use crate::ui::guide::{GuideAction, GuidePage, topics};
use crate::ui::theme::{accent, muted, success};
use crate::ui::{Action, DeployAction, ModAction, PyluneApp, Tab};
use eframe::egui::{self, RichText};

/// Buttons that perform a step for the user.
fn step_actions(ui: &mut egui::Ui, step: Step) -> Vec<Action> {
    let buttons: &[(&str, Action)] = match step {
        Step::FindGame => &[(
            "📁 Choose game folder…",
            Action::Guide(GuideAction::ChooseGame),
        )],
        Step::AddMod => &[
            (
                "📦 Import archives / files…",
                Action::Mod(ModAction::ImportFiles),
            ),
            ("🌐 Nexus tab", Action::SwitchTab(Tab::Nexus)),
        ],
        Step::EnableMod => &[("Show the mod list", Action::SwitchTab(Tab::Mods))],
        Step::Apply => &[("✔ Apply now", Action::Deploy(DeployAction::Apply))],
        Step::Play => &[("▶ Play", Action::Deploy(DeployAction::Launch))],
    };
    ui.horizontal_wrapped(|ui| {
        clicked(
            buttons
                .iter()
                .map(|(label, action)| (ui.button(*label).clicked(), action.clone())),
        )
    })
    .inner
}

/// One step: its state mark, title, and for the current step the explanation and buttons.
fn step_row(
    ui: &mut egui::Ui,
    facts: &Facts,
    (step, title, help): (Step, &str, &str),
) -> Vec<Action> {
    let current = facts.current() == Some(step);
    let (mark, color) = match (facts.done(step), current) {
        (true, _) => ("✔", success()),
        (false, true) => ("➡", accent()),
        (false, false) => ("○", muted()),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(mark).strong().color(color));
        let text = RichText::new(title).strong();
        ui.label(match current {
            true => text.color(accent()),
            false => text,
        });
    });
    match current {
        true => {
            ui.label(help);
            step_actions(ui, step)
        }
        false => Vec::new(),
    }
}

/// Concepts, progress and the steps; a finish state when all are done.
fn getting_started(ui: &mut egui::Ui, facts: &Facts) -> Vec<Action> {
    ui.label(RichText::new("Three things to know").strong());
    CONCEPTS
        .iter()
        .for_each(|c| drop(ui.label(format!("• {c}"))));
    ui.add_space(8.0);
    let done = facts.completed();
    ui.add(
        egui::ProgressBar::new(done as f32 / STEPS.len() as f32)
            .text(format!("{done} of {} done", STEPS.len())),
    );
    ui.add_space(4.0);
    let mut actions: Vec<Action> = STEPS.iter().flat_map(|s| step_row(ui, facts, *s)).collect();
    if facts.current().is_none() {
        ui.add_space(8.0);
        ui.label(RichText::new("🎉 You're all set").strong().color(success()));
        ui.label("Next: save your setup as a profile, check for updates, or browse the topics.");
        actions.extend(clicked([
            (
                ui.button("Profiles").clicked(),
                Action::SwitchTab(Tab::Profiles),
            ),
            (
                ui.button("How do I…").clicked(),
                Action::Guide(GuideAction::Open(GuidePage::Topics)),
            ),
        ]));
    }
    actions
}

/// Search box and the matching topics (a single match opens by itself).
fn topic_list(ui: &mut egui::Ui, query: &mut String) {
    ui.add(
        egui::TextEdit::singleline(query)
            .hint_text("🔍 Search, e.g. undo, collection, load order")
            .desired_width(f32::INFINITY),
    );
    let found = topics::search(query);
    if found.is_empty() {
        ui.label(RichText::new("No topic matches. Try fewer or other words.").color(muted()));
    }
    let single = found.len() == 1;
    egui::ScrollArea::vertical()
        .max_height(420.0)
        .show(ui, |ui| {
            found.iter().for_each(|topic| {
                egui::CollapsingHeader::new(topic.title)
                    .id_salt(topic.title)
                    .open(single.then_some(true))
                    .show(ui, |ui| topic.body.iter().for_each(|p| drop(ui.label(*p))));
            });
        });
}

/// Renders the guide window (if open) and returns triggered actions.
pub fn show(ctx: &egui::Context, app: &mut PyluneApp) -> Vec<Action> {
    if !app.guide.open {
        return Vec::new();
    }
    let facts = app.guide_facts();
    let mut show_at_start = app.config.behavior.show_guide;
    let mut open = true;
    let inner = egui::Window::new("❓ Pylune guide")
        .open(&mut open)
        .default_width(420.0)
        // Beside the mod list, below the header, so the steps' targets stay visible.
        .pivot(egui::Align2::RIGHT_TOP)
        .default_pos(ctx.content_rect().right_top() + egui::vec2(-16.0, 96.0))
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut app.guide.page,
                    GuidePage::GettingStarted,
                    "Getting started",
                );
                ui.selectable_value(&mut app.guide.page, GuidePage::Topics, "How do I…");
            });
            ui.separator();
            let actions = match app.guide.page {
                GuidePage::GettingStarted => getting_started(ui, &facts),
                GuidePage::Topics => {
                    topic_list(ui, &mut app.guide.query);
                    Vec::new()
                }
            };
            ui.separator();
            let toggled = ui
                .checkbox(&mut show_at_start, "Show this guide when Pylune starts")
                .changed();
            ui.label(
                RichText::new("Reopen it any time with ❓ Guide or F1.")
                    .small()
                    .color(muted()),
            );
            (actions, toggled)
        })
        .and_then(|r| r.inner);
    let (mut actions, toggled) = inner.unwrap_or_default();
    actions.extend(clicked([
        (
            toggled,
            Action::Guide(GuideAction::ShowAtStart(show_at_start)),
        ),
        (!open, Action::Guide(GuideAction::Close)),
    ]));
    actions
}
